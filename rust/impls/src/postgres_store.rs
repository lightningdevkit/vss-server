use api::error::VssError;
use api::kv_store::{KvStore, GLOBAL_VERSION_KEY, INITIAL_RECORD_VERSION};
use api::types::{
	DeleteObjectRequest, DeleteObjectResponse, GetObjectRequest, GetObjectResponse, KeyValue,
	ListKeyVersionsRequest, ListKeyVersionsResponse, PutObjectRequest, PutObjectResponse,
};
use async_trait::async_trait;
use bb8_postgres::bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use bytes::Bytes;
use chrono::Utc;
use std::cmp::min;
use std::io;
use std::io::{Error, ErrorKind};
use tokio_postgres::{error, NoTls, Transaction};

pub(crate) struct VssDbRecord {
	pub(crate) user_token: String,
	pub(crate) store_id: String,
	pub(crate) key: String,
	pub(crate) value: Vec<u8>,
	pub(crate) version: i64,
	pub(crate) created_at: chrono::DateTime<Utc>,
	pub(crate) last_updated_at: chrono::DateTime<Utc>,
}
const KEY_COLUMN: &str = "key";
const VALUE_COLUMN: &str = "value";
const VERSION_COLUMN: &str = "version";

const DB_VERSION_COLUMN: &str = "db_version";
#[cfg(test)]
const MIGRATION_LOG_COLUMN: &str = "upgrade_from";

const CHECK_DB_STMT: &str = "SELECT 1 FROM pg_database WHERE datname = $1";
const INIT_DB_CMD: &str = "CREATE DATABASE";
#[cfg(test)]
const DROP_DB_CMD: &str = "DROP DATABASE";
const GET_VERSION_STMT: &str = "SELECT db_version FROM vss_db_version;";
const UPDATE_VERSION_STMT: &str = "UPDATE vss_db_version SET db_version=$1;";
const LOG_MIGRATION_STMT: &str = "INSERT INTO vss_db_upgrades VALUES($1);";
#[cfg(test)]
const GET_MIGRATION_LOG_STMT: &str = "SELECT upgrade_from FROM vss_db_upgrades;";

// APPEND-ONLY list of migration statements
//
// Each statement MUST be applied in-order, and only once per database.
//
// We make an exception for the vss_db table creation statement, as users of VSS could have initialized the table
// themselves.
const MIGRATIONS: &[&str] = &[
	"CREATE TABLE vss_db_version (db_version INTEGER);",
	"INSERT INTO vss_db_version VALUES(1);",
	// A write-only log of all the migrations performed on this database, useful for debugging and testing
	"CREATE TABLE vss_db_upgrades (upgrade_from INTEGER);",
	// We do not complain if the table already exists, as users of VSS could have already created this table
	"CREATE TABLE IF NOT EXISTS vss_db (
	    user_token character varying(120) NOT NULL CHECK (user_token <> ''),
	    store_id character varying(120) NOT NULL CHECK (store_id <> ''),
	    key character varying(600) NOT NULL,
	    value bytea NULL,
	    version bigint NOT NULL,
	    created_at TIMESTAMP WITH TIME ZONE,
	    last_updated_at TIMESTAMP WITH TIME ZONE,
	    PRIMARY KEY (user_token, store_id, key)
	);",
];
#[cfg(test)]
const DUMMY_MIGRATION: &str = "SELECT 1 WHERE FALSE;";

/// The maximum number of key versions that can be returned in a single page.
///
/// This constant helps control memory and bandwidth usage for list operations,
/// preventing overly large payloads. If the number of results exceeds this limit,
/// the response will be paginated.
pub const LIST_KEY_VERSIONS_MAX_PAGE_SIZE: i32 = 100;

/// The maximum number of items allowed in a single `PutObjectRequest`.
///
/// Setting an upper bound on the number of items helps ensure that
/// each request stays within acceptable memory and performance limits.
/// Exceeding this value will result in request rejection through [`VssError::InvalidRequestError`].
pub const MAX_PUT_REQUEST_ITEM_COUNT: usize = 1000;

/// A [PostgreSQL](https://www.postgresql.org/) based backend implementation for VSS.
pub struct PostgresBackendImpl {
	pool: Pool<PostgresConnectionManager<NoTls>>,
}

