package org.vss.api;

import com.google.protobuf.ByteString;
import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.Response;
import java.nio.charset.StandardCharsets;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.vss.DeleteObjectRequest;
import org.vss.DeleteObjectResponse;
import org.vss.ErrorCode;
import org.vss.ErrorResponse;
import org.vss.KVStore;
import org.vss.KeyValue;
import org.vss.auth.AuthResponse;
import org.vss.auth.Authorizer;
import org.vss.exception.ConflictException;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

public class DeleteObjectApiTest {
  private DeleteObjectApi deleteObjectApi;
  private KVStore mockKVStore;
  private Authorizer mockAuthorizer;
  private HttpHeaders mockHeaders;

  private static String TEST_USER_TOKEN = "userToken";
  private static String TEST_STORE_ID = "storeId";
  private static String TEST_KEY = "key";
  private static KeyValue TEST_KV = KeyValue.newBuilder().setKey(TEST_KEY).setValue(
      ByteString.copyFrom("test_value", StandardCharsets.UTF_8)).build();

  @BeforeEach
  void setUp() {
    mockKVStore = mock(KVStore.class);
    mockAuthorizer = mock(Authorizer.class);
    deleteObjectApi = new DeleteObjectApi(mockKVStore, mockAuthorizer);
    mockHeaders = mock(HttpHeaders.class);
    when(mockAuthorizer.verify(any())).thenReturn(new AuthResponse(TEST_USER_TOKEN));
  }

  @Test
  void execute_ValidPayload_ReturnsResponse() {
    DeleteObjectRequest expectedRequest =
        DeleteObjectRequest.newBuilder().setStoreId(TEST_STORE_ID).setKeyValue(
            KeyValue.newBuilder().setKey(TEST_KEY).setVersion(0)
        ).build();
    byte[] payload = expectedRequest.toByteArray();
    DeleteObjectResponse mockResponse = DeleteObjectResponse.newBuilder().build();
    when(mockKVStore.delete(TEST_USER_TOKEN, expectedRequest)).thenReturn(mockResponse);

    Response actualResponse = deleteObjectApi.execute(payload, mockHeaders);

    assertThat(actualResponse.getStatus(), is(Response.Status.OK.getStatusCode()));
    assertThat(actualResponse.getEntity(), is(mockResponse.toByteArray()));
    verify(mockKVStore).delete(TEST_USER_TOKEN, expectedRequest);
  }

  @ParameterizedTest
  @MethodSource("provideErrorTestCases")
  void execute_InvalidPayload_ReturnsErrorResponse(Exception exception,
      ErrorCode errorCode, int statusCode) {
    DeleteObjectRequest expectedRequest =
        DeleteObjectRequest.newBuilder().setStoreId(TEST_STORE_ID).setKeyValue(
            KeyValue.newBuilder().setKey(TEST_KEY).setVersion(0)
        ).build();
    byte[] payload = expectedRequest.toByteArray();
    when(mockKVStore.delete(any(), any())).thenThrow(exception);

    Response response = deleteObjectApi.execute(payload, mockHeaders);

    ErrorResponse expectedErrorResponse = ErrorResponse.newBuilder()
        .setErrorCode(errorCode)
        .setMessage("")
        .build();
    assertThat(response.getEntity(), is(expectedErrorResponse.toByteArray()));
    assertThat(response.getStatus(), is(statusCode));
    verify(mockKVStore).delete(TEST_USER_TOKEN, expectedRequest);
  }

  private static Stream<Arguments> provideErrorTestCases() {
    return Stream.of(
        Arguments.of(new ConflictException(""), ErrorCode.CONFLICT_EXCEPTION, 409),
        Arguments.of(new IllegalArgumentException(""), ErrorCode.INVALID_REQUEST_EXCEPTION, 400),
        Arguments.of(new RuntimeException(""), ErrorCode.INTERNAL_SERVER_EXCEPTION, 500)
    );
  }
}
