//! Hosts VSS http-server implementation.
//!
//! VSS is an open-source project designed to offer a server-side cloud storage solution specifically
//! tailored for noncustodial Lightning supporting mobile wallets. Its primary objective is to
//! simplify the development process for Lightning wallets by providing a secure means to store
//! and manage the essential state required for Lightning Network (LN) operations.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(missing_docs)]

use tokio::net::TcpListener;
use tokio::signal::unix::SignalKind;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

use crate::vss_service::VssService;
use api::auth::{Authorizer, NoopAuthorizer};
use api::kv_store::KvStore;
use auth_impls::JWTAuthorizer;
use impls::postgres_store::{PostgresPlaintextBackend, PostgresTlsBackend};
use std::sync::Arc;

mod util;
mod vss_service;

fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() != 2 {
		eprintln!("Usage: {} <config-file-path>", args[0]);
		std::process::exit(1);
	}

	let config = util::config::load_configuration(&args[1]).unwrap_or_else(|e| {
		eprintln!("Failed to load configuration: {}", e);
		std::process::exit(-1);
	});

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

		let authorizer: Arc<dyn Authorizer> =
			if let Some(rsa_public_key) = config.jwt_rsa_public_key {
				let jwt_authorizer = JWTAuthorizer::new(rsa_public_key).await;
				println!("Configured JWT authorizer");
				Arc::new(jwt_authorizer)
			} else {
				let noop_authorizer = NoopAuthorizer {};
				println!("No authentication method configured");
				Arc::new(noop_authorizer)
			};

		let store: Arc<dyn KvStore> = if let Some(certificate) = config.tls_config {
			let postgres_tls_backend = PostgresTlsBackend::new(
				&config.postgresql_prefix,
				&config.default_db,
				&config.vss_db,
				certificate,
			)
			.await
			.unwrap_or_else(|e| {
				println!("Failed to start postgres TLS backend: {}", e);
				std::process::exit(-1);
			});
			println!(
				"Connected to PostgreSQL TLS backend with DSN: {}/{}",
				config.postgresql_prefix, config.vss_db
			);
			Arc::new(postgres_tls_backend)
		} else {
			let postgres_plaintext_backend = PostgresPlaintextBackend::new(
				&config.postgresql_prefix,
				&config.default_db,
				&config.vss_db,
			)
			.await
			.unwrap_or_else(|e| {
				println!("Failed to start postgres plaintext backend: {}", e);
				std::process::exit(-1);
			});
			println!(
				"Connected to PostgreSQL plaintext backend with DSN: {}/{}",
				config.postgresql_prefix, config.vss_db
			);
			Arc::new(postgres_plaintext_backend)
		};

		let rest_svc_listener = TcpListener::bind(&config.bind_address).await.unwrap_or_else(|e| {
			println!("Failed to bind listening port: {}", e);
			std::process::exit(-1);
		});
		println!("Listening for incoming connections on {}", config.bind_address);

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