async fn initialize_vss_database(postgres_endpoint: &str, db_name: &str) -> Result<(), Error> {
	let postgres_dsn = format!("{}/{}", postgres_endpoint, "postgres");
	let (client, connection) = tokio_postgres::connect(&postgres_dsn, NoTls)
		.await
		.map_err(|e| Error::new(ErrorKind::Other, format!("Connection error: {}", e)))?;
	// Connection must be driven on a separate task, and will resolve when the client is dropped
	tokio::spawn(async move {
		if let Err(e) = connection.await {
			eprintln!("Connection error: {}", e);
		}
	});

	let num_rows = client.execute(CHECK_DB_STMT, &[&db_name]).await.map_err(|e| {
		Error::new(
			ErrorKind::Other,
			format!("Failed to check presence of database {}: {}", db_name, e),
		)
	})?;

	if num_rows == 0 {
		let stmt = format!("{} {};", INIT_DB_CMD, db_name);
		client.execute(&stmt, &[]).await.map_err(|e| {
			Error::new(ErrorKind::Other, format!("Failed to create database {}: {}", db_name, e))
		})?;
		println!("Created database {}", db_name);
	}

	Ok(())
}

#[cfg(test)]
async fn drop_database(postgres_endpoint: &str, db_name: &str) -> Result<(), Error> {
	let postgres_dsn = format!("{}/{}", postgres_endpoint, "postgres");
	let (client, connection) = tokio_postgres::connect(&postgres_dsn, NoTls)
		.await
		.map_err(|e| Error::new(ErrorKind::Other, format!("Connection error: {}", e)))?;
	// Connection must be driven on a separate task, and will resolve when the client is dropped
	tokio::spawn(async move {
		if let Err(e) = connection.await {
			eprintln!("Connection error: {}", e);
		}
	});

	let drop_database_statement = format!("{} {};", DROP_DB_CMD, db_name);
	let num_rows = client.execute(&drop_database_statement, &[]).await.map_err(|e| {
		Error::new(
			ErrorKind::Other,
			format!("Failed to drop database {}: {}", db_name, e),
		)
	})?;
	assert_eq!(num_rows, 0);

	Ok(())
}

impl PostgresBackendImpl {
	/// Constructs a [`PostgresBackendImpl`] using `dsn` for PostgreSQL connection information.
	pub async fn new(postgres_endpoint: &str, db_name: &str) -> Result<Self, Error> {
		initialize_vss_database(postgres_endpoint, db_name).await?;

		let vss_dsn = format!("{}/{}", postgres_endpoint, db_name);
		let manager =
			PostgresConnectionManager::new_from_stringlike(vss_dsn, NoTls).map_err(|e| {
				Error::new(
					ErrorKind::Other,
					format!("Failed to create PostgresConnectionManager: {}", e),
				)
			})?;
		// By default, Pool maintains 0 long-running connections, so returning a pool
		// here is no guarantee that Pool established a connection to the database.
		//
		// See Builder::min_idle to increase the long-running connection count.
		let pool = Pool::builder()
			.build(manager)
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Failed to build Pool: {}", e)))?;
		let postgres_backend = PostgresBackendImpl { pool };

		#[cfg(not(test))]
		postgres_backend.migrate_vss_database(MIGRATIONS).await?;

		Ok(postgres_backend)
	}

