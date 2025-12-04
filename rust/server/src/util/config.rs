use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
	pub(crate) server_config: ServerConfig,
	pub(crate) postgresql_config: Option<PostgreSQLConfig>,
}

#[derive(Deserialize)]
pub(crate) struct ServerConfig {
	pub(crate) host: String,
	pub(crate) port: u16,
}

#[derive(Deserialize)]
pub(crate) struct PostgreSQLConfig {
	pub(crate) username: Option<String>, // Optional in TOML, can be overridden by env
	pub(crate) password: Option<String>, // Optional in TOML, can be overridden by env
	pub(crate) host: String,
	pub(crate) port: u16,
	pub(crate) database: String,
	pub(crate) tls: Option<TlsConfig>,
}

#[derive(Deserialize)]
pub(crate) struct TlsConfig {
	pub(crate) ca_file: Option<String>,
}

impl PostgreSQLConfig {
	pub(crate) fn to_postgresql_endpoint(&self) -> String {
		let username_env = std::env::var("VSS_POSTGRESQL_USERNAME");
		let username = username_env.as_ref()
			.ok()
			.or_else(|| self.username.as_ref())
			.expect("PostgreSQL database username must be provided in config or env var VSS_POSTGRESQL_USERNAME must be set.");
		let password_env = std::env::var("VSS_POSTGRESQL_PASSWORD");
		let password = password_env.as_ref()
			.ok()
			.or_else(|| self.password.as_ref())
			.expect("PostgreSQL database password must be provided in config or env var VSS_POSTGRESQL_PASSWORD must be set.");

		format!("postgresql://{}:{}@{}:{}", username, password, self.host, self.port)
	}
}

pub(crate) fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
	let config_str = std::fs::read_to_string(config_path)?;
	let config: Config = toml::from_str(&config_str)?;
	Ok(config)
}
