package org.vss.api;

import com.google.protobuf.ByteString;
import jakarta.ws.rs.core.Response;
import java.nio.charset.StandardCharsets;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.vss.ErrorCode;
import org.vss.ErrorResponse;
import org.vss.GetObjectRequest;
import org.vss.GetObjectResponse;
import org.vss.KVStore;
import org.vss.KeyValue;
import org.vss.exception.ConflictException;
import org.vss.exception.NoSuchKeyException;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

class GetObjectApiTest {
  private GetObjectApi getObjectApi;
  private KVStore mockKVStore;

  private static String TEST_STORE_ID = "storeId";
  private static String TEST_KEY = "key";
  private static KeyValue TEST_KV = KeyValue.newBuilder().setKey(TEST_KEY).setValue(
      ByteString.copyFrom("test_value", StandardCharsets.UTF_8)).build();

  @BeforeEach
  void setUp() {
    mockKVStore = mock(KVStore.class);
    getObjectApi = new GetObjectApi(mockKVStore);
  }

  @Test
  void execute_ValidPayload_ReturnsResponse() {
    GetObjectRequest expectedRequest =
        GetObjectRequest.newBuilder().setStoreId(TEST_STORE_ID).setKey(TEST_KEY).build();
    byte[] payload = expectedRequest.toByteArray();
    GetObjectResponse mockResponse = GetObjectResponse.newBuilder().setValue(TEST_KV).build();
    when(mockKVStore.get(expectedRequest)).thenReturn(mockResponse);

    Response actualResponse = getObjectApi.execute(payload);

    assertThat(actualResponse.getStatus(), is(Response.Status.OK.getStatusCode()));
    assertThat(actualResponse.getEntity(), is(mockResponse.toByteArray()));
    verify(mockKVStore).get(expectedRequest);
  }

  @ParameterizedTest
  @MethodSource("provideErrorTestCases")
  void execute_InvalidPayload_ReturnsErrorResponse(Exception exception,
      ErrorCode errorCode) {
    GetObjectRequest expectedRequest = GetObjectRequest.newBuilder()
        .setStoreId(TEST_STORE_ID)
        .setKey(TEST_KEY)
        .build();
    byte[] payload = expectedRequest.toByteArray();
    when(mockKVStore.get(any())).thenThrow(exception);

    Response response = getObjectApi.execute(payload);

    ErrorResponse expectedErrorResponse = ErrorResponse.newBuilder()
        .setErrorCode(errorCode)
        .setMessage("")
        .build();
    assertThat(response.getEntity(), is(expectedErrorResponse.toByteArray()));
    assertThat(response.getStatus(), is(expectedErrorResponse.getErrorCode().getNumber()));
    verify(mockKVStore).get(expectedRequest);
  }

  private static Stream<Arguments> provideErrorTestCases() {
    return Stream.of(
        Arguments.of(new ConflictException(""), ErrorCode.CONFLICT_EXCEPTION),
        Arguments.of(new IllegalArgumentException(""), ErrorCode.INVALID_REQUEST_EXCEPTION),
        Arguments.of(new NoSuchKeyException(""), ErrorCode.NO_SUCH_KEY_EXCEPTION),
        Arguments.of(new RuntimeException(""), ErrorCode.INTERNAL_SERVER_EXCEPTION)
    );
  }
}
