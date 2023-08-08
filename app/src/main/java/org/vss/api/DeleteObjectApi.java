package org.vss.api;

import jakarta.inject.Inject;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import lombok.extern.slf4j.Slf4j;
import org.vss.DeleteObjectRequest;
import org.vss.DeleteObjectResponse;
import org.vss.KVStore;

@Path(VssApiEndpoint.DELETE_OBJECT)
@Slf4j
public class DeleteObjectApi extends AbstractVssApi {
  @Inject
  public DeleteObjectApi(KVStore kvstore) {
    super(kvstore);
  }

  @POST
  @Produces(MediaType.APPLICATION_OCTET_STREAM)
  public Response execute(byte[] payload) {
    try {
      DeleteObjectRequest request = DeleteObjectRequest.parseFrom(payload);
      DeleteObjectResponse response = kvStore.delete(request);
      return toResponse(response);
    } catch (Exception e) {
      log.error("Exception in DeleteObjectApi: ", e);
      return toErrorResponse(e);
    }
  }
}