	async fn migrate_vss_database(&self, migrations: &[&str]) -> Result<(usize, usize), Error> {
		let mut conn = self.pool.get().await.map_err(|e| {
			Error::new(
				ErrorKind::Other,
				format!("Failed to fetch a connection from Pool: {}", e),
			)
		})?;

		// Get the next migration to be applied.
		let migration_start = match conn.query_one(GET_VERSION_STMT, &[]).await {
			Ok(row) => {
				let i: i32 = row.get(DB_VERSION_COLUMN);
				usize::try_from(i).expect("The column should always contain unsigned integers")
			},
			Err(e) => {
				// If the table is not defined, start at migration 0
				if let Some(&error::SqlState::UNDEFINED_TABLE) = e.code() {
					0
				} else {
					return Err(Error::new(
						ErrorKind::Other,
						format!("Failed to query the version of the database schema: {}", e),
					));
				}
			},
		};

		let tx = conn
			.transaction()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Transaction start error: {}", e)))?;

		if migration_start == migrations.len() {
			// No migrations needed, we are done
			return Ok((migration_start, migrations.len()));
		} else if migration_start > migrations.len() {
			panic!("We do not allow downgrades");
		}

		println!("Applying migration(s) {} through {}", migration_start, migrations.len() - 1);

		for (idx, &stmt) in (&migrations[migration_start..]).iter().enumerate() {
			let _num_rows = tx.execute(stmt, &[]).await.map_err(|e| {
				Error::new(
					ErrorKind::Other,
					format!(
						"Database migration no {} with stmt {} failed: {}",
						migration_start + idx,
						stmt,
						e
					),
				)
			})?;
		}

		let num_rows = tx
			.execute(
				LOG_MIGRATION_STMT,
				&[&i32::try_from(migration_start).expect("Read from an i32 further above")],
			)
			.await
			.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Failed to log database migration: {}", e))
			})?;
		assert_eq!(num_rows, 1, "LOG_MIGRATION_STMT should only add one row at a time");

		let next_migration_start =
			i32::try_from(migrations.len()).expect("Length is definitely smaller than i32::MAX");
		let num_rows =
			tx.execute(UPDATE_VERSION_STMT, &[&next_migration_start]).await.map_err(|e| {
				Error::new(
					ErrorKind::Other,
					format!("Failed to update the version of the schema: {}", e),
				)
			})?;
		assert_eq!(
			num_rows, 1,
			"UPDATE_VERSION_STMT should only update the unique row in the version table"
		);

		tx.commit().await.map_err(|e| {
			Error::new(ErrorKind::Other, format!("Transaction commit error: {}", e))
		})?;

		Ok((migration_start, migrations.len()))
	}

	#[cfg(test)]
	async fn get_schema_version(&self) -> usize {
		let conn = self.pool.get().await.unwrap();
		let row = conn.query_one(GET_VERSION_STMT, &[]).await.unwrap();
		usize::try_from(row.get::<&str, i32>(DB_VERSION_COLUMN)).unwrap()
	}

	#[cfg(test)]
	async fn get_upgrades_list(&self) -> Vec<usize> {
		let conn = self.pool.get().await.unwrap();
		let rows = conn.query(GET_MIGRATION_LOG_STMT, &[]).await.unwrap();
		rows.iter().map(|row| usize::try_from(row.get::<&str, i32>(MIGRATION_LOG_COLUMN)).unwrap()).collect()
	}

	fn build_vss_record(&self, user_token: String, store_id: String, kv: KeyValue) -> VssDbRecord {
		let now = Utc::now();
		VssDbRecord {
			user_token,
			store_id,
			key: kv.key,
			value: kv.value.to_vec(),
			version: kv.version,
			created_at: now,
			last_updated_at: now,
		}
	}

	async fn execute_non_conditional_upsert(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		let stmt = format!("INSERT INTO vss_db (user_token, store_id, key, value, version, created_at, last_updated_at)
                    VALUES ($1, $2, $3, $4, {}, $5, $6)
                    ON CONFLICT (user_token, store_id, key) DO UPDATE
                    SET value = EXCLUDED.value, version = {}, last_updated_at = EXCLUDED.last_updated_at", INITIAL_RECORD_VERSION, INITIAL_RECORD_VERSION);
		let num_rows = transaction
			.execute(
				&stmt,
				&[
					&vss_record.user_token,
					&vss_record.store_id,
					&vss_record.key,
					&vss_record.value,
					&vss_record.created_at,
					&vss_record.last_updated_at,
				],
			)
			.await
			.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Database operation failed. {}", e))
			})?;
		Ok(num_rows)
	}

	async fn execute_conditional_insert(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		let stmt = format!("INSERT INTO vss_db (user_token, store_id, key, value, version, created_at, last_updated_at)
                    VALUES ($1, $2, $3, $4, {}, $5, $6)
                    ON CONFLICT DO NOTHING", INITIAL_RECORD_VERSION);
		let num_rows = transaction
			.execute(
				&stmt,
				&[
					&vss_record.user_token,
					&vss_record.store_id,
					&vss_record.key,
					&vss_record.value,
					&vss_record.created_at,
					&vss_record.last_updated_at,
				],
			)
			.await
			.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Database operation failed. {}", e))
			})?;
		Ok(num_rows)
	}

	async fn execute_conditional_update(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		let stmt = "UPDATE vss_db SET value = $1, version = $2, last_updated_at = $3
                    WHERE user_token = $4 AND store_id = $5 AND key = $6 AND version = $7";
		let num_rows = transaction
			.execute(
				stmt,
				&[
					&vss_record.value,
					&vss_record.version.saturating_add(1),
					&vss_record.last_updated_at,
					&vss_record.user_token,
					&vss_record.store_id,
					&vss_record.key,
					&vss_record.version,
				],
			)
			.await
			.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Database operation failed. {}", e))
			})?;
		Ok(num_rows)
	}

	async fn execute_put_object_query(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		if vss_record.version == -1 {
			self.execute_non_conditional_upsert(transaction, vss_record).await
		} else if vss_record.version == 0 {
			self.execute_conditional_insert(transaction, vss_record).await
		} else {
			self.execute_conditional_update(transaction, vss_record).await
		}
	}

	async fn execute_non_conditional_delete(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		let stmt = "DELETE FROM vss_db WHERE user_token = $1 AND store_id = $2 AND key = $3";
		let num_rows = transaction
			.execute(stmt, &[&vss_record.user_token, &vss_record.store_id, &vss_record.key])
			.await
			.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Database operation failed. {}", e))
			})?;
		Ok(num_rows)
	}

	async fn execute_conditional_delete(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		let stmt = "DELETE FROM vss_db WHERE user_token = $1 AND store_id = $2 AND key = $3 AND version = $4";
		let num_rows = transaction
			.execute(
				stmt,
				&[
					&vss_record.user_token,
					&vss_record.store_id,
					&vss_record.key,
					&vss_record.version,
				],
			)
			.await
			.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Database operation failed. {}", e))
			})?;
		Ok(num_rows)
	}

	async fn execute_delete_object_query(
		&self, transaction: &Transaction<'_>, vss_record: &VssDbRecord,
	) -> io::Result<u64> {
		if vss_record.version == -1 {
			self.execute_non_conditional_delete(transaction, vss_record).await
		} else {
			self.execute_conditional_delete(transaction, vss_record).await
		}
	}
}

