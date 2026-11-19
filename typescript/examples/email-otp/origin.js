export function appOrigin(value, port = process.env.PORT || '4181') {
  value ??= 'http://127.0.0.1:' + port;
  const url = new URL(value);
  if (
    !/^(?:https:\/\/(?:[a-z0-9.-]+|\[::1\])|http:\/\/(?:localhost|127\.0\.0\.1))(?::\d{1,5})?\/?$/i.test(
      value,
    ) ||
    url.port === '0'
  ) {
    throw new Error(
      'SWIG_APP_ORIGIN must be an HTTPS origin or an HTTP origin on localhost or 127.0.0.1, without a path, query, or credentials.',
    );
  }
  return url;
}

export function isLoopback(host) {
  return host === 'localhost' || host === '127.0.0.1' || host === '[::1]';
}
