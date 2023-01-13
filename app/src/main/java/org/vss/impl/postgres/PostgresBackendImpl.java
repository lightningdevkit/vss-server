package org.vss.impl.postgres;

import com.google.inject.Inject;
import com.google.protobuf.ByteString;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
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

import static org.jooq.impl.DSL.val;
import static org.vss.postgres.tables.VssDb.VSS_DB;

@Singleton
public class PostgresBackendImpl implements KVStore {

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

    if (vssDbRecord != null) {
      return GetObjectResponse.newBuilder()
          .setValue(KeyValue.newBuilder()
              .setKey(vssDbRecord.getKey())
              .setValue(ByteString.copyFrom(vssDbRecord.getValue()))
              .setVersion(vssDbRecord.getVersion())
              .build())
          .build();
    } else {
      return GetObjectResponse.newBuilder()
          .setValue(KeyValue.newBuilder()
              .setKey(request.getKey()).build())
          .build();
    }
  }

  @Override
  public PutObjectResponse put(PutObjectRequest request) {

    String storeId = request.getStoreId();

    List<VssDbRecord> vssRecords = new ArrayList<>(request.getTransactionItemsList().stream()
        .map(kv -> buildVssRecord(storeId, kv)).toList());

    VssDbRecord globalVersionRecord = buildVssRecord(storeId,
        KeyValue.newBuilder()
            .setKey(GLOBAL_VERSION_KEY)
            .setVersion(request.getGlobalVersion())
            .build());

    vssRecords.add(globalVersionRecord);

    context.transaction((ctx) -> {
      DSLContext dsl = ctx.dsl();
      List<Query> batchQueries = vssRecords.stream()
          .map(vssRecord -> buildPutObjectQuery(dsl, vssRecord)).toList();

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

  private Query buildPutObjectQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return vssRecord.getVersion() == 0 ? buildInsertRecordQuery(dsl, vssRecord)
        : buildUpdateRecordQuery(dsl, vssRecord);
  }

  private Insert<VssDbRecord> buildInsertRecordQuery(DSLContext dsl, VssDbRecord vssRecord) {
    return dsl.insertInto(VSS_DB)
        .select(dsl.select(val(vssRecord.getStoreId()), val(vssRecord.getKey()),
                val(vssRecord.getValue()), val(1))
            .whereNotExists(dsl.selectOne().from(VSS_DB)
                .where(VSS_DB.STORE_ID.eq(vssRecord.getStoreId())
                    .and(VSS_DB.KEY.eq(vssRecord.getKey())))));
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

  @Override public ListKeyVersionsResponse listKeyVersions(ListKeyVersionsRequest request) {
    throw new UnsupportedOperationException("Operation not implemented");
  }
}

