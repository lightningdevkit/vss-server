# Versioned Storage Service (Rust)

This directory hosts the Rust-based implementation of the Versioned Storage Service (VSS).

### Prerequisites

- Install Rust and Cargo (https://www.rust-lang.org/tools/install).
- Install PostgreSQL 15 (https://www.postgresql.org/download/)
- Install OpenSSL (used for TLS connections to the PostgreSQL backend: https://docs.rs/openssl/latest/openssl/#automatic)

### Building

```
git clone https://github.com/lightningdevkit/vss-server.git
cd vss-server/rust

cargo build --release
```

### Running
1. **Edit Configuration**: Modify `./server/vss-server-config.toml` to set application configuration and
   environment variables as needed.
2. VSS will setup a PostgreSQL database on first launch if it is not found. You can also manually create the database
   using the statement at `./impls/src/postgres/sql/v0_create_vss_db.sql`.
3. Start server:
   ```
   cargo run server/vss-server-config.toml
   ```
4. VSS endpoint should be reachable at `http://localhost:8080/vss`.

### Configuration

Refer to `./server/vss-server-config.toml` to see available configuration options.

### Support

If you encounter any issues or have questions, feel free to open an issue on
the [GitHub repository](https://github.com/lightningdevkit/vss-server/issues). For further assistance or to discuss the
development of VSS, you can reach out to us in the [LDK Discord](https://discord.gg/5AcknnMfBw) in the `#vss` channel.

[LDK Discord]: https://discord.gg/5AcknnMfBw
