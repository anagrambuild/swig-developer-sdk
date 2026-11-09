import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

from release_plan import plan_release


class ReleasePlanTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.manifest = Path(self.directory.name) / "Cargo.toml"
        self.manifest.write_text('[package]\nname = "swig-developer-sdk"\nversion = "0.1.0"\n')
        self.commit = "a" * 40

    def published_responses(self, commit, *, yanked=False, checksum=None):
        archive = io.BytesIO()
        source = json.dumps({"git": {"sha1": commit}}).encode()
        with tarfile.open(fileobj=archive, mode="w:gz") as package:
            member = tarfile.TarInfo("swig-developer-sdk-0.1.0/.cargo_vcs_info.json")
            member.size = len(source)
            package.addfile(member, io.BytesIO(source))
        content = archive.getvalue()
        metadata = {
            "version": {
                "crate": "swig-developer-sdk",
                "num": "0.1.0",
                "yanked": yanked,
                "checksum": checksum or hashlib.sha256(content).hexdigest(),
            }
        }
        return [io.BytesIO(json.dumps(metadata).encode()), io.BytesIO(content)]

    @patch("release_plan.urlopen")
    def test_missing_version_publishes_and_creates_release(self, request):
        request.side_effect = HTTPError("https://crates.io", 404, "Not Found", {}, None)
        self.addCleanup(request.side_effect.close)
        self.assertEqual(plan_release(self.manifest, self.commit), ("0.1.0", True, True))

    @patch("release_plan.urlopen")
    def test_existing_version_skips_upload_and_recovers_same_commit_release(self, request):
        request.side_effect = self.published_responses(self.commit)
        self.assertEqual(plan_release(self.manifest, self.commit), ("0.1.0", False, True))

    @patch("release_plan.urlopen")
    def test_existing_version_does_not_tag_a_later_commit(self, request):
        request.side_effect = self.published_responses("b" * 40)
        self.assertEqual(plan_release(self.manifest, self.commit), ("0.1.0", False, False))

    @patch("release_plan.urlopen")
    def test_registry_failure_is_not_an_unpublished_version(self, request):
        for status in (403, 429, 500):
            with self.subTest(status=status):
                request.side_effect = HTTPError("https://crates.io", status, "Failure", {}, None)
                self.addCleanup(request.side_effect.close)
                with self.assertRaises(HTTPError) as failure:
                    plan_release(self.manifest, self.commit)
                self.assertEqual(failure.exception.code, status)

    @patch("release_plan.urlopen")
    def test_invalid_registry_response_stops_publication(self, request):
        request.return_value = io.BytesIO(b"not JSON")
        with self.assertRaises(json.JSONDecodeError):
            plan_release(self.manifest, self.commit)

    @patch("release_plan.urlopen")
    def test_yanked_version_cannot_be_republished(self, request):
        request.side_effect = self.published_responses(self.commit, yanked=True)
        with self.assertRaisesRegex(ValueError, "yanked"):
            plan_release(self.manifest, self.commit)

    @patch("release_plan.urlopen")
    def test_corrupted_archive_cannot_set_release_commit(self, request):
        request.side_effect = self.published_responses(self.commit, checksum="bad")
        with self.assertRaisesRegex(ValueError, "checksum"):
            plan_release(self.manifest, self.commit)

    @patch("release_plan.urlopen")
    def test_invalid_version_stops_before_registry_access(self, request):
        self.manifest.write_text('[package]\nname = "swig-developer-sdk"\nversion = "0.1.0/invalid"\n')
        with self.assertRaisesRegex(ValueError, "stable release version"):
            plan_release(self.manifest, self.commit)
        request.assert_not_called()
