package org.vss;

import com.google.inject.Guice;
import com.google.inject.Injector;
import jakarta.inject.Inject;
import jakarta.ws.rs.ApplicationPath;
import org.glassfish.hk2.api.ServiceLocator;
import org.glassfish.jersey.server.ResourceConfig;
import org.jvnet.hk2.guice.bridge.api.GuiceBridge;
import org.jvnet.hk2.guice.bridge.api.GuiceIntoHK2Bridge;
import org.vss.guice.BaseModule;

@ApplicationPath("/")
public class VSSApplication extends ResourceConfig {

  @Inject
  public VSSApplication(ServiceLocator serviceLocator) {
    packages("org.vss");
    Injector injector = Guice.createInjector(new BaseModule());
    initGuiceIntoHK2Bridge(serviceLocator, injector);
  }

  // By default, Jersey framework uses HK2 for dependency injection.
  // To use Guice as our dependency injection framework, we provide guice injector to hk2-bridge.
  // So that hk2 can query guice injector for creating/injecting objects.
  private void initGuiceIntoHK2Bridge(ServiceLocator serviceLocator, Injector injector) {
    GuiceBridge.getGuiceBridge().initializeGuiceBridge(serviceLocator);
    GuiceIntoHK2Bridge guiceBridge = serviceLocator.getService(GuiceIntoHK2Bridge.class);
    guiceBridge.bridgeGuiceInjector(injector);
  }
}
