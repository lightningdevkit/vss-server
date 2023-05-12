package org.vss;

import jakarta.ws.rs.ApplicationPath;
import org.glassfish.jersey.server.ResourceConfig;

@ApplicationPath("/")
public class VSSApplication extends ResourceConfig {
  public VSSApplication() {
  }
}