#[async_trait]
impl KvStore for PostgresBackendImpl {
	async fn get(
		&self, user_token: String, request: GetObjectRequest,
	) -> Result<GetObjectResponse, VssError> {
		let conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Connection error: {}", e)))?;
		let stmt = "SELECT key, value, version FROM vss_db WHERE user_token = $1 AND store_id = $2 AND key = $3";
		let row = conn
			.query_opt(stmt, &[&user_token, &request.store_id, &request.key])
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Query error: {}", e)))?;

		let key_value = if let Some(row) = row {
			KeyValue {
				key: row.get(KEY_COLUMN),
				value: Bytes::from(row.get::<_, Vec<u8>>(VALUE_COLUMN)),
				version: row.get(VERSION_COLUMN),
			}
		} else if request.key == GLOBAL_VERSION_KEY {
			KeyValue { key: GLOBAL_VERSION_KEY.to_string(), value: Bytes::new(), version: 0 }
		} else {
			return Err(VssError::NoSuchKeyError("Requested key not found.".to_string()));
		};
		Ok(GetObjectResponse { value: Some(key_value) })
	}

	async fn put(
		&self, user_token: String, request: PutObjectRequest,
	) -> Result<PutObjectResponse, VssError> {
		let store_id = request.store_id;
		if request.transaction_items.len() + request.delete_items.len() > MAX_PUT_REQUEST_ITEM_COUNT
		{
			return Err(VssError::InvalidRequestError(format!(
				"Number of write items per request should be less than equal to {}",
				MAX_PUT_REQUEST_ITEM_COUNT
			)));
		}
		let mut vss_put_records: Vec<VssDbRecord> = request
			.transaction_items
			.into_iter()
			.map(|kv| self.build_vss_record(user_token.to_string(), store_id.to_string(), kv))
			.collect();

		let vss_delete_records: Vec<VssDbRecord> = request
			.delete_items
			.into_iter()
			.map(|kv| self.build_vss_record(user_token.to_string(), store_id.to_string(), kv))
			.collect();

		if let Some(global_version) = request.global_version {
			let global_version_record = self.build_vss_record(
				user_token,
				store_id,
				KeyValue {
					key: GLOBAL_VERSION_KEY.to_string(),
					value: Bytes::new(),
					version: global_version,
				},
			);
			vss_put_records.push(global_version_record);
		}

		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Connection error: {}", e)))?;
		let transaction = conn
			.transaction()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Transaction start error: {}", e)))?;

		let mut batch_results = Vec::new();

		for vss_record in &vss_put_records {
			let num_rows = self.execute_put_object_query(&transaction, vss_record).await?;
			batch_results.push(num_rows);
		}

		for vss_record in &vss_delete_records {
			let num_rows = self.execute_delete_object_query(&transaction, vss_record).await?;
			batch_results.push(num_rows);
		}

		for num_rows in batch_results {
			if num_rows == 0 {
				transaction.rollback().await.map_err(|e| {
					Error::new(ErrorKind::Other, format!("Transaction rollback error: {}", e))
				})?;
				return Err(VssError::ConflictError(
					"Transaction could not be completed due to a possible conflict".to_string(),
				));
			}
		}

		transaction.commit().await.map_err(|e| {
			Error::new(ErrorKind::Other, format!("Transaction commit error: {}", e))
		})?;
		Ok(PutObjectResponse {})
	}

