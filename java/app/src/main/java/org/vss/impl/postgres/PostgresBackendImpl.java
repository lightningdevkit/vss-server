package org.vss.impl.postgres;

import com.google.inject.Inject;
import com.google.protobuf.ByteString;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.time.temporal.ChronoUnit;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import javax.inject.Singleton;
import org.jooq.DSLContext;
import org.jooq.DeleteConditionStep;
import org.jooq.Insert;
import org.jooq.Query;
import org.jooq.Update;
import org.vss.DeleteObjectRequest;
import org.vss.DeleteObjectResponse;
import org.vss.GetObjectRequest;
import org.vss.GetObjectResponse;
import org.vss.KVStore;
import org.vss.KeyValue;
import org.vss.ListKeyVersionsRequest;
import org.vss.ListKeyVersionsResponse;
import org.vss.PutObjectRequest;
import org.vss.PutObjectResponse;
import org.vss.exception.ConflictException;
import org.vss.exception.NoSuchKeyException;
import org.vss.postgres.tables.records.VssDbRecord;

import static org.vss.postgres.tables.VssDb.VSS_DB;

@Singleton
public class PostgresBackendImpl implements KVStore {

  private static final int LIST_KEY_VERSIONS_MAX_PAGE_SIZE = 100;
  private static final KeyValue DEFAULT_GLOBAL_VERSION_KV = KeyValue.newBuilder()
      .setKey(GLOBAL_VERSION_KEY)
      .setValue(ByteString.EMPTY)
      .setVersion(0L)
      .build();
  private final DSLContext context;

  @Inject
  public PostgresBackendImpl(DSLContext context) {
    this.context = context;
  }

  @Override
  public GetObjectResponse get(String userToken, GetObjectRequest request) {

    VssDbRecord vssDbRecord = context.selectFrom(VSS_DB)
        .where(VSS_DB.USER_TOKEN.eq(userToken)
            .and(VSS_DB.STORE_ID.eq(request.getStoreId())
                .and(VSS_DB.KEY.eq(request.getKey()))))
        .fetchOne();

    final KeyValue keyValue;
    if (vssDbRecord == null) {
      if (GLOBAL_VERSION_KEY.equals(request.getKey())) {
        keyValue = DEFAULT_GLOBAL_VERSION_KV;
      } else {
        throw new NoSuchKeyException(
            "Specified key: " + request.getKey() + " in request does not exist.");
      }
    } else {
      keyValue = KeyValue.newBuilder()
          .setKey(vssDbRecord.getKey())
          .setValue(ByteString.copyFrom(vssDbRecord.getValue()))
          .setVersion(vssDbRecord.getVersion())
          .build();
    }

    return GetObjectResponse.newBuilder()
        .setValue(keyValue)
        .build();
  }

