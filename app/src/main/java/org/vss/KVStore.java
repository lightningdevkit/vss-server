package org.vss;

public interface KVStore {

  String GLOBAL_VERSION_KEY = "vss_global_version";

  GetObjectResponse get(GetObjectRequest request);

  PutObjectResponse put(PutObjectRequest request);

  DeleteObjectResponse delete(DeleteObjectRequest request);

  ListKeyVersionsResponse listKeyVersions(ListKeyVersionsRequest request);
}
