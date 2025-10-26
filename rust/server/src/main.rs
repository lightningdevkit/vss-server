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
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::signal::unix::SignalKind;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

use api::auth::{Authorizer, NoopAuthorizer};
use api::kv_store::KvStore;
#[cfg(feature = "jwt")]
use auth_impls::jwt::JWTAuthorizer;
#[cfg(feature = "sigs")]
use auth_impls::signature::SignatureValidatingAuthorizer;
use impls::in_memory_store::InMemoryBackendImpl;
use impls::postgres_store::{Certificate, PostgresPlaintextBackend, PostgresTlsBackend};
use util::config::{Config, ServerConfig, StoreType};
use vss_service::VssService;

mod util;
mod vss_service;

fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() < 2 {
		eprintln!("Usage: {} <config-file-path> [--in-memory]", args[0]);
		std::process::exit(1);
	}

	let config_path = &args[1];
	let use_in_memory = args.contains(&"--in-memory".to_string());

	let mut config = match util::config::load_config(config_path) {
		Ok(cfg) => cfg,
		Err(e) => {
			eprintln!("Failed to load configuration: {}", e);
			std::process::exit(1);
		},
	};

	if use_in_memory {
		config.server_config.store_type = StoreType::InMemory;
	}

	let Config {
		server_config: ServerConfig { host, port, store_type },
		jwt_auth_config,
		postgresql_config,
	} = config;

	let addr: SocketAddr = match format!("{}:{}", host, port).parse() {
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

		let authorizer: Arc<dyn Authorizer> = if use_in_memory {
			Arc::new(NoopAuthorizer {})
		} else {
			let mut authorizer_opt: Option<Arc<dyn Authorizer>> = None;
			#[cfg(feature = "jwt")]
			{
				let rsa_pem_env = match std::env::var("VSS_JWT_RSA_PEM") {
					Ok(env) => Some(env),
					Err(std::env::VarError::NotPresent) => None,
					Err(e) => {
						println!("Failed to load the VSS_JWT_RSA_PEM env var: {}", e);
						std::process::exit(-1);
					},
				};
				let rsa_pem = rsa_pem_env.or(jwt_auth_config.map(|config| config.rsa_pem));
				if let Some(pem) = rsa_pem {
					authorizer_opt = match JWTAuthorizer::new(pem.as_str()).await {
						Ok(auth) => {
							println!("Configured JWT authorizer with RSA public key");
							Some(Arc::new(auth))
						},
						Err(e) => {
							println!("Failed to parse the PEM formatted RSA public key: {}", e);
							std::process::exit(-1);
						},
					};
				}
			}
			#[cfg(feature = "sigs")]
			{
				if authorizer_opt.is_none() {
					println!("Configured signature-validating authorizer");
					authorizer_opt = Some(Arc::new(SignatureValidatingAuthorizer));
				}
			}
			if let Some(auth) = authorizer_opt {
				auth
			} else {
				println!("No authentication method configured, all storage with the same store id will be commingled.");
				Arc::new(NoopAuthorizer {})
			}
		};

		let store: Arc<dyn KvStore> = match store_type {
			StoreType::InMemory => {
				println!("Using in-memory backend for testing");
				Arc::new(InMemoryBackendImpl::new())
			}

			StoreType::Postgres => {
				let postgresql_config = postgresql_config
					.expect("PostgreSQL configuration required for postgres backend");

				let endpoint = postgresql_config.to_postgresql_endpoint();
				let db_name = postgresql_config.database;

				let store: Arc<dyn KvStore> = if let Some(tls_config) = postgresql_config.tls {
					let additional_certificate = tls_config.ca_file.map(|file| {
						let certificate = match std::fs::read(&file) {
							Ok(cert) => cert,
							Err(e) => {
								println!("Failed to read certificate file: {}", e);
								std::process::exit(-1);
							}
						};
						match Certificate::from_pem(&certificate) {
							Ok(cert) => cert,
							Err(e) => {
								println!("Failed to parse certificate file: {}", e);
								std::process::exit(-1);
							}
						}
					});

					let backend = match PostgresTlsBackend::new(&endpoint, &db_name, additional_certificate).await {
						Ok(b) => b,
						Err(e) => {
							println!("Failed to start postgres tls backend: {}", e);
							std::process::exit(-1);
						}
					};

					println!("Connected to PostgreSQL backend with DSN: {}/{}", endpoint, db_name);
					Arc::new(backend)
				} else {
					let backend = match PostgresPlaintextBackend::new(&endpoint, &db_name).await {
						Ok(b) => b,
						Err(e) => {
							println!("Failed to start postgres plaintext backend: {}", e);
							std::process::exit(-1);
						}
					};

					println!("Connected to PostgreSQL backend with DSN: {}/{}", endpoint, db_name);
					Arc::new(backend)
				};

				store
			}
		};

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
