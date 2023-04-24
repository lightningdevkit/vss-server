package org.vss;

import com.google.protobuf.ByteString;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Set;
import java.util.stream.Collectors;
import javax.annotation.Nullable;
import org.junit.jupiter.api.Test;
import org.testcontainers.shaded.org.apache.commons.lang3.StringUtils;
import org.vss.exception.ConflictException;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;
import static org.hamcrest.Matchers.lessThan;
import static org.hamcrest.Matchers.lessThanOrEqualTo;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertFalse;
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

    assertThat(getObject(KVStore.GLOBAL_VERSION_KEY).getVersion(), is(2L));
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

    assertThat(getObject(KVStore.GLOBAL_VERSION_KEY).getVersion(), is(2L));
  }

  @Test
  void putShouldFailWhenKeyVersionMismatched() {
    putObjects(0L, List.of(kv("k1", "k1v1", 0)));

    // global_version correctly changed but key-version conflict.
    assertThrows(ConflictException.class, () -> putObjects(1L, List.of(kv("k1", "k1v2", 0))));

    // Verify that values didn't change
    KeyValue response = getObject("k1");
    assertThat(response.getKey(), is("k1"));
    assertThat(response.getVersion(), is(1L));
    assertThat(response.getValue().toStringUtf8(), is("k1v1"));

    assertThat(getObject(KVStore.GLOBAL_VERSION_KEY).getVersion(), is(1L));
  }

  @Test
  void putMultiObjectShouldFailWhenSingleKeyVersionMismatched() {
    final List<KeyValue> keyValues = List.of(kv("k1", "k1v1", 0),
        kv("k2", "k2v1", 0));

    assertDoesNotThrow(() -> putObjects(null, keyValues));

    List<KeyValue> second_request = List.of(kv("k1", "k1v2", 0),
        kv("k2", "k2v2", 1));

    assertThrows(ConflictException.class, () -> putObjects(null, second_request));

    // Verify that values didn't change
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

    assertThat(getObject(KVStore.GLOBAL_VERSION_KEY).getVersion(), is(0L));
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

  @Test
  void listShouldReturnPaginatedResponse() {

    int totalKvObjects = 1000;
    for (int i = 0; i < totalKvObjects; i++) {
      putObjects((long) i, List.of(kv("k" + i, "k1v1", 0)));
    }
    // Overwrite k1 once and k2 twice.
    putObjects(1000L, List.of(kv("k1", "k1v2", 1)));
    putObjects(1001L, List.of(kv("k2", "k2v2", 1)));
    putObjects(1002L, List.of(kv("k2", "k2v3", 2)));

    ListKeyVersionsResponse previousPage = null;
    List<KeyValue> allKeyVersions = new ArrayList<>();

    while (previousPage == null || !previousPage.getKeyVersionsList().isEmpty()) {
      ListKeyVersionsResponse currentPage;

      if (previousPage == null) {
        currentPage = list(null, null, null);

        // Ensure first page contains correct global version
        assertThat(currentPage.getGlobalVersion(), is(1003L));
      } else {
        String nextPageToken = previousPage.getNextPageToken();
        currentPage = list(nextPageToken, null, null);

        // Ensure pages after first page dont contain global version.
        assertThat(currentPage.hasGlobalVersion(), is(false));
      }

      allKeyVersions.addAll(currentPage.getKeyVersionsList());
      previousPage = currentPage;
    }

    // Ensure page results don't intersect/duplicate and return complete view.
    Set<String> uniqueKeys = allKeyVersions.stream().map(KeyValue::getKey).distinct()
        .collect(Collectors.toSet());
    assertThat(uniqueKeys.size(), is(totalKvObjects));

    // Ensure that we don't return "vss_global_version" as part of keys.
    assertFalse(uniqueKeys.contains(KVStore.GLOBAL_VERSION_KEY));

    // Ensure correct key version for k1
    KeyValue k1_response =
        allKeyVersions.stream().filter(kv -> "k1".equals(kv.getKey())).findFirst().get();
    assertThat(k1_response.getKey(), is("k1"));
    assertThat(k1_response.getVersion(), is(2L));
    assertThat(k1_response.getValue().toStringUtf8(), is(""));

    // Ensure correct key version for k2
    KeyValue k2_response =
        allKeyVersions.stream().filter(kv -> "k2".equals(kv.getKey())).findFirst().get();
    assertThat(k2_response.getKey(), is("k2"));
    assertThat(k2_response.getVersion(), is(3L));
    assertThat(k2_response.getValue().toStringUtf8(), is(""));
  }

  @Test
  void listShouldHonourPageSizeAndKeyPrefixIfProvided() {
    int totalKvObjects = 20;
    int pageSize = 5;
    for (int i = 0; i < totalKvObjects; i++) {
      putObjects((long) i, List.of(kv(i + "k", "k1v1", 0)));
    }

    ListKeyVersionsResponse previousPage = null;
    List<KeyValue> allKeyVersions = new ArrayList<>();
    String keyPrefix = "1";

    while (previousPage == null || !previousPage.getKeyVersionsList().isEmpty()) {
      ListKeyVersionsResponse currentPage;

      if (previousPage == null) {
        currentPage = list(null, pageSize, keyPrefix);
      } else {
        String nextPageToken = previousPage.getNextPageToken();
        currentPage = list(nextPageToken, pageSize, keyPrefix);
      }

      allKeyVersions.addAll(currentPage.getKeyVersionsList());

      // Each page.size() is less than or equal to pageSize in request.
      assertThat(currentPage.getKeyVersionsList().size(), lessThanOrEqualTo(pageSize));
      previousPage = currentPage;
    }

    Set<String> uniqueKeys =
        allKeyVersions.stream().map(KeyValue::getKey).collect(Collectors.toSet());

    // Returns keys only with provided keyPrefix
    assertThat(uniqueKeys.size(), is(11));
    assertThat(uniqueKeys,
        is(Set.of("1k", "10k", "11k", "12k", "13k", "14k", "15k", "16k", "17k", "18k", "19k")));
  }

  @Test
  void listShouldReturnZeroGlobalVersionWhenGlobalVersioningNotEnabled() {
    int totalKvObjects = 1000;
    for (int i = 0; i < totalKvObjects; i++) {
      putObjects(null, List.of(kv("k" + i, "k1v1", 0)));
    }

    ListKeyVersionsResponse previousPage = null;
    List<KeyValue> allKeyVersions = new ArrayList<>();

    while (previousPage == null || !previousPage.getKeyVersionsList().isEmpty()) {
      ListKeyVersionsResponse currentPage;

      if (previousPage == null) {
        currentPage = list(null, null, null);

        // Ensure first page returns global version as ZERO
        assertThat(currentPage.getGlobalVersion(), is(0L));
      } else {
        String nextPageToken = previousPage.getNextPageToken();
        currentPage = list(nextPageToken, null, null);

        // Ensure pages after first page do not contain global version.
        assertThat(currentPage.hasGlobalVersion(), is(false));
      }

      allKeyVersions.addAll(currentPage.getKeyVersionsList());
      previousPage = currentPage;
    }
    // Returns complete view.
    Set<String> uniqueKeys = allKeyVersions.stream().map(KeyValue::getKey).distinct()
        .collect(Collectors.toSet());
    assertThat(uniqueKeys.size(), is(totalKvObjects));

    // Ensure that we don't return "vss_global_version" as part of keys.
    assertFalse(uniqueKeys.contains(KVStore.GLOBAL_VERSION_KEY));
  }

  @Test
  void listShouldLimitMaxPageSize() {

    int totalKvObjects = 10000;

    // Each implementation is free to choose its own max_page_size but there should be a reasonable max
    // keeping scalability and performance in mind.
    // Revisit this test case if some implementation wants to support higher page size.
    int vssArbitraryPageSizeMax = 3000;

    for (int i = 0; i < totalKvObjects; i++) {
      putObjects((long) i, List.of(kv("k" + i, "k1v1", 0)));
    }

    ListKeyVersionsResponse previousPage = null;
    List<KeyValue> allKeyVersions = new ArrayList<>();

    while (previousPage == null || !previousPage.getKeyVersionsList().isEmpty()) {
      ListKeyVersionsResponse currentPage;

      if (previousPage == null) {
        currentPage = list(null, null, null);
      } else {
        String nextPageToken = previousPage.getNextPageToken();
        currentPage = list(nextPageToken, null, null);
      }

      allKeyVersions.addAll(currentPage.getKeyVersionsList());

      // Each page.size() is less than MAX_PAGE_SIZE
      assertThat(currentPage.getKeyVersionsList().size(), lessThan(vssArbitraryPageSizeMax));
      previousPage = currentPage;
    }

    assertThat(allKeyVersions.size(), is(totalKvObjects));
  }

  private KeyValue getObject(String key) {
    GetObjectRequest getRequest = GetObjectRequest.newBuilder()
        .setStoreId(STORE_ID)
        .setKey(key)
        .build();
    return this.kvStore.get(getRequest).getValue();
  }

  private void putObjects(@Nullable Long globalVersion, List<KeyValue> keyValues) {
    PutObjectRequest.Builder putObjectRequestBuilder = PutObjectRequest.newBuilder()
        .setStoreId(STORE_ID)
        .addAllTransactionItems(keyValues);

    if (Objects.nonNull(globalVersion)) {
      putObjectRequestBuilder.setGlobalVersion(globalVersion);
    }

    this.kvStore.put(putObjectRequestBuilder.build());
  }

  private ListKeyVersionsResponse list(@Nullable String nextPageToken, @Nullable Integer pageSize,
      @Nullable String keyPrefix) {
    ListKeyVersionsRequest.Builder listRequestBuilder = ListKeyVersionsRequest.newBuilder()
        .setStoreId(STORE_ID);

    if (StringUtils.isNotBlank(nextPageToken)) {
      listRequestBuilder.setPageToken(nextPageToken);
    }
    if (pageSize != null) {
      listRequestBuilder.setPageSize(pageSize);
    }
    if (StringUtils.isNotBlank(keyPrefix)) {
      listRequestBuilder.setKeyPrefix(keyPrefix);
    }

    return this.kvStore.listKeyVersions(listRequestBuilder.build());
  }

  private KeyValue kv(String key, String value, int version) {
    return KeyValue.newBuilder().setKey(key).setVersion(version).setValue(
        ByteString.copyFrom(value.getBytes(
            StandardCharsets.UTF_8))).build();
  }
}
