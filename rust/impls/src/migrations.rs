pub(crate) const DB_VERSION_COLUMN: &str = "db_version";
#[cfg(test)]
pub(crate) const MIGRATION_LOG_COLUMN: &str = "upgrade_from";

pub(crate) const CHECK_DB_STMT: &str = "SELECT 1 FROM pg_database WHERE datname = $1";
pub(crate) const INIT_DB_CMD: &str = "CREATE DATABASE";
#[cfg(test)]
pub(crate) const DROP_DB_CMD: &str = "DROP DATABASE";
pub(crate) const GET_VERSION_STMT: &str = "SELECT db_version FROM vss_db_version;";
pub(crate) const UPDATE_VERSION_STMT: &str = "UPDATE vss_db_version SET db_version=$1;";
pub(crate) const LOG_MIGRATION_STMT: &str = "INSERT INTO vss_db_upgrades VALUES($1);";
#[cfg(test)]
pub(crate) const GET_MIGRATION_LOG_STMT: &str = "SELECT upgrade_from FROM vss_db_upgrades;";

// APPEND-ONLY list of migration statements
//
// Each statement MUST be applied in-order, and only once per database.
//
// We make an exception for the vss_db table creation statement, as users of VSS could have initialized the table
// themselves.
pub(crate) const MIGRATIONS: &[&str] = &[
	"CREATE TABLE vss_db_version (db_version INTEGER);",
	"INSERT INTO vss_db_version VALUES(1);",
	// A write-only log of all the migrations performed on this database, useful for debugging and testing
	"CREATE TABLE vss_db_upgrades (upgrade_from INTEGER);",
	// We do not complain if the table already exists, as users of VSS could have already created this table
	"CREATE TABLE IF NOT EXISTS vss_db (
	    user_token character varying(120) NOT NULL CHECK (user_token <> ''),
	    store_id character varying(120) NOT NULL,
	    key character varying(600) NOT NULL,
	    value bytea NULL,
	    version bigint NOT NULL,
	    created_at TIMESTAMP WITH TIME ZONE,
	    last_updated_at TIMESTAMP WITH TIME ZONE,
	    PRIMARY KEY (user_token, store_id, key)
	);",
	"ALTER TABLE vss_db DROP CONSTRAINT IF EXISTS vss_db_store_id_check;",
	"UPDATE vss_db SET created_at = COALESCE(last_updated_at, NOW()) WHERE created_at IS NULL;",
	"ALTER TABLE vss_db ALTER COLUMN created_at SET NOT NULL;",
	"CREATE INDEX idx_vss_db_created_at ON vss_db (user_token, store_id, created_at, key);",
];
#[cfg(test)]
pub(crate) const DUMMY_MIGRATION: &str = "SELECT 1 WHERE FALSE;";
