package org.vss.api;

import com.google.protobuf.GeneratedMessageV3;
import com.google.protobuf.InvalidProtocolBufferException;
import jakarta.inject.Inject;
import jakarta.ws.rs.core.Response;
import org.vss.ErrorCode;
import org.vss.ErrorResponse;
import org.vss.KVStore;
import org.vss.exception.AuthException;
import org.vss.exception.ConflictException;
import org.vss.exception.NoSuchKeyException;

public abstract class AbstractVssApi {
  final KVStore kvStore;

  @Inject
  public AbstractVssApi(KVStore kvStore) {
    this.kvStore = kvStore;
  }

  Response toResponse(GeneratedMessageV3 protoResponse) {

    return Response
        .status(Response.Status.OK)
        .entity(protoResponse.toByteArray())
        .build();
  }

  Response toErrorResponse(Exception e) {
    ErrorCode errorCode;
    int statusCode;
    if (e instanceof ConflictException) {
      errorCode = ErrorCode.CONFLICT_EXCEPTION;
      statusCode = 409;
    } else if (e instanceof IllegalArgumentException
        || e instanceof InvalidProtocolBufferException) {
      errorCode = ErrorCode.INVALID_REQUEST_EXCEPTION;
      statusCode = 400;
    } else if (e instanceof NoSuchKeyException) {
      errorCode = ErrorCode.NO_SUCH_KEY_EXCEPTION;
      statusCode = 404;
    } else if (e instanceof AuthException) {
      errorCode = ErrorCode.AUTH_EXCEPTION;
      statusCode = 401;
    } else {
      errorCode = ErrorCode.INTERNAL_SERVER_EXCEPTION;
      statusCode = 500;
    }

    ErrorResponse errorResponse = ErrorResponse.newBuilder()
        .setErrorCode(errorCode)
        .setMessage(e.getMessage())
        .build();

    return Response.status(statusCode)
        .entity(errorResponse.toByteArray())
        .build();
  }
}