	async fn delete(
		&self, user_token: String, request: DeleteObjectRequest,
	) -> Result<DeleteObjectResponse, VssError> {
		let store_id = request.store_id;
		let key_value = request.key_value.ok_or_else(|| {
			VssError::InvalidRequestError("key_value missing in DeleteObjectRequest".to_string())
		})?;
		let vss_record = self.build_vss_record(user_token, store_id, key_value);

		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Connection error: {}", e)))?;
		let transaction = conn
			.transaction()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Transaction start error: {}", e)))?;

		let num_rows = self.execute_delete_object_query(&transaction, &vss_record).await?;

		if num_rows == 0 {
			transaction.rollback().await.map_err(|e| {
				Error::new(ErrorKind::Other, format!("Transaction rollback error: {}", e))
			})?;
			return Ok(DeleteObjectResponse {});
		}

		transaction.commit().await.map_err(|e| {
			Error::new(ErrorKind::Other, format!("Transaction commit error: {}", e))
		})?;
		Ok(DeleteObjectResponse {})
	}

	async fn list_key_versions(
		&self, user_token: String, request: ListKeyVersionsRequest,
	) -> Result<ListKeyVersionsResponse, VssError> {
		let store_id = &request.store_id;
		let key_prefix = &request.key_prefix;
		let page_token = &request.page_token;
		let page_size = request.page_size.unwrap_or(i32::MAX);

		// Only fetch global_version for first page.
		// Fetch global_version before fetching any key_versions to ensure that,
		// all current key_versions were stored at global_version or later.
		let mut global_version = None;
		if page_token.is_none() {
			let get_global_version_request = GetObjectRequest {
				store_id: store_id.to_string(),
				key: GLOBAL_VERSION_KEY.to_string(),
			};
			let get_response = self.get(user_token.clone(), get_global_version_request).await?;
			// unwrap safety: get request always return a value when global_version is queried.
			global_version = Some(get_response.value.unwrap().version);
		}

		let limit = min(page_size, LIST_KEY_VERSIONS_MAX_PAGE_SIZE) as i64;

		let conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Connection error: {}", e)))?;

		let stmt = "SELECT key, version FROM vss_db WHERE user_token = $1 AND store_id = $2 AND key > $3 AND key LIKE $4 ORDER BY key LIMIT $5";

		let key_like = format!("{}%", key_prefix.as_deref().unwrap_or_default());
		let page_token_param = page_token.as_deref().unwrap_or_default();
		let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
			vec![&user_token, &store_id, &page_token_param, &key_like, &limit];

		let rows = conn
			.query(stmt, &params)
			.await
			.map_err(|e| Error::new(ErrorKind::Other, format!("Query error: {}", e)))?;

		let key_versions: Vec<_> = rows
			.iter()
			.filter(|&row| row.get::<&str, &str>(KEY_COLUMN) != GLOBAL_VERSION_KEY)
			.map(|row| KeyValue {
				key: row.get(KEY_COLUMN),
				value: Bytes::new(),
				version: row.get(VERSION_COLUMN),
			})
			.collect();

		let mut next_page_token = Some("".to_string());
		if !key_versions.is_empty() {
			next_page_token = key_versions.get(key_versions.len() - 1).map(|kv| kv.key.to_string());
		}

		Ok(ListKeyVersionsResponse { key_versions, next_page_token, global_version })
	}
}

#[cfg(test)]
mod tests {
	use crate::postgres_store::PostgresBackendImpl;
	use api::define_kv_store_tests;
	use tokio::sync::OnceCell;
	use super::{MIGRATIONS, DUMMY_MIGRATION, drop_database};

