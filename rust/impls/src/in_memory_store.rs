use crate::postgres_store::{
	VssDbRecord, LIST_KEY_VERSIONS_MAX_PAGE_SIZE, MAX_PUT_REQUEST_ITEM_COUNT,
};
use api::error::VssError;
use api::kv_store::{KvStore, GLOBAL_VERSION_KEY, INITIAL_RECORD_VERSION};
use api::types::{
	DeleteObjectRequest, DeleteObjectResponse, GetObjectRequest, GetObjectResponse, KeyValue,
	ListKeyVersionsRequest, ListKeyVersionsResponse, PutObjectRequest, PutObjectResponse,
};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::prelude::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

fn build_storage_key(user_token: &str, store_id: &str, key: &str) -> String {
	format!("{}#{}#{}", user_token, store_id, key)
}

/// In-memory implementation of the VSS Store.
pub struct InMemoryBackendImpl {
	store: Arc<RwLock<HashMap<String, VssDbRecord>>>,
}

impl InMemoryBackendImpl {
	/// Creates an in-memory instance.
	pub fn new() -> Self {
		Self { store: Arc::new(RwLock::new(HashMap::new())) }
	}

	fn get_current_global_version(
		&self, guard: &HashMap<String, VssDbRecord>, user_token: &str, store_id: &str,
	) -> i64 {
		let global_key = build_storage_key(user_token, store_id, GLOBAL_VERSION_KEY);
		guard.get(&global_key).map(|r| r.version).unwrap_or(0)
	}
}

// Validation functions - check if operations can succeed without modifying data
fn validate_put_operation(
	store: &HashMap<String, VssDbRecord>, user_token: &str, store_id: &str, key_value: &KeyValue,
) -> Result<(), VssError> {
	let key = build_storage_key(user_token, store_id, &key_value.key);

	if key_value.version == -1 {
		// Non-conditional upsert always succeeds
		Ok(())
	} else if key_value.version == 0 {
		if store.contains_key(&key) {
			Err(VssError::ConflictError(format!(
				"Key {} already exists for conditional insert",
				key_value.key
			)))
		} else {
			Ok(())
		}
	} else {
		if let Some(existing) = store.get(&key) {
			if existing.version == key_value.version {
				Ok(())
			} else {
				Err(VssError::ConflictError(format!(
					"Version mismatch for key {}: expected {}, found {}",
					key_value.key, key_value.version, existing.version
				)))
			}
		} else {
			Err(VssError::ConflictError(format!(
				"Key {} does not exist for conditional update",
				key_value.key
			)))
		}
	}
}

fn validate_delete_operation(
	store: &HashMap<String, VssDbRecord>, user_token: &str, store_id: &str, key_value: &KeyValue,
) -> Result<(), VssError> {
	let key = build_storage_key(user_token, store_id, &key_value.key);

	if key_value.version == -1 {
		// Non-conditional delete always succeeds
		Ok(())
	} else {
		if let Some(existing) = store.get(&key) {
			if existing.version == key_value.version {
				Ok(())
			} else {
				Err(VssError::ConflictError(format!(
					"Version mismatch for delete key {}: expected {}, found {}",
					key_value.key, key_value.version, existing.version
				)))
			}
		} else {
			Err(VssError::ConflictError(format!(
				"Key {} does not exist for conditional delete",
				key_value.key
			)))
		}
	}
}

fn execute_non_conditional_upsert(
	store: &mut HashMap<String, VssDbRecord>, user_token: &str, store_id: &str, key_value: KeyValue,
) {
	let key = build_storage_key(user_token, store_id, &key_value.key);
	let now = Utc::now();

	match store.entry(key) {
		std::collections::hash_map::Entry::Occupied(mut occ) => {
			let existing = occ.get_mut();
			existing.version = INITIAL_RECORD_VERSION as i64;
			existing.value = key_value.value.to_vec();
			existing.last_updated_at = now;
		},
		std::collections::hash_map::Entry::Vacant(vac) => {
			let new_record = VssDbRecord {
				user_token: user_token.to_string(),
				store_id: store_id.to_string(),
				key: key_value.key,
				value: key_value.value.to_vec(),
				version: INITIAL_RECORD_VERSION as i64,
				created_at: now,
				last_updated_at: now,
			};
			vac.insert(new_record);
		},
	}
}

