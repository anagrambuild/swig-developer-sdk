# Swig Developer SDK

[Documentation](https://build.onswig.com/developer-sdk) · [Developer Portal](https://dashboard.onswig.com)

Server-side TypeScript, Python, and Rust SDKs for Swig's hosted wallet API. Create an
API key in the Developer Portal, then start with your language's guide:

- [TypeScript](./typescript/README.md): `bun add @swig-wallet/developer-sdk`
- [Python](./python/README.md): `pip install swig-developer-sdk`
- [Rust](./rust/sdk/README.md): `cargo add swig-developer-sdk`

The SDK prepares wallet transactions. Your application signs them, then submits
or sponsors them. Keep the API key on your server.

Each SDK keeps its source, tests, and build output in its own directory:
`typescript/`, `python/`, or `rust/`.

Apache-2.0. See [LICENSE.txt](./LICENSE.txt).