	const POSTGRES_ENDPOINT: &str = "postgresql://postgres:postgres@localhost:5432";
	const MIGRATIONS_START: usize = 0;
	const MIGRATIONS_END: usize = MIGRATIONS.len();

	static START: OnceCell<()> = OnceCell::const_new();

	define_kv_store_tests!(PostgresKvStoreTest, PostgresBackendImpl, {
		let db_name = "postgres_kv_store_tests";
		START
			.get_or_init(|| async {
				let _ = drop_database(POSTGRES_ENDPOINT, db_name).await;
				let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
				let (start, end) = store.migrate_vss_database(MIGRATIONS).await.unwrap();
				assert_eq!(start, MIGRATIONS_START);
				assert_eq!(end, MIGRATIONS_END);
			})
			.await;
		let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
		let (start, end) = store.migrate_vss_database(MIGRATIONS).await.unwrap();
		assert_eq!(start, MIGRATIONS_END);
		assert_eq!(end, MIGRATIONS_END);
		assert_eq!(store.get_upgrades_list().await, [MIGRATIONS_START]);
		assert_eq!(store.get_schema_version().await, MIGRATIONS_END);
		store
	});

	#[tokio::test]
	#[should_panic(expected = "We do not allow downgrades")]
	async fn panic_on_downgrade() {
		let db_name = "panic_on_downgrade_test";
		let _ = drop_database(POSTGRES_ENDPOINT, db_name).await;
		{
			let mut migrations = MIGRATIONS.to_vec();
			migrations.push(DUMMY_MIGRATION);
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let (start, end) = store.migrate_vss_database(&migrations).await.unwrap();
			assert_eq!(start, MIGRATIONS_START);
			assert_eq!(end, MIGRATIONS_END + 1);
		};
		{
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let _ = store.migrate_vss_database(MIGRATIONS).await.unwrap();
		};
	}

	#[tokio::test]
	async fn new_migrations_increments_upgrades() {
		let db_name = "new_migrations_increments_upgrades_test";
		let _ = drop_database(POSTGRES_ENDPOINT, db_name).await;
		{
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let (start, end) = store.migrate_vss_database(MIGRATIONS).await.unwrap();
			assert_eq!(start, MIGRATIONS_START);
			assert_eq!(end, MIGRATIONS_END);
			assert_eq!(store.get_upgrades_list().await, [MIGRATIONS_START]);
			assert_eq!(store.get_schema_version().await, MIGRATIONS_END);
		};
		{
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let (start, end) = store.migrate_vss_database(MIGRATIONS).await.unwrap();
			assert_eq!(start, MIGRATIONS_END);
			assert_eq!(end, MIGRATIONS_END);
			assert_eq!(store.get_upgrades_list().await, [MIGRATIONS_START]);
			assert_eq!(store.get_schema_version().await, MIGRATIONS_END);
		};

		let mut migrations = MIGRATIONS.to_vec();
		migrations.push(DUMMY_MIGRATION);
		{
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let (start, end) = store.migrate_vss_database(&migrations).await.unwrap();
			assert_eq!(start, MIGRATIONS_END);
			assert_eq!(end, MIGRATIONS_END + 1);
			assert_eq!(store.get_upgrades_list().await, [MIGRATIONS_START, MIGRATIONS_END]);
			assert_eq!(store.get_schema_version().await, MIGRATIONS_END + 1);
		};

		migrations.push(DUMMY_MIGRATION);
		migrations.push(DUMMY_MIGRATION);
		{
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let (start, end) = store.migrate_vss_database(&migrations).await.unwrap();
			assert_eq!(start, MIGRATIONS_END + 1);
			assert_eq!(end, MIGRATIONS_END + 3);
			assert_eq!(store.get_upgrades_list().await, [MIGRATIONS_START, MIGRATIONS_END, MIGRATIONS_END + 1]);
			assert_eq!(store.get_schema_version().await, MIGRATIONS_END + 3);
		};

		{
			let store = PostgresBackendImpl::new(POSTGRES_ENDPOINT, db_name).await.unwrap();
			let list = store.get_upgrades_list().await;
			assert_eq!(list, [MIGRATIONS_START, MIGRATIONS_END, MIGRATIONS_END + 1]);
			let version = store.get_schema_version().await;
			assert_eq!(version, MIGRATIONS_END + 3);
		}

		drop_database(POSTGRES_ENDPOINT, db_name).await.unwrap();
	}
}
