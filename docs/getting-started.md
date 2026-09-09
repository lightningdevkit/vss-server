# Getting Started

This guide covers a local source build and the required PostgreSQL and authentication setup. For
the threat model and auth overview, see the root [README](../README.md).

## Prerequisites

- Rust and Cargo, using at least the repository MSRV of 1.85.0.
- PostgreSQL 15 or newer.
- **(Optional)** OpenSSL development/runtime libraries for JWT authentication and PostgreSQL TLS
  support. These features are enabled by default.

## Quick Start with Docker PostgreSQL

Start only the PostgreSQL service from the included Compose file:

```bash
docker compose up -d postgres
cargo build --release
cargo run -- server/vss-server-config.toml
```

The sample config connects to `postgres:postgres@127.0.0.1:5432`, creates the `vss` database if it
does not exist, and runs schema migrations on startup. The VSS endpoint is
`http://localhost:8080/vss`; `/metrics` is available without VSS authentication:

```bash
curl -f http://localhost:8080/metrics
```

The default `cargo run` command starts the server with signature authentication, because the sample
config does not include a JWT key. VSS API requests still need a valid `Authorization` header. To
exercise VSS API endpoints without auth headers in a local-only environment, use
[Local No-Auth Mode](#local-no-auth-mode).

To run both PostgreSQL and the server in containers:

```bash
docker compose up --build
```

## PostgreSQL Setup

VSS first connects to `default_database`, then creates `vss_database` if it is missing, then connects
to `vss_database` and applies migrations. With the sample config these are `postgres` and `vss`.

The configured PostgreSQL role must be able to connect to `default_database`. It must also either:

- have permission to run `CREATE DATABASE vss` when `vss_database` does not exist, or
- use an already-created `vss_database` and have privileges to create/alter tables, indexes,
  sequences, and rows in that database.

For an existing local PostgreSQL instance, create the database yourself if the VSS user cannot create
databases:

```bash
createdb -U postgres vss
```

Then update `[postgresql_config]` in `server/vss-server-config.toml` or set the corresponding
environment variables:

```bash
VSS_PSQL_USERNAME=postgres
VSS_PSQL_PASSWORD=postgres
VSS_PSQL_ADDRESS=127.0.0.1:5432
VSS_PSQL_DEFAULT_DB=postgres
VSS_PSQL_VSS_DB=vss
```

## Authentication Setup

Default builds include both `jwt` and `sigs` features. At startup, VSS uses JWT auth if an RSA public
key is configured; otherwise it uses signature auth. Both schemes read the HTTP `Authorization`
header.

### Signature Auth

No TOML setting is required. Leave `[jwt_auth_config]` unset and the default build will require each
request to include a proof of private key knowledge in `Authorization`. The proof is the compressed
secp256k1 public key hex, followed by a compact ECDSA signature hex, followed by a Unix timestamp.
The signature covers VSS's signing constant, the public key, and the timestamp. Signature auth
isolates storage by public key, but does not decide who may create a new storage identity;
production deployments still need HTTPS, rate limiting, and an external access control layer.

### JWT Auth

Configure the RSA public key used to verify RS256 JWTs:

```toml
[jwt_auth_config]
rsa_pem = """
-----BEGIN PUBLIC KEY-----
...
-----END PUBLIC KEY-----
"""
```

or set `VSS_JWT_RSA_PEM`. Clients must send `Authorization: Bearer <jwt>`. Tokens must be RS256,
include `sub` and `exp` claims, and omit `aud`; `sub` becomes the VSS storage user token. VSS only
verifies tokens, you must run the service that issues them.

### Optional Features

The following optional cargo features are available, all enabled by default:

- `jwt`: Enables JWT authentication
- `sigs`: Enables Signature authentication
- `postgres-native-tls`: Enables connecting to PostgreSQL via TLS

For example, to build without OpenSSL enable only the `sigs` feature:

```bash
cargo build --release --no-default-features --features sigs
```

### Local No-Auth Mode

For local development only, build with the cfg-gated no-op authorizer:

```bash
RUSTFLAGS="--cfg noop_authorizer" cargo run --no-default-features -- server/vss-server-config.toml
```

Do not expose this mode outside a local test environment.

## Configuration Reference

Default builds read the following settings. Each listed TOML option can be overridden by its
environment variable.

| TOML setting | Environment variable | Purpose |
| --- | --- | --- |
| `server_config.bind_address` | `VSS_BIND_ADDRESS` | HTTP listen address, for example `127.0.0.1:8080`. |
| `server_config.max_request_body_size` | `VSS_MAX_REQUEST_BODY_SIZE` | Request body limit in bytes. Defaults to 1 GiB. |
| `jwt_auth_config.rsa_pem` | `VSS_JWT_RSA_PEM` | RSA public key for JWT verification. Requires the `jwt` feature, which is enabled by default. |
| `postgresql_config.username` | `VSS_PSQL_USERNAME` | PostgreSQL user. |
| `postgresql_config.password` | `VSS_PSQL_PASSWORD` | PostgreSQL password. |
| `postgresql_config.address` | `VSS_PSQL_ADDRESS` | PostgreSQL host and port. |
| `postgresql_config.default_database` | `VSS_PSQL_DEFAULT_DB` | Database used for startup and database creation. |
| `postgresql_config.vss_database` | `VSS_PSQL_VSS_DB` | VSS application database. |
| `postgresql_config.tls` | `VSS_PSQL_TLS` | Enables PostgreSQL TLS with system trust roots. |
| `postgresql_config.tls.crt_pem` | `VSS_PSQL_CRT_PEM` | Adds a PEM root certificate and enables PostgreSQL TLS. |
| `log_config.level` | `VSS_LOG_LEVEL` | Log level. Defaults to `debug`. |
| `log_config.file` | `VSS_LOG_FILE` | Log file path. Defaults to `vss.log`. |

## Production Notes

VSS is stateless and can be run behind a load balancer, but PostgreSQL is the durable state store.
Internet-facing deployments must terminate HTTPS in front of VSS, configure authentication, and add
rate limiting.

## Support

Open issues in the [GitHub repository](https://github.com/lightningdevkit/vss-server/issues), or use
the [LDK Discord](https://discord.gg/5AcknnMfBw) `#vss` channel.

## MSRV

The Minimum Supported Rust Version (MSRV) is 1.85.0.
