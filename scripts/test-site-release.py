"""Regression tests for the website's published-release download metadata."""

import copy
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "site_release", Path(__file__).with_name("site-release.py")
)
site_release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(site_release)


def release_fixture():
    names = [
        "1537paperstreet_1.2.3_universal.dmg",
        "1537paperstreet_1.2.3_amd64.deb",
        "1537paperstreet_1.2.3_x64-setup.exe",
    ]
    return {
        "tag_name": "v1.2.3",
        "published_at": "2026-10-20T12:00:00Z",
        "draft": False,
        "prerelease": False,
        "assets": [
            {
                "name": name,
                "size": 12_500_000,
                "state": "uploaded",
                "browser_download_url": (
                    "https://github.com/mpakus/1537paperstreet/releases/download/"
                    f"v1.2.3/{name}"
                ),
            }
            for name in names
        ],
    }


class SiteReleaseTests(unittest.TestCase):
    def setUp(self):
        self.page = "\n".join(
            f"<!-- release:{key} -->old<!-- /release:{key} -->"
            for key in ("info", "macos", "linux", "windows")
        )
        self.release = release_fixture()

    def test_published_version_date_and_direct_downloads(self):
        result = site_release.render_release(self.page, self.release)
        self.assertIn("Version 1.2.3", result)
        self.assertIn("October 20, 2026", result)
        self.assertEqual(result.count("12.5 MB"), 3)
        for asset in self.release["assets"]:
            self.assertIn(asset["browser_download_url"], result)
        self.assertNotIn("releases/latest", result)
        self.assertEqual(site_release.render_release(result, self.release), result)

    def test_incomplete_release_cannot_replace_downloads(self):
        for change in ("missing", "uploading", "empty", "duplicate"):
            with self.subTest(change=change):
                release = copy.deepcopy(self.release)
                if change == "missing":
                    release["assets"].pop()
                elif change == "uploading":
                    release["assets"][0]["state"] = "new"
                elif change == "empty":
                    release["assets"][0]["size"] = 0
                else:
                    release["assets"].append(release["assets"][0])
                with self.assertRaises(ValueError):
                    site_release.render_release(self.page, release)

    def test_draft_prerelease_and_invalid_tag_are_rejected(self):
        for key, value in (("draft", True), ("prerelease", True), ("tag_name", 'v1.2.3"><script>')):
            with self.subTest(key=key):
                release = {**self.release, key: value}
                with self.assertRaises(ValueError):
                    site_release.render_release(self.page, release)

    def test_unexpected_download_destination_is_rejected(self):
        self.release["assets"][0]["browser_download_url"] = "https://example.com/installer.dmg"
        with self.assertRaises(ValueError):
            site_release.render_release(self.page, self.release)

    def test_missing_or_duplicate_template_marker_is_rejected(self):
        for page in ("", self.page + "\n" + self.page):
            with self.assertRaises(ValueError):
                site_release.render_release(page, self.release)

    def test_invalid_release_leaves_the_page_unchanged(self):
        self.release["assets"].pop()
        with tempfile.TemporaryDirectory() as directory:
            page = Path(directory) / "index.html"
            release = Path(directory) / "release.json"
            page.write_text(self.page, encoding="utf-8")
            release.write_text(json.dumps(self.release), encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("site-release.py")),
                 str(release), str(page)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 1)
            self.assertIn("Cannot publish download links", result.stderr)
            self.assertEqual(page.read_text(encoding="utf-8"), self.page)


if __name__ == "__main__":
    unittest.main()
