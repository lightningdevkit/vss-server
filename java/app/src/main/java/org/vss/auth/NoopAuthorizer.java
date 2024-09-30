package org.vss.auth;

import jakarta.ws.rs.core.HttpHeaders;
import org.vss.exception.AuthException;

// A no-operation authorizer, that lets any user-request go through.
public class NoopAuthorizer implements Authorizer {
    private static String UNAUTHENTICATED_USER = "unauth-user";

    @Override
    public AuthResponse verify(HttpHeaders headers) throws AuthException {
        return new AuthResponse(UNAUTHENTICATED_USER);
    }
}
