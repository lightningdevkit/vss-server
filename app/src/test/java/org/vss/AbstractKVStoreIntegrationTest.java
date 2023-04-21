package org.vss;

import com.google.protobuf.ByteString;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Objects;
import org.junit.jupiter.api.Test;
import org.vss.exception.ConflictException;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

public abstract class AbstractKVStoreIntegrationTest {

  private final String STORE_ID = "storeId";

  protected KVStore kvStore;

  @Test
  void putShouldSucceedWhenSingleObjectPutOperation() {
    assertDoesNotThrow(() -> putObjects(0L, List.of(kv("k1", "k1v1", 0))));
    assertDoesNotThrow(() -> putObjects(1L, List.of(kv("k1", "k1v2", 1))));

    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(2L));
    assertThat(response.getValue().toStringUtf8(), is("k1v2"));
  }

  @Test
  void putShouldSucceedWhenMultiObjectPutOperation() {
    final List<KeyValue> keyValues = List.of(kv("k1", "k1v1", 0),
        kv("k2", "k2v1", 0));

    assertDoesNotThrow(() -> putObjects(0L, keyValues));

    List<KeyValue> second_request = List.of(kv("k1", "k1v2", 1),
        kv("k2", "k2v2", 1));
    putObjects(1L, second_request);

    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(2L));
    assertThat(response.getValue().toStringUtf8(), is("k1v2"));

    response = getObject("k2");
    assertThat(response.getKey(), is("k2"));
    assertThat(response.getVersion(), is(2L));
    assertThat(response.getValue().toStringUtf8(), is("k2v2"));
  }

  @Test
  void putShouldFailWhenKeyVersionMismatched() {
    putObjects(0L, List.of(kv("k1", "k1v1", 0)));

    // global_version correctly changed but key-version conflict.
    assertThrows(ConflictException.class, () -> putObjects(1L, List.of(kv("k1", "k1v2", 0))));

    //Verify that values didn't change
    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k1v1"));
  }

  @Test
  void putMultiObjectShouldFailWhenSingleKeyVersionMismatched() {
    final List<KeyValue> keyValues = List.of(kv("k1", "k1v1", 0),
        kv("k2", "k2v1", 0));

    assertDoesNotThrow(() -> putObjects(null, keyValues));

    List<KeyValue> second_request = List.of(kv("k1", "k1v2", 0),
        kv("k2", "k2v2", 1));

    assertThrows(ConflictException.class, () -> putObjects(null, second_request));

    //Verify that values didn't change
    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k1v1"));

    response = getObject("k2");
    assertThat(response.getKey(), is("k2"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k2v1"));
  }

  @Test
  void putShouldFailWhenGlobalVersionMismatched() {
    putObjects(0L, List.of(kv("k1", "k1v1", 0)));

    // key-version correctly changed but global_version conflict.
    assertThrows(ConflictException.class, () -> putObjects(0L, List.of(kv("k1", "k1v2", 1))));

    //Verify that values didn't change
    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k1v1"));
  }

  @Test
  void putShouldSucceedWhenNoGlobalVersionIsGiven() {
    assertDoesNotThrow(() -> putObjects(null, List.of(kv("k1", "k1v1", 0))));
    assertDoesNotThrow(() -> putObjects(null, List.of(kv("k1", "k1v2", 1))));

    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(2L));
    assertThat(response.getValue().toStringUtf8(), is("k1v2"));
  }

  @Test
  void getShouldReturnEmptyResponseWhenKeyDoesNotExist() {
    KeyValue response = getObject("non_existent_key");

    assertThat(response.getKey(), is("non_existent_key"));
    assertTrue(response.getValue().isEmpty());
    assertThat(response.getVersion(), is(0L));
  }

  @Test
  void getShouldReturnCorrectValueWhenKeyExists() {

    putObjects(0L, List.of(kv("k1", "k1v1", 0)));

    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k1v1"));

    List<KeyValue> keyValues = List.of(kv("k1", "k1v2", 1),
        kv("k2", "k2v1", 0));
    putObjects(1L, keyValues);

    response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(2L));
    assertThat(response.getValue().toStringUtf8(), is("k1v2"));

    response = getObject("k2");
    assertThat(response.getKey(), is("k2"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k2v1"));

    keyValues = List.of(kv("k2", "k2v2", 1),
        kv("k3", "k3v1", 0));
    putObjects(2L, keyValues);

    response = getObject("k2");
    assertThat(response.getKey(), is("k2"));
    assertThat(response.getVersion(), is(2L));
    assertThat(response.getValue().toStringUtf8(), is("k2v2"));

    response = getObject("k3");
    assertThat(response.getKey(), is("k3"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k3v1"));
  }

  private KeyValue getObject(String key) {
    GetObjectRequest getRequest = GetObjectRequest.newBuilder()
        .setStoreId(STORE_ID)
        .setKey(key)
        .build();
    return this.kvStore.get(getRequest).getValue();
  }

  private void putObjects(Long globalVersion, List<KeyValue> keyValues) {
    PutObjectRequest.Builder putObjectRequestBuilder = PutObjectRequest.newBuilder()
        .setStoreId(STORE_ID)
        .addAllTransactionItems(keyValues);

    if (Objects.nonNull(globalVersion)) {
      putObjectRequestBuilder.setGlobalVersion(globalVersion);
    }

    this.kvStore.put(putObjectRequestBuilder.build());
  }

  private KeyValue kv(String key, String value, int version) {
    return KeyValue.newBuilder().setKey(key).setVersion(version).setValue(
        ByteString.copyFrom(value.getBytes(
            StandardCharsets.UTF_8))).build();
  }
}
