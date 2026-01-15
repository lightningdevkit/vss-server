use crate::error::VssError;
use crate::kv_store::{KvStore, GLOBAL_VERSION_KEY};
use crate::types::{
	DeleteObjectRequest, GetObjectRequest, KeyValue, ListKeyVersionsRequest,
	ListKeyVersionsResponse, PutObjectRequest,
};
use async_trait::async_trait;
use bytes::Bytes;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// Defines KvStoreTestSuite which is required for an implementation to be VSS protocol compliant.
#[macro_export]
macro_rules! define_kv_store_tests {
	($test_suite_name:ident, $store_type:path, $create_store_expr: expr) => {
		use crate::api::error::VssError;
		use crate::api::kv_store_tests::KvStoreTestSuite;
		use async_trait::async_trait;
		struct $test_suite_name;

		#[async_trait]
		impl KvStoreTestSuite for $test_suite_name {
			type Store = $store_type;

			async fn create_store() -> Self::Store {
				$create_store_expr
			}
		}

		macro_rules! create_test {
			($test_fn:ident) => {
				#[tokio::test]
				async fn $test_fn() -> Result<(), VssError> {
					$test_suite_name::$test_fn().await?;
					Ok(())
				}
			};
		}

		create_test!(put_should_succeed_when_single_object_put_operation);
		create_test!(put_should_succeed_when_multi_object_put_operation);
		create_test!(put_should_fail_when_key_version_mismatched);
		create_test!(put_multi_object_should_fail_when_single_key_version_mismatched);
		create_test!(put_should_fail_when_global_version_mismatched);
		create_test!(put_should_succeed_when_no_global_version_is_given);
		create_test!(put_and_delete_should_succeed_as_atomic_transaction);
		create_test!(delete_should_succeed_when_item_exists);
		create_test!(delete_should_succeed_when_item_does_not_exist);
		create_test!(delete_should_be_idempotent);
		create_test!(get_should_throw_no_such_key_exception_when_key_does_not_exist);
		create_test!(get_should_return_correct_value_when_key_exists);
		create_test!(list_should_return_paginated_response);
		create_test!(list_should_honour_page_size_and_key_prefix_if_provided);
		create_test!(list_should_return_zero_global_version_when_global_versioning_not_enabled);
		create_test!(list_should_limit_max_page_size);
	};
}

/// Contains tests for a [`KvStore`] implementation to ensure it complies with the VSS protocol.
#[allow(missing_docs)]
#[async_trait]
pub trait KvStoreTestSuite {
	/// The type of store being tested. This must implement the [`KvStore`] trait.
	type Store: KvStore + 'static;

	/// Creates and returns a new instance of the store to be tested.
	async fn create_store() -> Self::Store;

	async fn put_should_succeed_when_single_object_put_operation() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		// Conditional Put
		ctx.put_objects(Some(0), vec![kv("k1", "k1v1", 0)]).await?;

		ctx.put_objects(Some(1), vec![kv("k1", "k1v2", 1)]).await?;

		// Non-conditional Put
		ctx.put_objects(Some(2), vec![kv("k2", "k2v1", -1)]).await?;
		ctx.put_objects(Some(3), vec![kv("k2", "k2v2", -1)]).await?;
		ctx.put_objects(Some(4), vec![kv("k2", "k2v3", -1)]).await?;