  @Override
  public PutObjectResponse put(String userToken, PutObjectRequest request) {

    String storeId = request.getStoreId();

    List<VssDbRecord> vssPutRecords = new ArrayList<>(request.getTransactionItemsList().stream()
        .map(kv -> buildVssRecord(userToken, storeId, kv)).toList());

    List<VssDbRecord> vssDeleteRecords = new ArrayList<>(request.getDeleteItemsList().stream()
        .map(kv -> buildVssRecord(userToken, storeId, kv)).toList());

    if (request.hasGlobalVersion()) {
      VssDbRecord globalVersionRecord = buildVssRecord(userToken, storeId,
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
    if (vssRecord.getVersion() == -1) {
      return buildNonConditionalDeleteQuery(dsl, vssRecord);
    } else {
      return buildConditionalDeleteQuery(dsl, vssRecord);
    }
  }

  private static DeleteConditionStep<VssDbRecord> buildNonConditionalDeleteQuery(DSLContext dsl,
      VssDbRecord vssRecord) {
    return dsl.deleteFrom(VSS_DB).where(VSS_DB.USER_TOKEN.eq(vssRecord.getUserToken())
        .and(VSS_DB.STORE_ID.eq(vssRecord.getStoreId())
        .and(VSS_DB.KEY.eq(vssRecord.getKey()))));
  }

  private static DeleteConditionStep<VssDbRecord> buildConditionalDeleteQuery(DSLContext dsl,
      VssDbRecord vssRecord) {
    return dsl.deleteFrom(VSS_DB).where(VSS_DB.USER_TOKEN.eq(vssRecord.getUserToken())
        .and(VSS_DB.STORE_ID.eq(vssRecord.getStoreId())
        .and(VSS_DB.KEY.eq(vssRecord.getKey()))
        .and(VSS_DB.VERSION.eq(vssRecord.getVersion()))));
  }

  private Query buildPutObjectQuery(DSLContext dsl, VssDbRecord vssRecord) {
    if (vssRecord.getVersion() == -1) {
      return buildNonConditionalUpsertRecordQuery(dsl, vssRecord);
    } else if (vssRecord.getVersion() == 0) {
      return buildConditionalInsertRecordQuery(dsl, vssRecord);
    } else {
      return buildConditionalUpdateRecordQuery(dsl, vssRecord);
    }
  }

  private Query buildNonConditionalUpsertRecordQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return dsl.insertInto(VSS_DB)
        .values(vssRecord.getUserToken(), vssRecord.getStoreId(), vssRecord.getKey(),
            vssRecord.getValue(), 1, vssRecord.getCreatedAt(), vssRecord.getLastUpdatedAt())
        .onConflict(VSS_DB.USER_TOKEN, VSS_DB.STORE_ID, VSS_DB.KEY)
        .doUpdate()
        .set(VSS_DB.VALUE, vssRecord.getValue())
        .set(VSS_DB.VERSION, 1L)
        .set(VSS_DB.LAST_UPDATED_AT, vssRecord.getLastUpdatedAt());
  }

  private Insert<VssDbRecord> buildConditionalInsertRecordQuery(DSLContext dsl,
      VssDbRecord vssRecord) {
    return dsl.insertInto(VSS_DB)
        .values(vssRecord.getUserToken(), vssRecord.getStoreId(), vssRecord.getKey(),
            vssRecord.getValue(), 1, vssRecord.getCreatedAt(), vssRecord.getLastUpdatedAt())
        .onDuplicateKeyIgnore();
  }

  private Update<VssDbRecord> buildConditionalUpdateRecordQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return dsl.update(VSS_DB)
        .set(Map.of(VSS_DB.VALUE, vssRecord.getValue(),
            VSS_DB.VERSION, vssRecord.getVersion() + 1,
            VSS_DB.LAST_UPDATED_AT, vssRecord.getLastUpdatedAt()))
        .where(VSS_DB.USER_TOKEN.eq(vssRecord.getUserToken())
            .and(VSS_DB.STORE_ID.eq(vssRecord.getStoreId())
            .and(VSS_DB.KEY.eq(vssRecord.getKey()))
            .and(VSS_DB.VERSION.eq(vssRecord.getVersion()))));
  }

  private VssDbRecord buildVssRecord(String userToken, String storeId, KeyValue kv) {
    OffsetDateTime today = OffsetDateTime.now(ZoneOffset.UTC).truncatedTo(ChronoUnit.DAYS);
    return new VssDbRecord()
        .setUserToken(userToken)
        .setStoreId(storeId)
        .setKey(kv.getKey())
        .setValue(kv.getValue().toByteArray())
        .setVersion(kv.getVersion())
        .setCreatedAt(today)
        .setLastUpdatedAt(today);
  }

  @Override
  public DeleteObjectResponse delete(String userToken, DeleteObjectRequest request) {
    String storeId = request.getStoreId();
    VssDbRecord vssDbRecord = buildVssRecord(userToken, storeId, request.getKeyValue());

    context.transaction((ctx) -> {
      DSLContext dsl = ctx.dsl();
      Query deleteObjectQuery = buildDeleteObjectQuery(dsl, vssDbRecord);
      dsl.execute(deleteObjectQuery);
    });

    return DeleteObjectResponse.newBuilder().build();
  }

  @Override
  public ListKeyVersionsResponse listKeyVersions(String userToken, ListKeyVersionsRequest request) {
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
      globalVersion = get(userToken, getGlobalVersionRequest).getValue().getVersion();
    }

    List<VssDbRecord> vssDbRecords = context.select(VSS_DB.KEY, VSS_DB.VERSION).from(VSS_DB)
        .where(VSS_DB.USER_TOKEN.eq(userToken)
            .and(VSS_DB.STORE_ID.eq(storeId)
            .and(VSS_DB.KEY.startsWith(keyPrefix))))
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
