package org.vss.exception;

public class NoSuchKeyException extends RuntimeException {
  public NoSuchKeyException(String message) {
    super(message);
  }
}
