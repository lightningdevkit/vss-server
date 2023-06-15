package org.vss.api;

import com.google.protobuf.GeneratedMessageV3;
import com.google.protobuf.InvalidProtocolBufferException;
import jakarta.inject.Inject;
import jakarta.ws.rs.core.Response;
import org.vss.ErrorCode;
import org.vss.ErrorResponse;
import org.vss.KVStore;
import org.vss.exception.ConflictException;

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
    if (e instanceof ConflictException) {
      errorCode = ErrorCode.CONFLICT_EXCEPTION;
    } else if (e instanceof IllegalArgumentException
        || e instanceof InvalidProtocolBufferException) {
      errorCode = ErrorCode.INVALID_REQUEST_EXCEPTION;
    } else {
      errorCode = ErrorCode.INTERNAL_SERVER_EXCEPTION;
    }

    ErrorResponse errorResponse = ErrorResponse.newBuilder()
        .setErrorCode(errorCode)
        .setMessage(e.getMessage())
        .build();

    return Response.status(errorCode.getNumber())
        .entity(errorResponse.toByteArray())
        .build();
  }
}
