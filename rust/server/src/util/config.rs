use auth_impls::DecodingKey;
use impls::postgres_store::Certificate;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct Config {
	server_config: ServerConfig,
	postgresql_config: Option<PostgreSQLConfig>,
}

#[derive(Deserialize)]
struct ServerConfig {
	bind_address: SocketAddr,
	rsa_pub_file_path: Option<String>,
}

// All fields can be overriden by their corresponding environment variables
#[derive(Deserialize)]
struct PostgreSQLConfig {
	// VSS_POSTGRESQL_USERNAME
	username: Option<String>,
	// VSS_POSTGRESQL_PASSWORD
	password: Option<String>,
	// VSS_POSTGRESQL_ADDRESS
	address: Option<SocketAddr>,
	// VSS_POSTGRESQL_DEFAULT_DATABASE
	default_database: Option<String>,
	// VSS_POSTGRESQL_VSS_DATABASE
	vss_database: Option<String>,
	// Set VSS_POSTGRESQL_TLS=1 for tls: Some(TlsConfig { crt_file_path: None })
	// Set VSS_POSTGRESQL_CRT_FILE_PATH=ca.crt for tls: Some(TlsConfig { crt_file_path: String::from("ca.crt") })
	tls: Option<TlsConfig>,
}

#[derive(Deserialize)]
struct TlsConfig {
	crt_file_path: Option<String>,
}

pub(crate) struct Configuration {
	pub(crate) bind_address: SocketAddr,
	pub(crate) jwt_rsa_public_key: Option<DecodingKey>,
	pub(crate) postgresql_prefix: String,
	pub(crate) default_db: String,
	pub(crate) vss_db: String,
	// The Some(None) variant maps to a TLS connection with no additional certificates
	pub(crate) tls_config: Option<Option<Certificate>>,
}

fn load_postgresql_prefix(config: Option<&PostgreSQLConfig>) -> Result<String, String> {
	let username_env = std::env::var("VSS_POSTGRESQL_USERNAME").ok();
	let username = username_env.as_ref()
		.or(config.and_then(|c| c.username.as_ref()))
		.ok_or("PostgreSQL database username must be provided in config or env var VSS_POSTGRESQL_USERNAME must be set.")?;

	let password_env = std::env::var("VSS_POSTGRESQL_PASSWORD").ok();
	let password = password_env.as_ref()
		.or(config.and_then(|c| c.password.as_ref()))
		.ok_or("PostgreSQL database password must be provided in config or env var VSS_POSTGRESQL_PASSWORD must be set.")?;

	let address_env: Option<SocketAddr> =
		if let Some(addr) = std::env::var("VSS_POSTGRESQL_ADDRESS").ok() {
			let socket_addr = addr
				.parse()
				.map_err(|e| format!("Unable to parse postgresql address env var: {}", e))?;
			Some(socket_addr)
		} else {
			None
		};
	let address = address_env.as_ref()
		.or(config.and_then(|c| c.address.as_ref()))
		.ok_or("PostgreSQL service address must be provided in config or env var VSS_POSTGRESQL_ADDRESS must be set.")?;

	Ok(format!("postgresql://{}:{}@{}", username, password, address))
}

pub(crate) fn load_configuration(config_file_path: &str) -> Result<Configuration, String> {
	let config_file = std::fs::read_to_string(config_file_path)
		.map_err(|e| format!("Failed to read configuration file: {}", e))?;
	let Config {
		server_config: ServerConfig { bind_address, rsa_pub_file_path },
		postgresql_config,
	} = toml::from_str(&config_file)
		.map_err(|e| format!("Failed to parse configuration file: {}", e))?;

	let jwt_rsa_public_key = if let Some(file_path) = rsa_pub_file_path {
		let rsa_pub_file = std::fs::read(file_path)
			.map_err(|e| format!("Failed to read RSA public key file: {}", e))?;
		let rsa_public_key = DecodingKey::from_rsa_pem(&rsa_pub_file)
			.map_err(|e| format!("Failed to parse RSA public key file: {}", e))?;
		Some(rsa_public_key)
	} else {
		None
	};

	let postgresql_prefix = load_postgresql_prefix(postgresql_config.as_ref())?;

	let default_db_env = std::env::var("VSS_POSTGRESQL_DEFAULT_DATABASE").ok();
	let default_db = default_db_env
		.or(postgresql_config.as_ref().and_then(|c| c.default_database.clone()))
		.ok_or(String::from("PostgreSQL default database name must be provided in config or env var VSS_POSTGRESQL_DEFAULT_DATABASE must be set."))?;

	let vss_db_env = std::env::var("VSS_POSTGRESQL_VSS_DATABASE").ok();
	let vss_db = vss_db_env
		.or(postgresql_config.as_ref().and_then(|c| c.vss_database.clone()))
		.ok_or(String::from("PostgreSQL vss database name must be provided in config or env var VSS_POSTGRESQL_VSS_DATABASE must be set."))?;

	let crt_file_path_env = std::env::var("VSS_POSTGRESQL_CRT_FILE_PATH").ok();
	let crt_file_path = crt_file_path_env.or(postgresql_config
		.as_ref()
		.and_then(|c| c.tls.as_ref())
		.and_then(|tls| tls.crt_file_path.clone()));
	let certificate = if let Some(file_path) = crt_file_path {
		let crt_file = std::fs::read(&file_path)
			.map_err(|e| format!("Failed to read certificate file: {}", e))?;
		let certificate = Certificate::from_pem(&crt_file)
			.map_err(|e| format!("Failed to parse certificate file: {}", e))?;
		Some(certificate)
	} else {
		None
	};

	let tls_config_env = std::env::var("VSS_POSTGRESQL_TLS").ok();
	let tls_config = (certificate.is_some()
		|| tls_config_env.is_some()
		|| postgresql_config.and_then(|c| c.tls).is_some())
	.then_some(certificate);

	Ok(Configuration {
		bind_address,
		jwt_rsa_public_key,
		postgresql_prefix,
		default_db,
		vss_db,
		tls_config,
	})
}
