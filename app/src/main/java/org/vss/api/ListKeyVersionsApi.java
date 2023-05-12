package org.vss.api;

import jakarta.inject.Inject;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import lombok.extern.slf4j.Slf4j;
import org.vss.KVStore;
import org.vss.ListKeyVersionsRequest;
import org.vss.ListKeyVersionsResponse;

@Path(VssApiEndpoint.LIST_KEY_VERSIONS)
@Slf4j
public class ListKeyVersionsApi extends AbstractVssApi {

  @Inject
  public ListKeyVersionsApi(KVStore kvStore) {
    super(kvStore);
  }

  @POST
  @Produces(MediaType.APPLICATION_OCTET_STREAM)
  public Response execute(byte[] payload) {
    try {
      ListKeyVersionsRequest request = ListKeyVersionsRequest.parseFrom(payload);
      ListKeyVersionsResponse response = kvStore.listKeyVersions(request);
      return toResponse(response);
    } catch (Exception e) {
      log.error("Exception in ListKeyVersionsApi: ", e);
      return toErrorResponse(e);
    }
  }
}
