package org.vss.impl.postgres;

import java.sql.Connection;
import java.sql.DriverManager;
import org.jooq.DSLContext;
import org.jooq.SQLDialect;
import org.jooq.impl.DSL;
import org.junit.jupiter.api.BeforeEach;
import org.testcontainers.containers.PostgreSQLContainer;
import org.testcontainers.junit.jupiter.Container;
import org.testcontainers.junit.jupiter.Testcontainers;
import org.vss.AbstractKVStoreIntegrationTest;

@Testcontainers
public class PostgresBackendImplIntegrationTest extends AbstractKVStoreIntegrationTest {

  private final String POSTGRES_TEST_CONTAINER_DOCKER_IMAGE = "postgres:15";

  @Container
  private final PostgreSQLContainer postgreSQLContainer =
      new PostgreSQLContainer(POSTGRES_TEST_CONTAINER_DOCKER_IMAGE)
          .withDatabaseName("postgres")
          .withUsername("postgres")
          .withPassword("postgres");

  @BeforeEach
  public void initEach() throws Exception {

    // This is required to get postgres driver in classpath before we attempt to fetch a connection
    Class.forName("org.postgresql.Driver");
    Connection conn = DriverManager.getConnection(postgreSQLContainer.getJdbcUrl(),
        postgreSQLContainer.getUsername(), postgreSQLContainer.getPassword());
    DSLContext dslContext = DSL.using(conn, SQLDialect.POSTGRES);

    this.kvStore = new PostgresBackendImpl(dslContext);

    createTable(dslContext);
  }

  private void createTable(DSLContext dslContext) {
    dslContext.execute("CREATE TABLE vss_db ("
        + "store_id character varying(120)  NOT NULL CHECK (store_id <> ''),"
        + "key character varying(120)  NOT NULL,"
        + "value bytea  NULL,"
        + "version bigint  NOT NULL,"
        + "PRIMARY KEY (store_id, key));");
  }
}
