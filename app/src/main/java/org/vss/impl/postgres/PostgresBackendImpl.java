package org.vss.impl.postgres;

import com.google.inject.Inject;
import com.google.protobuf.ByteString;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import javax.inject.Singleton;
import org.jooq.DSLContext;
import org.jooq.Insert;
import org.jooq.Query;
import org.jooq.Update;
import org.vss.GetObjectRequest;
import org.vss.GetObjectResponse;
import org.vss.KVStore;
import org.vss.KeyValue;
import org.vss.ListKeyVersionsRequest;
import org.vss.ListKeyVersionsResponse;
import org.vss.PutObjectRequest;
import org.vss.PutObjectResponse;
import org.vss.exception.ConflictException;
import org.vss.postgres.tables.records.VssDbRecord;

import static org.vss.postgres.tables.VssDb.VSS_DB;

@Singleton
public class PostgresBackendImpl implements KVStore {

  private static final int LIST_KEY_VERSIONS_MAX_PAGE_SIZE = 100;
  private final DSLContext context;

  @Inject
  public PostgresBackendImpl(DSLContext context) {
    this.context = context;
  }

  @Override
  public GetObjectResponse get(GetObjectRequest request) {

    VssDbRecord vssDbRecord = context.selectFrom(VSS_DB)
        .where(VSS_DB.STORE_ID.eq(request.getStoreId())
            .and(VSS_DB.KEY.eq(request.getKey())))
        .fetchOne();

    final KeyValue keyValue;

    if (vssDbRecord != null) {
      keyValue = KeyValue.newBuilder()
          .setKey(vssDbRecord.getKey())
          .setValue(ByteString.copyFrom(vssDbRecord.getValue()))
          .setVersion(vssDbRecord.getVersion())
          .build();
    } else {
      keyValue = KeyValue.newBuilder()
          .setKey(request.getKey()).build();
    }

    return GetObjectResponse.newBuilder()
        .setValue(keyValue)
        .build();
  }

  @Override
  public PutObjectResponse put(PutObjectRequest request) {

    String storeId = request.getStoreId();

    List<VssDbRecord> vssPutRecords = new ArrayList<>(request.getTransactionItemsList().stream()
        .map(kv -> buildVssRecord(storeId, kv)).toList());

    List<VssDbRecord> vssDeleteRecords = new ArrayList<>(request.getDeleteItemsList().stream()
        .map(kv -> buildVssRecord(storeId, kv)).toList());

    if (request.hasGlobalVersion()) {
      VssDbRecord globalVersionRecord = buildVssRecord(storeId,
          KeyValue.newBuilder()
              .setKey(GLOBAL_VERSION_KEY)
              .setVersion(request.getGlobalVersion())
              .setValue(ByteString.EMPTY)
              .build());

      vssPutRecords.add(globalVersionRecord);
    }

    context.transaction((ctx) -> {
      DSLContext dsl = ctx.dsl();
      List<Query> batchQueries = new ArrayList<>();

      batchQueries.addAll(vssPutRecords.stream()
          .map(vssRecord -> buildPutObjectQuery(dsl, vssRecord)).toList());
      batchQueries.addAll(vssDeleteRecords.stream()
          .map(vssRecord -> buildDeleteObjectQuery(dsl, vssRecord)).toList());

      int[] batchResult = dsl.batch(batchQueries).execute();

      for (int numOfRowsUpdated : batchResult) {
        if (numOfRowsUpdated == 0) {
          throw new ConflictException(
              "Transaction could not be completed due to a possible conflict");
        }
      }
    });

    return PutObjectResponse.newBuilder().build();
  }

  private Query buildDeleteObjectQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return dsl.deleteFrom(VSS_DB).where(VSS_DB.STORE_ID.eq(vssRecord.getStoreId())
        .and(VSS_DB.KEY.eq(vssRecord.getKey()))
        .and(VSS_DB.VERSION.eq(vssRecord.getVersion())));
  }

  private Query buildPutObjectQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return vssRecord.getVersion() == 0 ? buildInsertRecordQuery(dsl, vssRecord)
        : buildUpdateRecordQuery(dsl, vssRecord);
  }

  private Insert<VssDbRecord> buildInsertRecordQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return dsl.insertInto(VSS_DB)
        .values(vssRecord.getStoreId(), vssRecord.getKey(),
            vssRecord.getValue(), 1)
        .onDuplicateKeyIgnore();
  }

  private Update<VssDbRecord> buildUpdateRecordQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return dsl.update(VSS_DB)
        .set(Map.of(VSS_DB.VALUE, vssRecord.getValue(),
            VSS_DB.VERSION, vssRecord.getVersion() + 1))
        .where(VSS_DB.STORE_ID.eq(vssRecord.getStoreId())
            .and(VSS_DB.KEY.eq(vssRecord.getKey()))
            .and(VSS_DB.VERSION.eq(vssRecord.getVersion())));
  }

  private VssDbRecord buildVssRecord(String storeId, KeyValue kv) {
    return new VssDbRecord()
        .setStoreId(storeId)
        .setKey(kv.getKey())
        .setValue(kv.getValue().toByteArray())
        .setVersion(kv.getVersion());
  }

  @Override
  public ListKeyVersionsResponse listKeyVersions(ListKeyVersionsRequest request) {
    String storeId = request.getStoreId();
    String keyPrefix = request.getKeyPrefix();
    String pageToken = request.getPageToken();
    int pageSize = request.hasPageSize() ? request.getPageSize() : Integer.MAX_VALUE;

    // Only fetch global_version for first page.
    // Fetch global_version before fetching any key_versions to ensure that,
    // all current key_versions were stored at global_version or later.
    Long globalVersion = null;
    if (!request.hasPageToken()) {
      GetObjectRequest getGlobalVersionRequest = GetObjectRequest.newBuilder()
          .setStoreId(storeId)
          .setKey(GLOBAL_VERSION_KEY)
          .build();
      globalVersion = get(getGlobalVersionRequest).getValue().getVersion();
    }

    List<VssDbRecord> vssDbRecords = context.select(VSS_DB.KEY, VSS_DB.VERSION).from(VSS_DB)
        .where(VSS_DB.STORE_ID.eq(storeId)
            .and(VSS_DB.KEY.startsWith(keyPrefix)))
        .orderBy(VSS_DB.KEY)
        .seek(pageToken)
        .limit(Math.min(pageSize, LIST_KEY_VERSIONS_MAX_PAGE_SIZE))
        .stream()
        .map(record -> record.into(VssDbRecord.class))
        .toList();

    List<KeyValue> keyVersions = vssDbRecords.stream()
        .filter(kv -> !GLOBAL_VERSION_KEY.equals(kv.getKey()))
        .map(kv -> KeyValue.newBuilder()
            .setKey(kv.getKey())
            .setVersion(kv.getVersion())
            .build())
        .toList();

    String nextPageToken = "";
    if (!keyVersions.isEmpty()) {
      nextPageToken = keyVersions.get(keyVersions.size() - 1).getKey();
    }

    ListKeyVersionsResponse.Builder responseBuilder = ListKeyVersionsResponse.newBuilder()
        .addAllKeyVersions(keyVersions)
        .setNextPageToken(nextPageToken);

    if (Objects.nonNull(globalVersion)) {
      responseBuilder.setGlobalVersion(globalVersion);
    }

    return responseBuilder.build();
  }
}
