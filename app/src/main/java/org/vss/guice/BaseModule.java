package org.vss.guice;

import com.google.inject.AbstractModule;
import com.google.inject.Singleton;
import org.vss.KVStore;
import org.vss.impl.postgres.PostgresBackendImpl;

public class BaseModule extends AbstractModule {
  @Override
  protected void configure() {
    bind(KVStore.class).to(PostgresBackendImpl.class).in(Singleton.class);
  }
}
