package org.vss;

public interface KVStore {

  String GLOBAL_VERSION_KEY = "vss_global_version";

  GetObjectResponse get(String userToken, GetObjectRequest request);

  PutObjectResponse put(String userToken, PutObjectRequest request);

  DeleteObjectResponse delete(String userToken, DeleteObjectRequest request);

  ListKeyVersionsResponse listKeyVersions(String userToken, ListKeyVersionsRequest request);
}
