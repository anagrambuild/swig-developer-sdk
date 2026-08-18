# Releasing the Developer SDK

TypeScript and Python use independent, version-qualified tags from the same
repository. Release tags must point to commits already merged into `main`.

## Python

1. Set the version in `python/pyproject.toml` and refresh `python/uv.lock`.
2. Merge the release commit into `main`.
3. Tag that commit as `python-v<version>` and push the tag.

`publish-python.yml` runs formatting, linting, type checks, tests, and a package
build before publishing through PyPI trusted publishing. The trusted publisher
identity is:

- repository: `anagrambuild/swig-developer-sdk`
- workflow: `publish-python.yml`
- environment: `pypi`

PyPI currently allows any GitHub environment for this identity, so the more
restrictive workflow environment above is accepted. Remove the old
`anagrambuild/swig-ts` publisher only after the first standalone release has
published successfully.

## TypeScript

1. Set the version in `typescript/package.json`.
2. Merge the release commit into `main`.
3. Tag that commit as `typescript-v<version>` and push the tag.

`publish-typescript.yml` runs the repository checks and build before publishing
`@swig-wallet/developer-sdk` with npm provenance. It currently authenticates
with the repository `NPM_TOKEN` secret.
