# Versioned Storage Service (Rust)

This directory hosts the Rust-based implementation of the Versioned Storage Service (VSS).

### Prerequisites

- Install Rust and Cargo (https://www.rust-lang.org/tools/install).
- Install PostgreSQL 15 (https://www.postgresql.org/download/)

### Building

```
git clone https://github.com/lightningdevkit/vss-server.git
cd vss-server/rust

cargo build --release
```

### Running
1. **Edit Configuration**: Modify `./server/vss-server-config.toml` to set application configuration and
   environment variables as needed. Add PostgreSQL endpoint configuration.
2. Create table in PostgreSQL using `./impls/src/postgres/sql/`
3. Start server:
   ```
   cargo run -- server/vss-server-config.toml
   ```
4. VSS endpoint should be reachable at `http://localhost:8080/vss`.

### Configuration

Refer to `./server/vss-server-config.toml` to see available configuration options.

### Sentry Integration (Optional)

VSS supports [Sentry](https://sentry.io) for error tracking and monitoring. To enable Sentry:

1. **Via Configuration File**: Add the following to your `vss-server-config.toml`:
   ```toml
   [sentry_config]
   dsn = "https://your-sentry-dsn@sentry.io/project-id"
   environment = "production"  # Optional: e.g., "production", "staging", "development"
   sample_rate = 1.0           # Optional: Value between 0.0 and 1.0 (default: 1.0)
   ```

2. **Via Environment Variables** (recommended for production):
   - `SENTRY_DSN`: Your Sentry DSN
   - `SENTRY_ENVIRONMENT`: Environment name (e.g., "production")
   - `SENTRY_SAMPLE_RATE`: Sample rate for error events (0.0 to 1.0)

   Environment variables take precedence over configuration file values.

If no DSN is provided or the `sentry_config` section is omitted, Sentry will not be initialized.

### Support

If you encounter any issues or have questions, feel free to open an issue on
the [GitHub repository](https://github.com/lightningdevkit/vss-server/issues). For further assistance or to discuss the
development of VSS, you can reach out to us in the [LDK Discord](https://discord.gg/5AcknnMfBw) in the `#vss` channel.

[LDK Discord]: https://discord.gg/5AcknnMfBw
