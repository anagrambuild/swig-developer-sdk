import hashlib
import io
import json
import os
from pathlib import Path
import re
import tarfile
import tomllib
from urllib.error import HTTPError
from urllib.request import Request, urlopen


def plan_release(manifest: Path, commit: str) -> tuple[str, bool, bool]:
    package = tomllib.loads(manifest.read_text())["package"]
    name = package["name"]
    version = package["version"]
    if name != "swig-developer-sdk" or not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("Expected the SDK package with a stable release version")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("Expected the full release commit SHA")

    request = Request(
        f"https://crates.io/api/v1/crates/{name}/{version}",
        headers={"User-Agent": "swig-developer-sdk-release"},
    )
    try:
        with urlopen(request, timeout=30) as response:
            published = json.load(response)["version"]
    except HTTPError as error:
        if error.code == 404:
            return version, True, True
        raise

    if published["crate"] != name or published["num"] != version:
        raise ValueError("Registry returned a different package version")
    if published["yanked"]:
        raise ValueError("The current package version has been yanked; bump the version")

    # Recover a failed GitHub release only for the commit actually published.
    archive_request = Request(
        f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
        headers={"User-Agent": "swig-developer-sdk-release"},
    )
    with urlopen(archive_request, timeout=30) as response:
        archive = response.read()
    if hashlib.sha256(archive).hexdigest() != published["checksum"]:
        raise ValueError("Published package checksum does not match the registry")
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as package_archive:
        vcs_file = package_archive.extractfile(f"{name}-{version}/.cargo_vcs_info.json")
        if vcs_file is None:
            raise ValueError("Published package has no source commit")
        with vcs_file:
            source = json.load(vcs_file)["git"]
    if source.get("dirty", False):
        raise ValueError("Published package was built from a dirty checkout")
    return version, False, source["sha1"] == commit


if __name__ == "__main__":
    version, publish, release = plan_release(Path("sdk/Cargo.toml"), os.environ["GITHUB_SHA"])
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"version={version}\npublish={str(publish).lower()}\nrelease={str(release).lower()}\n")
    print(f"swig-developer-sdk {version}: publish={publish}, github_release={release}")
