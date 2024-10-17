package org.vss.auth;

import jakarta.ws.rs.core.HttpHeaders;
import org.vss.exception.AuthException;

// Interface for authorizer that is run before every request.
public interface Authorizer {
    AuthResponse verify(HttpHeaders headers) throws AuthException;
}