		// Get object k1
		let response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 2);
		assert_eq!(response.value, Bytes::from("k1v2"));

		// Get object k2
		let response = ctx.get_object("k2").await?;
		assert_eq!(response.key, "k2");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k2v3"));

		// Get GLOBAL_VERSION_KEY
		let response = ctx.get_object(GLOBAL_VERSION_KEY).await?;
		assert_eq!(response.version, 5);
		Ok(())
	}

	async fn put_should_succeed_when_multi_object_put_operation() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let key_values = vec![kv("k1", "k1v1", 0), kv("k2", "k2v1", 0)];

		ctx.put_objects(Some(0), key_values).await?;

		let second_request = vec![kv("k1", "k1v2", 1), kv("k2", "k2v2", 1)];
		ctx.put_objects(Some(1), second_request).await?;

		let response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 2);
		assert_eq!(response.value, Bytes::from("k1v2"));

		let response = ctx.get_object("k2").await?;
		assert_eq!(response.key, "k2");
		assert_eq!(response.version, 2);
		assert_eq!(response.value, Bytes::from("k2v2"));

		let response = ctx.get_object(GLOBAL_VERSION_KEY).await?;
		assert_eq!(response.version, 2);

		Ok(())
	}

	async fn put_should_fail_when_key_version_mismatched() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		// Initial put
		ctx.put_objects(Some(0), vec![kv("k1", "k1v1", 0)]).await?;

		// Attempt to put with mismatched key version
		let result = ctx.put_objects(Some(1), vec![kv("k1", "k1v2", 0)]).await;

		assert!(matches!(result, Err(VssError::ConflictError(..))));

		// Verify values didn't change
		let response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k1v1"));

		let response = ctx.get_object(GLOBAL_VERSION_KEY).await?;
		assert_eq!(response.version, 1);
		Ok(())
	}

	async fn put_multi_object_should_fail_when_single_key_version_mismatched(
	) -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let key_values = vec![kv("k1", "k1v1", 0), kv("k2", "k2v1", 0)];

		ctx.put_objects(None, key_values).await?;

		let second_request = vec![kv("k1", "k1v2", 0), kv("k2", "k2v2", 1)];

		// Should throw ConflictError due to key-version mismatch on "k1"
		let result = ctx.put_objects(None, second_request).await;
		assert!(matches!(result, Err(VssError::ConflictError(..))));

		// Verify values didn't change
		let response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k1v1"));

		let response = ctx.get_object("k2").await?;
		assert_eq!(response.key, "k2");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k2v1"));

		Ok(())
	}

	async fn put_should_fail_when_global_version_mismatched() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.put_objects(Some(0), vec![kv("k1", "k1v1", 0)]).await?;

		// Should throw ConflictError due to global_version mismatch
		let result = ctx.put_objects(Some(0), vec![kv("k1", "k1v2", 1)]).await;
		assert!(matches!(result, Err(VssError::ConflictError(_))));

		// Verify values didn't change
		let response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k1v1"));

		Ok(())
	}

	async fn put_should_succeed_when_no_global_version_is_given() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.put_objects(None, vec![kv("k1", "k1v1", 0)]).await?;
		ctx.put_objects(None, vec![kv("k1", "k1v2", 1)]).await?;

		let response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 2);
		assert_eq!(response.value, Bytes::from("k1v2"));

		let response = ctx.get_object(GLOBAL_VERSION_KEY).await?;
		assert_eq!(response.version, 0);

		Ok(())
	}

	async fn put_and_delete_should_succeed_as_atomic_transaction() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.put_objects(None, vec![kv("k1", "k1v1", 0)]).await?;

		// Put and Delete succeeds
		ctx.put_and_delete_objects(None, vec![kv("k2", "k2v1", 0)], vec![kv("k1", "", 1)]).await?;

		let response = ctx.get_object("k2").await?;
		assert_eq!(response.key, "k2");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k2v1"));

		// "k1" should be deleted
		let result = ctx.get_object("k1").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));

		// Delete fails (and hence put as well) due to mismatched version for the deleted item.
		let result = ctx
			.put_and_delete_objects(None, vec![kv("k3", "k3v1", 0)], vec![kv("k2", "", 3)])
			.await;
		assert!(matches!(result, Err(VssError::ConflictError(_))));

		// Verify "k3" was not inserted and "k2" still exists
		let result = ctx.get_object("k3").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));
		ctx.get_object("k2").await?;

		// Put fails (and hence delete as well) due to mismatched version for the put item.
		let result = ctx
			.put_and_delete_objects(None, vec![kv("k3", "k3v1", 1)], vec![kv("k2", "", 1)])
			.await;
		assert!(matches!(result, Err(VssError::ConflictError(_))));

		// Put and delete both fail due to mismatched global version.
		let result = ctx
			.put_and_delete_objects(Some(2), vec![kv("k3", "k3v1", 0)], vec![kv("k2", "", 1)])
			.await;
		assert!(matches!(result, Err(VssError::ConflictError(_))));

		// Verify "k3" was not inserted and "k2" still exists
		let result = ctx.get_object("k3").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));
		ctx.get_object("k2").await?;

		let response = ctx.get_object(GLOBAL_VERSION_KEY).await?;
		assert_eq!(response.version, 0);

		Ok(())
	}

	async fn delete_should_succeed_when_item_exists() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.put_objects(None, vec![kv("k1", "k1v1", 0)]).await?;

		// Conditional Delete
		ctx.delete_object(kv("k1", "", 1)).await?;

		let result = ctx.get_object("k1").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));

		ctx.put_objects(None, vec![kv("k1", "k1v1", 0)]).await?;
		ctx.put_objects(None, vec![kv("k1", "k1v2", 1)]).await?;

		// Non-conditional Delete
		ctx.delete_object(kv("k1", "", -1)).await?;

		let result = ctx.get_object("k1").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));

		Ok(())
	}

	async fn delete_should_succeed_when_item_does_not_exist() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.delete_object(kv("non_existent_key", "", 0)).await?;

		Ok(())
	}

	async fn delete_should_be_idempotent() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.put_objects(None, vec![kv("k1", "k1v1", 0)]).await?;
		ctx.delete_object(kv("k1", "", 1)).await?;
		ctx.delete_object(kv("k1", "", 1)).await?;

		let result = ctx.get_object("k1").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));

		Ok(())
	}

	async fn get_should_throw_no_such_key_exception_when_key_does_not_exist() -> Result<(), VssError>
	{
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let result = ctx.get_object("non_existent_key").await;
		assert!(matches!(result, Err(VssError::NoSuchKeyError(_))));

		Ok(())
	}
	async fn get_should_return_correct_value_when_key_exists() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		ctx.put_objects(Some(0), vec![kv("k1", "k1v1", 0)]).await?;

		let mut response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k1v1"));

		let key_values = vec![kv("k1", "k1v2", 1), kv("k2", "k2v1", 0)];
		ctx.put_objects(Some(1), key_values).await?;

		response = ctx.get_object("k1").await?;
		assert_eq!(response.key, "k1");
		assert_eq!(response.version, 2);
		assert_eq!(response.value, Bytes::from("k1v2"));

		response = ctx.get_object("k2").await?;
		assert_eq!(response.key, "k2");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k2v1"));

		let key_values = vec![kv("k2", "k2v2", 1), kv("k3", "k3v1", 0)];
		ctx.put_objects(Some(2), key_values).await?;

		response = ctx.get_object("k2").await?;
		assert_eq!(response.key, "k2");
		assert_eq!(response.version, 2);
		assert_eq!(response.value, Bytes::from("k2v2"));

		response = ctx.get_object("k3").await?;
		assert_eq!(response.key, "k3");
		assert_eq!(response.version, 1);
		assert_eq!(response.value, Bytes::from("k3v1"));

		Ok(())
	}

	async fn list_should_return_paginated_response() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let total_kv_objects = 1000;
		for i in 0..total_kv_objects {
			ctx.put_objects(Some(i as i64), vec![kv(&format!("k{}", i), "k1v1", 0)]).await?;
		}

		ctx.put_objects(Some(1000), vec![kv("k1", "k1v2", 1)]).await?;
		ctx.put_objects(Some(1001), vec![kv("k2", "k2v2", 1)]).await?;
		ctx.put_objects(Some(1002), vec![kv("k2", "k2v3", 2)]).await?;

		let mut next_page_token: Option<String> = None;
		let mut all_key_versions: Vec<KeyValue> = Vec::new();

		loop {
			let current_page = match next_page_token.take() {
				None => {
					let page = ctx.list(None, None, None).await?;
					assert_eq!(page.global_version, Some(1003));
					page
				},
				Some(next_page_token) => {
					let page = ctx.list(Some(next_page_token), None, None).await?;
					assert!(page.global_version.is_none());
					page
				},
			};

			if current_page.key_versions.is_empty() {
				break;
			}

			all_key_versions.extend(current_page.key_versions);
			next_page_token = current_page.next_page_token;
		}

		if let Some(k1_response) = all_key_versions.iter().find(|kv| kv.key == "k1") {
			assert_eq!(k1_response.key, "k1");
			assert_eq!(k1_response.version, 2);
			assert_eq!(k1_response.value, Bytes::new());
		}

		if let Some(k2_response) = all_key_versions.iter().find(|kv| kv.key == "k2") {
			assert_eq!(k2_response.key, "k2");
			assert_eq!(k2_response.version, 3);
			assert_eq!(k2_response.value, Bytes::new());
		}

		let unique_keys: std::collections::HashSet<String> =
			all_key_versions.into_iter().map(|kv| kv.key).collect();
		assert_eq!(unique_keys.len(), total_kv_objects as usize);
		assert!(!unique_keys.contains(GLOBAL_VERSION_KEY));

		Ok(())
	}

	async fn list_should_honour_page_size_and_key_prefix_if_provided() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let total_kv_objects = 20;
		let page_size = 5;
		for i in 0..total_kv_objects {
			ctx.put_objects(Some(i as i64), vec![kv(&format!("{}k", i), "k1v1", 0)]).await?;
		}

		let mut next_page_token: Option<String> = None;
		let mut all_key_versions: Vec<KeyValue> = Vec::new();
		let key_prefix = "1";

		loop {
			let current_page = match next_page_token.take() {
				None => ctx.list(None, Some(page_size), Some(key_prefix.to_string())).await?,
				Some(next_page_token) => {
					ctx.list(Some(next_page_token), Some(page_size), Some(key_prefix.to_string()))
						.await?
				},
			};

			if current_page.key_versions.is_empty() {
				break;
			}

			assert!(current_page.key_versions.len() <= page_size as usize);
			all_key_versions.extend(current_page.key_versions);
			next_page_token = current_page.next_page_token;
		}

		let unique_keys: std::collections::HashSet<String> =
			all_key_versions.into_iter().map(|kv| kv.key).collect();

		assert_eq!(unique_keys.len(), 11);
		let expected_keys: std::collections::HashSet<String> =
			["1k", "10k", "11k", "12k", "13k", "14k", "15k", "16k", "17k", "18k", "19k"]
				.into_iter()
				.map(|s| s.to_string())
				.collect();
		assert_eq!(unique_keys, expected_keys);

		Ok(())
	}

	async fn list_should_return_zero_global_version_when_global_versioning_not_enabled(
	) -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let total_kv_objects = 1000;
		for i in 0..total_kv_objects {
			ctx.put_objects(None, vec![kv(&format!("k{}", i), "k1v1", 0)]).await?;
		}

		let mut next_page_token: Option<String> = None;
		let mut all_key_versions: Vec<KeyValue> = Vec::new();

		loop {
			let current_page = match next_page_token.take() {
				None => {
					let page = ctx.list(None, None, None).await?;
					assert_eq!(page.global_version.unwrap_or(0), 0);
					page
				},
				Some(next_page_token) => ctx.list(Some(next_page_token), None, None).await?,
			};

			if current_page.key_versions.is_empty() {
				break;
			}

			all_key_versions.extend(current_page.key_versions);
			next_page_token = current_page.next_page_token;
		}

		let unique_keys: std::collections::HashSet<String> =
			all_key_versions.into_iter().map(|kv| kv.key).collect();
		assert_eq!(unique_keys.len(), total_kv_objects as usize);
		assert!(!unique_keys.contains(GLOBAL_VERSION_KEY));

		Ok(())
	}

	async fn list_should_limit_max_page_size() -> Result<(), VssError> {
		let kv_store = Self::create_store().await;
		let ctx = TestContext::new(&kv_store);

		let total_kv_objects = 10_000;
		let vss_arbitrary_page_size_max = 3000;
		for i in 0..total_kv_objects {
			ctx.put_objects(Some(i as i64), vec![kv(&format!("k{}", i), "k1v1", 0)]).await?;
		}

		let mut next_page_token: Option<String> = None;
		let mut all_key_versions: Vec<KeyValue> = Vec::new();

		loop {
			let current_page = match next_page_token.take() {
				None => ctx.list(None, None, None).await?,
				Some(next_page_token) => ctx.list(Some(next_page_token), None, None).await?,
			};
			if current_page.key_versions.is_empty() {
				break;
			}

			assert!(
				current_page.key_versions.len() < vss_arbitrary_page_size_max as usize,
				"Page size exceeds the maximum allowed size"
			);
			all_key_versions.extend(current_page.key_versions);
			next_page_token = current_page.next_page_token;
		}

		assert_eq!(all_key_versions.len(), total_kv_objects as usize);

		Ok(())
	}
}

