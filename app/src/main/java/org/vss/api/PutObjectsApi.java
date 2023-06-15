package org.vss.api;

import jakarta.inject.Inject;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import lombok.extern.slf4j.Slf4j;
import org.vss.KVStore;
import org.vss.PutObjectRequest;
import org.vss.PutObjectResponse;

@Path(VssApiEndpoint.PUT_OBJECTS)
@Slf4j
public class PutObjectsApi extends AbstractVssApi {

  @Inject
  public PutObjectsApi(KVStore kvStore) {
    super(kvStore);
  }

  @POST
  @Produces(MediaType.APPLICATION_OCTET_STREAM)
  public Response execute(byte[] payload) {
    try {
      PutObjectRequest putObjectRequest = PutObjectRequest.parseFrom(payload);
      PutObjectResponse response = kvStore.put(putObjectRequest);
      return toResponse(response);
    } catch (Exception e) {
      log.error("Exception in PutObjectsApi: ", e);
      return toErrorResponse(e);
    }
  }
}
