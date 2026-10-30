import copy
import unittest

from scripts.validate_release import validate_release


class ReleaseContractTests(unittest.TestCase):
    def setUp(self):
        self.event = {
            "action": "closed",
            "repository": {"full_name": "owner/project"},
            "pull_request": {
                "merged": True,
                "base": {"ref": "main"},
                "head": {"ref": "release/v1.1.0", "repo": {"full_name": "owner/project"}},
                "merge_commit_sha": "a" * 40,
            },
        }
        self.files = [[{"filename": "Cargo.toml"}, {"filename": "Cargo.lock"}]]

    def test_merged_version_update_is_accepted(self):
        self.assertEqual(validate_release(self.event, "1.1.0", self.files, "1.0.0"), "1.1.0")

    def test_prerelease_is_accepted(self):
        self.event["pull_request"]["head"]["ref"] = "release/v1.1.0-rc.1"
        self.assertEqual(validate_release(self.event, "1.1.0-rc.1", self.files, "1.0.0"), "1.1.0-rc.1")

    def test_non_release_or_unmerged_pr_is_rejected(self):
        for field, value in [("merged", False), ("base", {"ref": "develop"}), ("merge_commit_sha", "")]:
            event = copy.deepcopy(self.event)
            event["pull_request"][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_release(event, "1.1.0", self.files, "1.0.0")
        self.event["pull_request"]["head"]["ref"] = "chore/update"
        with self.assertRaises(ValueError):
            validate_release(self.event, "1.1.0", self.files, "1.0.0")

    def test_mismatched_version_unchanged_version_or_extra_files_are_rejected(self):
        for version, files, previous in [("1.2.0", self.files, "1.0.0"), ("1.1.0", self.files, "1.1.0"), ("1.1.0", [[{"filename": "Cargo.toml"}, {"filename": "src/lib.rs"}]], "1.0.0"), ("1.1.0", [[{"filename": "Cargo.lock"}]], "1.0.0")]:
            with self.subTest(version=version, files=files, previous=previous), self.assertRaises(ValueError):
                validate_release(self.event, version, files, previous)

    def test_foreign_repository_is_rejected(self):
        self.event["pull_request"]["head"]["repo"]["full_name"] = "other/project"
        with self.assertRaises(ValueError):
            validate_release(self.event, "1.1.0", self.files, "1.0.0")


if __name__ == "__main__":
    unittest.main()