/// Represents the context used for testing [`KvStore`] operations.
pub struct TestContext<'a> {
	kv_store: &'a dyn KvStore,
	user_token: String,
	store_id: String,
}

impl<'a> TestContext<'a> {
	/// Creates a new [`TestContext`] with the given [`KvStore`] implementation.
	pub fn new(kv_store: &'a dyn KvStore) -> Self {
		let store_id: String = (0..7).map(|_| thread_rng().sample(Alphanumeric) as char).collect();
		TestContext { kv_store, user_token: "userToken".to_string(), store_id }
	}

	async fn get_object(&self, key: &str) -> Result<KeyValue, VssError> {
		let request = GetObjectRequest { store_id: self.store_id.clone(), key: key.to_string() };
		let response = self.kv_store.get(self.user_token.clone(), request).await?;
		Ok(response.value.unwrap())
	}

	async fn put_objects(
		&self, global_version: Option<i64>, key_values: Vec<KeyValue>,
	) -> Result<(), VssError> {
		let request = PutObjectRequest {
			store_id: self.store_id.clone(),
			transaction_items: key_values,
			delete_items: vec![],
			global_version,
		};
		self.kv_store.put(self.user_token.clone(), request).await?;
		Ok(())
	}

	async fn put_and_delete_objects(
		&self, global_version: Option<i64>, put_key_values: Vec<KeyValue>,
		delete_key_values: Vec<KeyValue>,
	) -> Result<(), VssError> {
		let request = PutObjectRequest {
			store_id: self.store_id.clone(),
			transaction_items: put_key_values,
			delete_items: delete_key_values,
			global_version,
		};
		self.kv_store.put(self.user_token.clone(), request).await?;
		Ok(())
	}

	async fn delete_object(&self, key_value: KeyValue) -> Result<(), VssError> {
		let request =
			DeleteObjectRequest { store_id: self.store_id.clone(), key_value: Some(key_value) };
		self.kv_store.delete(self.user_token.clone(), request).await?;
		Ok(())
	}

	async fn list(
		&self, next_page_token: Option<String>, page_size: Option<i32>, key_prefix: Option<String>,
	) -> Result<ListKeyVersionsResponse, VssError> {
		let request = ListKeyVersionsRequest {
			store_id: self.store_id.clone(),
			page_token: next_page_token,
			page_size,
			key_prefix,
		};
		let response = self.kv_store.list_key_versions(self.user_token.clone(), request).await?;
		Ok(response)
	}
}

fn kv(key: &str, value: &str, version: i64) -> KeyValue {
	KeyValue { key: key.to_string(), version, value: Bytes::from(value.to_string()) }
}
