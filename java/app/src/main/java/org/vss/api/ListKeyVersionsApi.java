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
import org.vss.KVStore;
import org.vss.ListKeyVersionsRequest;
import org.vss.ListKeyVersionsResponse;
import org.vss.auth.AuthResponse;
import org.vss.auth.Authorizer;

@Path(VssApiEndpoint.LIST_KEY_VERSIONS)
@Slf4j
public class ListKeyVersionsApi extends AbstractVssApi {

  @Inject
  public ListKeyVersionsApi(KVStore kvStore, Authorizer authorizer) {
    super(kvStore, authorizer);
  }

  @POST
  @Produces(MediaType.APPLICATION_OCTET_STREAM)
  public Response execute(byte[] payload, @Context HttpHeaders headers) {
    try {
      AuthResponse authResponse = authorizer.verify(headers);
      ListKeyVersionsRequest request = ListKeyVersionsRequest.parseFrom(payload);
      ListKeyVersionsResponse response = kvStore.listKeyVersions(authResponse.getUserToken(), request);
      return toResponse(response);
    } catch (Exception e) {
      log.error("Exception in ListKeyVersionsApi: ", e);
      return toErrorResponse(e);
    }
  }
}
