package org.vss.api;

import jakarta.inject.Inject;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.Context;
import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import lombok.extern.slf4j.Slf4j;
import org.vss.DeleteObjectRequest;
import org.vss.DeleteObjectResponse;
import org.vss.KVStore;
import org.vss.auth.AuthResponse;
import org.vss.auth.Authorizer;

@Path(VssApiEndpoint.DELETE_OBJECT)
@Slf4j
public class DeleteObjectApi extends AbstractVssApi {
  @Inject
  public DeleteObjectApi(KVStore kvstore, Authorizer authorizer) {
    super(kvstore, authorizer);
  }

  @POST
  @Produces(MediaType.APPLICATION_OCTET_STREAM)
  public Response execute(byte[] payload, @Context HttpHeaders headers) {
    try {
      AuthResponse authResponse = authorizer.verify(headers);
      DeleteObjectRequest request = DeleteObjectRequest.parseFrom(payload);
      DeleteObjectResponse response = kvStore.delete(authResponse.getUserToken(), request);
      return toResponse(response);
    } catch (Exception e) {
      log.error("Exception in DeleteObjectApi: ", e);
      return toErrorResponse(e);
    }
  }
}
