package org.vss.api;

import jakarta.inject.Inject;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import lombok.extern.slf4j.Slf4j;
import org.vss.GetObjectRequest;
import org.vss.GetObjectResponse;
import org.vss.KVStore;

@Path(VssApiEndpoint.GET_OBJECT)
@Slf4j
public class GetObjectApi extends AbstractVssApi {

  @Inject
  public GetObjectApi(KVStore kvstore) {
    super(kvstore);
  }

  @POST
  @Produces(MediaType.APPLICATION_OCTET_STREAM)
  public Response execute(byte[] payload) {
    try {
      GetObjectRequest request = GetObjectRequest.parseFrom(payload);
      GetObjectResponse response = kvStore.get(request);
      return toResponse(response);
    } catch (Exception e) {
      log.error("Exception in GetObjectApi: ", e);
      return toErrorResponse(e);
    }
  }
}
