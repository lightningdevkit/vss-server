package org.vss.guice;

import com.google.inject.AbstractModule;
import com.google.inject.Provides;
import com.google.inject.Singleton;
import com.zaxxer.hikari.HikariConfig;
import com.zaxxer.hikari.HikariDataSource;
import java.io.IOException;
import java.io.InputStream;
import java.util.Properties;
import org.jooq.DSLContext;
import org.jooq.SQLDialect;
import org.jooq.impl.DSL;
import org.jooq.tools.StringUtils;
import org.vss.KVStore;
import org.vss.auth.Authorizer;
import org.vss.auth.NoopAuthorizer;
import org.vss.impl.postgres.PostgresBackendImpl;

public class BaseModule extends AbstractModule {

  @Override
  protected void configure() {
    // Provide PostgresBackend as default implementation for KVStore.
    bind(KVStore.class).to(PostgresBackendImpl.class).in(Singleton.class);

    // Default to Noop Authorizer.
    bind(Authorizer.class).to(NoopAuthorizer.class).in(Singleton.class);
  }

  @Provides
  @Singleton
  // Provide DSLContext which is to be used by PostgresBackend
  public DSLContext provideDSLContext() throws ClassNotFoundException {
    // Required to load postgres drivers in tomcat
    Class.forName("org.postgresql.Driver");
    return DSL.using(HikariCPDataSource.dataSource, SQLDialect.POSTGRES);
  }
}

// Provide Hikari Connection Pooling configuration for jdbc connection management.
// Hikari is high-performance connection pooling library which will maintain a set of connections
// to the database for us.
// When we provide `HikariCPDataSource` to DSLContext, jOOQ will internally `acquire` and `release`
// connections from pool.
// For HikariCP config, we provide some sane defaults, but they are meant to be changed and tuned.
// For specific parameter functionality, refer to HikariCP docs.
class HikariCPDataSource {

  private static HikariConfig config = new HikariConfig();
  public static HikariDataSource dataSource;

  static {
    try (InputStream input = HikariCPDataSource.class.getClassLoader()
        .getResourceAsStream("hikariJdbc.properties")) {
      Properties hikariJdbcProperties = new Properties();
      hikariJdbcProperties.load(input);

      config.setJdbcUrl(hikariJdbcProperties.getProperty("jdbc.url"));
      config.setUsername(hikariJdbcProperties.getProperty("jdbc.username"));
      config.setPassword(hikariJdbcProperties.getProperty("jdbc.password"));

      config.setMaximumPoolSize(
          Integer.parseInt(hikariJdbcProperties.getProperty("hikaricp.maxPoolSize")));
      config.setMinimumIdle(
          Integer.parseInt(hikariJdbcProperties.getProperty("hikaricp.minimumIdle")));
      config.setConnectionTimeout(
          Long.parseLong(hikariJdbcProperties.getProperty("hikaricp.connectionTimeout")));
      config.setIdleTimeout(
          Long.parseLong(hikariJdbcProperties.getProperty("hikaricp.idleTimeout")));
      config.setMaxLifetime(
          Long.parseLong(hikariJdbcProperties.getProperty("hikaricp.maxLifetime")));

      config.addDataSourceProperty("cachePrepStmts",
          hikariJdbcProperties.getProperty("hikaricp.cachePrepStmts"));
      config.addDataSourceProperty("prepStmtCacheSize",
          hikariJdbcProperties.getProperty("hikaricp.prepStmtCacheSize"));
      config.addDataSourceProperty("prepStmtCacheSqlLimit",
          hikariJdbcProperties.getProperty("hikaricp.prepStmtCacheSqlLimit"));

      dataSource = new HikariDataSource(config);
    } catch (IOException e) {
      throw new RuntimeException("Unable to read hikariJdbcProperties from resources");
    }
  }

  private HikariCPDataSource() {
  }
}
