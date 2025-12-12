//! Hosts VSS http-server implementation.
//!
//! VSS is an open-source project designed to offer a server-side cloud storage solution specifically
//! tailored for noncustodial Lightning supporting mobile wallets. Its primary objective is to
//! simplify the development process for Lightning wallets by providing a secure means to store
//! and manage the essential state required for Lightning Network (LN) operations.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(missing_docs)]

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::signal::unix::SignalKind;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

use crate::vss_service::VssService;
use api::auth::{Authorizer, NoopAuthorizer};
use api::kv_store::KvStore;
use auth_impls::{DecodingKey, JWTAuthorizer};
use impls::postgres_store::{Certificate, PostgresPlaintextBackend, PostgresTlsBackend};
use std::sync::Arc;

mod util;
mod vss_service;

use util::config::{Config, ServerConfig};

fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() != 2 {
		eprintln!("Usage: {} <config-file-path>", args[0]);
		std::process::exit(1);
	}

	let config = match util::config::load_config(&args[1]) {
		Ok(cfg) => cfg,
		Err(e) => {
			eprintln!("Failed to load configuration: {}", e);
			std::process::exit(1);
		},
	};

	let addr: SocketAddr =
		match format!("{}:{}", config.server_config.host, config.server_config.port).parse() {
			Ok(addr) => addr,
			Err(e) => {
				eprintln!("Invalid host/port configuration: {}", e);
				std::process::exit(1);
			},
		};

	let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
		Ok(runtime) => Arc::new(runtime),
		Err(e) => {
			eprintln!("Failed to setup tokio runtime: {}", e);
			std::process::exit(-1);
		},
	};

	runtime.block_on(async {
		let mut sigterm_stream = match tokio::signal::unix::signal(SignalKind::terminate()) {
			Ok(stream) => stream,
			Err(e) => {
				println!("Failed to register for SIGTERM stream: {}", e);
				std::process::exit(-1);
			},
		};

		let authorizer: Arc<dyn Authorizer> = if let Some(file_path) = rsa_pub_file_path {
			let rsa_pub_file = match std::fs::read(file_path) {
				Ok(pem) => pem,
				Err(e) => {
					println!("Failed to read RSA public key file: {}", e);
					std::process::exit(-1);
				},
			};
			let rsa_public_key = match DecodingKey::from_rsa_pem(&rsa_pub_file) {
				Ok(pem) => pem,
				Err(e) => {
					println!("Failed to parse RSA public key file: {}", e);
					std::process::exit(-1);
				},
			};
			Arc::new(JWTAuthorizer::new(rsa_public_key).await)
		} else {
			Arc::new(NoopAuthorizer {})
		};

		let endpoint = postgresql_config.to_postgresql_endpoint();
		let default_db = postgresql_config.default_database;
		let vss_db = postgresql_config.vss_database;
		let store: Arc<dyn KvStore> = if let Some(tls_config) = postgresql_config.tls {
			let addl_certificate = tls_config.ca_file.map(|file| {
				let certificate = match std::fs::read(&file) {
					Ok(cert) => cert,
					Err(e) => {
						println!("Failed to read certificate file: {}", e);
						std::process::exit(-1);
					},
				};
				match Certificate::from_pem(&certificate) {
					Ok(cert) => cert,
					Err(e) => {
						println!("Failed to parse certificate file: {}", e);
						std::process::exit(-1);
					},
				}
			});
			let postgres_tls_backend =
				match PostgresTlsBackend::new(&endpoint, &default_db, &vss_db, addl_certificate)
					.await
				{
					Ok(backend) => backend,
					Err(e) => {
						println!("Failed to start postgres tls backend: {}", e);
						std::process::exit(-1);
					},
				};
			Arc::new(postgres_tls_backend)
		} else {
			let postgres_plaintext_backend =
				match PostgresPlaintextBackend::new(&endpoint, &default_db, &vss_db).await {
					Ok(backend) => backend,
					Err(e) => {
						println!("Failed to start postgres plaintext backend: {}", e);
						std::process::exit(-1);
					},
				};
			Arc::new(postgres_plaintext_backend)
		};
		println!("Connected to PostgreSQL backend with DSN: {}/{}", endpoint, db_name);

		let rest_svc_listener =
			TcpListener::bind(&addr).await.expect("Failed to bind listening port");
		println!("Listening for incoming connections on {}", addr);
		loop {
			tokio::select! {
				res = rest_svc_listener.accept() => {
					match res {
						Ok((stream, _)) => {
							let io_stream = TokioIo::new(stream);
							let vss_service = VssService::new(Arc::clone(&store), Arc::clone(&authorizer));
							runtime.spawn(async move {
								if let Err(err) = http1::Builder::new().serve_connection(io_stream, vss_service).await {
									eprintln!("Failed to serve connection: {}", err);
								}
							});
						},
						Err(e) => eprintln!("Failed to accept connection: {}", e),
					}
				}
				_ = tokio::signal::ctrl_c() => {
					println!("Received CTRL-C, shutting down..");
					break;
				}
				_ = sigterm_stream.recv() => {
					println!("Received SIGTERM, shutting down..");
					break;
				}
			}
		}
	});
}

/// Initializes Sentry error tracking if configured.
///
/// Sentry must be initialized before the tokio runtime starts to ensure proper
/// Hub inheritance for spawned threads. Returns a guard that must be kept alive
/// for the duration of the program to ensure events are flushed on shutdown.
fn initialize_sentry(
	sentry_config: &Option<util::config::SentryConfig>,
) -> Option<sentry::ClientInitGuard> {
	let config = match sentry_config {
		Some(cfg) => cfg,
		None => return None,
	};

	let dsn = match config.get_dsn() {
		Some(dsn) if !dsn.is_empty() => dsn,
		_ => return None,
	};

	let environment = config.get_environment();
	let sample_rate = config.get_sample_rate();

	let guard = sentry::init((
		dsn,
		sentry::ClientOptions {
			release: sentry::release_name!(),
			environment: environment.map(std::borrow::Cow::Owned),
			sample_rate,
			..Default::default()
		},
	));

	if guard.is_enabled() {
		println!(
			"Sentry initialized (environment: {}, sample_rate: {})",
			config.get_environment().unwrap_or_else(|| "default".to_string()),
			sample_rate
		);

		// Send a test message to verify Sentry is configured correctly
		sentry::capture_message("VSS server started - Sentry integration test", sentry::Level::Info);
	}

	Some(guard)
}
