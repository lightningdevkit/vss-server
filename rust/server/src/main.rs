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
use impls::in_memory_store::InMemoryBackendImpl;
use impls::postgres_store::PostgresBackendImpl;
use std::sync::Arc;

pub(crate) mod util;
pub(crate) mod vss_service;

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

	// Override the `store_type` if --in-memory flag passed
	if use_in_memory {
		println!("Overriding backend type: using in-memory backend (via --in-memory flag)");
		config.server_config.store_type = "in_memory".to_string();
	}

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
		let authorizer = Arc::new(NoopAuthorizer {});
		let store: Arc<dyn KvStore> = match config.server_config.store_type.as_str() {
            "postgres" => {
                let pg_config = config.postgresql_config
                    .expect("PostgreSQL configuration required for postgres backend");
                let endpoint = pg_config.to_postgresql_endpoint();
                let db_name = pg_config.database;
                match PostgresBackendImpl::new(&endpoint, &db_name).await {
                    Ok(backend) => {
                        println!("Connected to PostgreSQL backend with DSN: {}/{}", endpoint, db_name);
                        Arc::new(backend)
                    },
                    Err(e) => {
                        eprintln!("Failed to connect to PostgreSQL backend: {}", e);
                        std::process::exit(1);
                    },
                }
            },
            "in_memory" => {
                println!("Using in-memory backend for testing");
                Arc::new(InMemoryBackendImpl::new())
            },
            _ => {
                eprintln!("Invalid backend_type: {}. Must be 'postgres' or 'in_memory'", config.server_config.store_type);
                std::process::exit(1);
            },
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
							let vss_service = VssService::new(Arc::clone(&store) as Arc<dyn KvStore>, Arc::clone(&authorizer) as Arc<dyn Authorizer>);
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