fn execute_conditional_insert(
	store: &mut HashMap<String, VssDbRecord>, user_token: &str, store_id: &str, key_value: KeyValue,
) {
	let key = build_storage_key(user_token, store_id, &key_value.key);
	let now = Utc::now();

	let new_record = VssDbRecord {
		user_token: user_token.to_string(),
		store_id: store_id.to_string(),
		key: key_value.key,
		value: key_value.value.to_vec(),
		version: INITIAL_RECORD_VERSION as i64,
		created_at: now,
		last_updated_at: now,
	};
	store.insert(key, new_record);
}

fn execute_conditional_update(
	store: &mut HashMap<String, VssDbRecord>, user_token: &str, store_id: &str, key_value: KeyValue,
) {
	let key = build_storage_key(user_token, store_id, &key_value.key);
	let now = Utc::now();

	if let Some(existing) = store.get_mut(&key) {
		existing.version = key_value.version.saturating_add(1);
		existing.value = key_value.value.to_vec();
		existing.last_updated_at = now;
	}
}

fn execute_put_object(
	store: &mut HashMap<String, VssDbRecord>, user_token: &str, store_id: &str, key_value: KeyValue,
) {
	if key_value.version == -1 {
		execute_non_conditional_upsert(store, user_token, store_id, key_value);
	} else if key_value.version == 0 {
		execute_conditional_insert(store, user_token, store_id, key_value);
	} else {
		execute_conditional_update(store, user_token, store_id, key_value);
	}
}

fn execute_delete_object(
	store: &mut HashMap<String, VssDbRecord>, user_token: &str, store_id: &str,
	key_value: &KeyValue,
) {
	let key = build_storage_key(user_token, store_id, &key_value.key);
	store.remove(&key);
}

#[async_trait]
impl KvStore for InMemoryBackendImpl {
	async fn get(
		&self, user_token: String, request: GetObjectRequest,
	) -> Result<GetObjectResponse, VssError> {
		let key = build_storage_key(&user_token, &request.store_id, &request.key);
		let guard = self.store.read().await;

		if let Some(record) = guard.get(&key) {
			Ok(GetObjectResponse {
				value: Some(KeyValue {
					key: record.key.clone(),
					value: Bytes::from(record.value.clone()),
					version: record.version,
				}),
			})
		} else if request.key == GLOBAL_VERSION_KEY {
			Ok(GetObjectResponse {
				value: Some(KeyValue {
					key: GLOBAL_VERSION_KEY.to_string(),
					value: Bytes::new(),
					version: 0,
				}),
			})
		} else {
			Err(VssError::NoSuchKeyError("Requested key not found.".to_string()))
		}
	}

	async fn put(
		&self, user_token: String, request: PutObjectRequest,
	) -> Result<PutObjectResponse, VssError> {
		if request.transaction_items.len() + request.delete_items.len() > MAX_PUT_REQUEST_ITEM_COUNT
		{
			return Err(VssError::InvalidRequestError(format!(
				"Number of write items per request should be less than equal to {}",
				MAX_PUT_REQUEST_ITEM_COUNT
			)));
		}

		let store_id = request.store_id.clone();
		let mut guard = self.store.write().await;

		if let Some(version) = request.global_version {
			validate_put_operation(
				&guard,
				&user_token,
				&store_id,
				&KeyValue { key: GLOBAL_VERSION_KEY.to_string(), value: Bytes::new(), version },
			)?;
		}

		for key_value in &request.transaction_items {
			validate_put_operation(&guard, &user_token, &store_id, key_value)?;
		}

		for key_value in &request.delete_items {
			validate_delete_operation(&guard, &user_token, &store_id, key_value)?;
		}

		for key_value in request.transaction_items {
			execute_put_object(&mut guard, &user_token, &store_id, key_value);
		}

		for key_value in &request.delete_items {
			execute_delete_object(&mut guard, &user_token, &store_id, key_value);
		}

		if let Some(version) = request.global_version {
			execute_put_object(
				&mut guard,
				&user_token,
				&store_id,
				KeyValue { key: GLOBAL_VERSION_KEY.to_string(), value: Bytes::new(), version },
			);
		}

		Ok(PutObjectResponse {})
	}

	async fn delete(
		&self, user_token: String, request: DeleteObjectRequest,
	) -> Result<DeleteObjectResponse, VssError> {
		let key_value = request.key_value.ok_or_else(|| {
			VssError::InvalidRequestError("key_value missing in DeleteObjectRequest".to_string())
		})?;

		let store_id = request.store_id.clone();
		let mut guard = self.store.write().await;

		execute_delete_object(&mut guard, &user_token, &store_id, &key_value);

		Ok(DeleteObjectResponse {})
	}

