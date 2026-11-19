export function appOrigin(value = 'https://127.0.0.1:4181') {
  const url = new URL(value);
  if (
    !/^https:\/\/(?:[a-z0-9.-]+|\[::1\])(?::\d{1,5})?\/?$/i.test(value) ||
    url.port === '0'
  ) {
    throw new Error(
      'SWIG_APP_ORIGIN must be an HTTPS origin such as https://127.0.0.1:4181, without a path, query, or credentials.',
    );
  }
  return url;
}

export function isLoopback(host) {
  return host === 'localhost' || host === '127.0.0.1' || host === '[::1]';
}
