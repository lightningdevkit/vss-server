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
        .getResourceAsStream("application.properties")) {
      Properties applicationProperties = new Properties();
      applicationProperties.load(input);

      config.setJdbcUrl(getEnvOrConfigProperty("vss.jdbc.url", applicationProperties));
      config.setUsername(getEnvOrConfigProperty("vss.jdbc.username", applicationProperties));
      config.setPassword(getEnvOrConfigProperty("vss.jdbc.password", applicationProperties));

      config.setMaximumPoolSize(
          Integer.parseInt(getEnvOrConfigProperty("vss.hikaricp.maxPoolSize", applicationProperties)));
      config.setMinimumIdle(
          Integer.parseInt(getEnvOrConfigProperty("vss.hikaricp.minimumIdle", applicationProperties)));
      config.setConnectionTimeout(
          Long.parseLong(getEnvOrConfigProperty("vss.hikaricp.connectionTimeout", applicationProperties)));
      config.setIdleTimeout(
          Long.parseLong(getEnvOrConfigProperty("vss.hikaricp.idleTimeout", applicationProperties)));
      config.setMaxLifetime(
          Long.parseLong(getEnvOrConfigProperty("vss.hikaricp.maxLifetime", applicationProperties)));

      config.addDataSourceProperty("cachePrepStmts",
          getEnvOrConfigProperty("vss.hikaricp.cachePrepStmts", applicationProperties));
      config.addDataSourceProperty("prepStmtCacheSize",
          getEnvOrConfigProperty("vss.hikaricp.prepStmtCacheSize", applicationProperties));
      config.addDataSourceProperty("prepStmtCacheSqlLimit",
          getEnvOrConfigProperty("vss.hikaricp.prepStmtCacheSqlLimit", applicationProperties));

      dataSource = new HikariDataSource(config);
    } catch (IOException e) {
      throw new RuntimeException("Unable to read application.properties from resources");
    }
  }

  // Retrieves the value of a specified property, first checking environment variables,
  // then falling back to provided configuration properties if the environment variable is not set.
  private static String getEnvOrConfigProperty(String key, Properties hikariJdbcProperties) {
    String propertyValue = System.getenv(key);
    if (StringUtils.isBlank(propertyValue)) {
      propertyValue = hikariJdbcProperties.getProperty(key);
    }
    return propertyValue;
  }

  private HikariCPDataSource() {
  }
}
