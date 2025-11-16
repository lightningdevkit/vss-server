use chrono::Utc;

/// A record stored in the VSS database.
pub struct VssDbRecord {
    /// Token uniquely identifying the user that owns this record.
    pub user_token: String,
    /// Identifier for the store this record belongs to.
    pub store_id: String,
    /// Key under which the value is stored.
    pub key: String,
    /// Stored value as raw bytes.
    pub value: Vec<u8>,
    /// Version number for optimistic concurrency control.
    pub version: i64,
    /// Timestamp when the record was created (UTC).
    pub created_at: chrono::DateTime<Utc>,
    /// Timestamp when the record was last updated (UTC).
    pub last_updated_at: chrono::DateTime<Utc>,
}

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
