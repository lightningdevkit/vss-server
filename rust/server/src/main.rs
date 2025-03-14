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
use impls::postgres_store::PostgresBackendImpl;
use std::sync::Arc;

pub(crate) mod util;
pub(crate) mod vss_service;

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
		let authorizer = Arc::new(NoopAuthorizer {});
		let store = Arc::new(
			PostgresBackendImpl::new(&config.postgresql_config.expect("PostgreSQLConfig must be defined in config file.").to_connection_string())
				.await
				.unwrap(),
		);
		let rest_svc_listener =
			TcpListener::bind(&addr).await.expect("Failed to bind listening port");
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
