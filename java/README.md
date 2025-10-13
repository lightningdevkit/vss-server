> [!CAUTION]
> The Java-based implementation of VSS-server is now **deprecated**; please use the Rust-based VSS-server implementation instead.

# Versioned Storage Service (Java)

This directory hosts the Java-based implementation of the Versioned Storage Service (VSS).

### Prerequisites

- Install Gradle (https://gradle.org/install/).
- Install JDK 17 (https://docs.aws.amazon.com/corretto/latest/corretto-17-ug/).
- Install Docker (https://docs.docker.com/desktop/).
- Install PostgreSQL 15 (https://www.postgresql.org/download/).


### Installation without docker

see [here](./INSTALL.md)

### Building

```
git clone https://github.com/lightningdevkit/vss-server.git
cd vss-server/java

gradle wrapper --gradle-version 8.1.1
./gradlew build -x test  # Running tests requires docker-engine to be running.
```

* WAR file for deployment should be generated at `./app/build/libs/vss-1.0.war`

#### Only required for development:

* Generating updated [protobuf] objects:
    ```
    ./gradlew generateProto
    ```

* Generating updated [Jooq] objects:
    ```
    ./gradlew generateJooq
    ```

### Running

#### For local testing with a PostgreSQL container:

```
    docker-compose up --build
    
    # To inspect the database:
    `psql postgresql://postgres:YOU_MUST_CHANGE_THIS_PASSWORD@localhost:5432/postgres`
```

#### To run just the VSS-server:

1. **Edit Configuration**: Modify the Dockerfile or `./app/src/main/resources/application.properties` to set application configurations and
   environment variables as needed. Add PostgreSQL endpoint configuration.
2. Create table in PostgreSQL using `./app/src/main/java/org/vss/impl/postgres/sql/`
3. **Build the Docker Image**:
    ```
        docker build . --tag 'vss_server_java'
    ```
3. **Run the Docker Container**:
    ```
        docker run --detach -p 8080:8080 'vss_server_java'
    ```
4. VSS endpoint should be reachable at `http://localhost:8080/vss`.

### Configuration

Refer `./app/src/main/resources/application.properties` to see available configuration options.

Each property can be overridden by setting an environment variable with the same name.
For example, to override `vss.jdbc.url`, set an environment variable `vss.jdbc.url` with the new value.

### Support

If you encounter any issues or have questions, feel free to open an issue on
the [GitHub repository](https://github.com/lightningdevkit/vss-server/issues). For further assistance or to discuss the
development of VSS, you can reach out to us in the [LDK Discord](https://discord.gg/5AcknnMfBw) in the `#vss` channel.

[LDK Discord]: https://discord.gg/5AcknnMfBw

[protobuf]: https://protobuf.dev/

[Jooq]: https://www.jooq.org/