	async fn list_key_versions(
		&self, user_token: String, request: ListKeyVersionsRequest,
	) -> Result<ListKeyVersionsResponse, VssError> {
		let store_id = request.store_id;
		let key_prefix = request.key_prefix.unwrap_or("".to_string());
		let page_token_option = request.page_token;
		let page_size = request.page_size.unwrap_or(i32::MAX);
		let limit = std::cmp::min(page_size, LIST_KEY_VERSIONS_MAX_PAGE_SIZE) as usize;

		let (keys_with_versions, global_version) = {
			let guard = self.store.read().await;

			let mut global_version: Option<i64> = None;
			if page_token_option.is_none() {
				global_version =
					Some(self.get_current_global_version(&guard, &user_token, &store_id));
			}

			let storage_prefix = format!("{}#{}#", user_token, store_id);
			let mut temp: Vec<(String, i64)> = Vec::new();

			for (storage_key, r) in guard.iter() {
				if !storage_key.starts_with(&storage_prefix) {
					continue;
				}
				let key = &storage_key[storage_prefix.len()..];
				if key == GLOBAL_VERSION_KEY {
					continue;
				}
				if !key_prefix.is_empty() && !key.starts_with(&key_prefix) {
					continue;
				}
				temp.push((key.to_string(), r.version));
			}

			(temp, global_version)
		};

		let mut keys_with_versions = keys_with_versions;
		keys_with_versions.sort_by(|a, b| a.0.cmp(&b.0));

		let start_idx = if page_token_option.is_none() {
			0
		} else if page_token_option.as_deref() == Some("") {
			keys_with_versions.len()
		} else {
			let token = page_token_option.as_deref().unwrap();
			keys_with_versions
				.iter()
				.position(|(k, _)| k.as_str() > token)
				.unwrap_or(keys_with_versions.len())
		};

		let page_items: Vec<KeyValue> = keys_with_versions
			.iter()
			.skip(start_idx)
			.take(limit)
			.map(|(key, version)| KeyValue {
				key: key.clone(),
				value: Bytes::new(),
				version: *version,
			})
			.collect();

		let next_page_token = if page_items.is_empty() {
			Some("".to_string())
		} else {
			page_items.last().map(|kv| kv.key.clone())
		};

		Ok(ListKeyVersionsResponse { key_versions: page_items, next_page_token, global_version })
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use api::define_kv_store_tests;
	use api::types::{GetObjectRequest, KeyValue, PutObjectRequest};
	use bytes::Bytes;
	use tokio::test;

	define_kv_store_tests!(InMemoryKvStoreTest, InMemoryBackendImpl, InMemoryBackendImpl::new());

	#[test]
	async fn test_in_memory_crud() {
		let store = InMemoryBackendImpl::new();
		let user_token = "test_user".to_string();
		let store_id = "test_store".to_string();

		let put_request = PutObjectRequest {
			store_id: store_id.clone(),
			transaction_items: vec![KeyValue {
				key: "key1".to_string(),
				value: Bytes::from("value1"),
				version: 0,
			}],
			delete_items: vec![],
			global_version: None,
		};
		store.put(user_token.clone(), put_request).await.unwrap();

		let get_request = GetObjectRequest { store_id: store_id.clone(), key: "key1".to_string() };
		let response = store.get(user_token.clone(), get_request).await.unwrap();
		let key_value = response.value.unwrap();
		assert_eq!(key_value.value, Bytes::from("value1"));
		assert_eq!(key_value.version, 1, "Expected version 1 after put");

		let list_request = ListKeyVersionsRequest {
			store_id: store_id.clone(),
			key_prefix: None,
			page_size: Some(1),
			page_token: None,
		};
		let response = store.list_key_versions(user_token.clone(), list_request).await.unwrap();
		assert_eq!(response.key_versions.len(), 1);
		assert_eq!(response.key_versions[0].key, "key1");
		assert_eq!(response.key_versions[0].version, 1);

		let delete_request = DeleteObjectRequest {
			store_id: store_id.clone(),
			key_value: Some(KeyValue { key: "key1".to_string(), value: Bytes::new(), version: 1 }),
		};
		store.delete(user_token.clone(), delete_request).await.unwrap();

		let get_request = GetObjectRequest { store_id: store_id.clone(), key: "key1".to_string() };
		assert!(matches!(
			store.get(user_token.clone(), get_request).await,
			Err(VssError::NoSuchKeyError(_))
		));

		let global_request =
			GetObjectRequest { store_id: store_id.clone(), key: GLOBAL_VERSION_KEY.to_string() };
		let response = store.get(user_token.clone(), global_request).await.unwrap();
		assert_eq!(response.value.unwrap().version, 0, "Expected global_version=0");
	}
}
