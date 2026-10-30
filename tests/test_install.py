import contextlib
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import install_plugin as install


class InstallContractTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.artifact = self.root / "build output/release/sample.aex"
        self.destination = self.root / "test plugins"
        self.final = self.destination / "sample.aex"
        self.output = io.StringIO()
        self.enterContext(patch.object(install, "ROOT", self.root))
        self.enterContext(patch.object(install.platform, "system", return_value="Windows"))
        self.enterContext(patch.dict(os.environ, {
            "PluginName": "sample",
            "CARGO_TARGET_DIR": "build output",
            "AE_PLUGIN_DIR": "test plugins",
        }, clear=True))
        self.enterContext(contextlib.redirect_stdout(self.output))
        self.enterContext(contextlib.redirect_stderr(self.output))
        self.build = self.enterContext(patch.object(install.subprocess, "run"))

    def existing_plugin(self):
        self.destination.mkdir()
        self.final.write_bytes(b"previous plugin")
        (self.destination / "unrelated.aex").write_bytes(b"other plugin")

    def test_plan_resolves_relative_paths_without_building_or_writing(self):
        self.assertEqual(install.main(["--dry-run"]), 0)
        self.assertIn(str(self.artifact), self.output.getvalue())
        self.assertIn(str(self.final), self.output.getvalue())
        self.build.assert_not_called()
        self.assertEqual(list(self.root.iterdir()), [])

    def test_explicit_destination_overrides_environment(self):
        explicit = self.root / "owner's plugins $example"
        self.assertEqual(install.main([str(explicit), "--dry-run"]), 0)
        self.assertIn(str(explicit / "sample.aex"), self.output.getvalue())
        self.assertNotIn(str(self.final), self.output.getvalue())
        self.assertFalse(explicit.exists())

    def test_build_failure_preserves_installed_plugin_and_does_not_copy_stale_artifact(self):
        self.existing_plugin()
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(b"stale artifact")
        self.build.side_effect = subprocess.CalledProcessError(7, ["just", "release"])
        self.assertEqual(install.main([]), 7)
        self.assertEqual(self.final.read_bytes(), b"previous plugin")
        self.assertEqual((self.destination / "unrelated.aex").read_bytes(), b"other plugin")

    def test_successful_install_uses_current_build_and_updates_only_its_file(self):
        self.existing_plugin()

        def build_current(*args, **kwargs):
            self.artifact.parent.mkdir(parents=True)
            self.artifact.write_bytes(b"current build")

        self.build.side_effect = build_current
        self.assertEqual(install.main([]), 0)
        self.assertEqual(self.final.read_bytes(), b"current build")
        self.assertEqual((self.destination / "unrelated.aex").read_bytes(), b"other plugin")
        self.assertEqual(sorted(p.name for p in self.destination.iterdir()), ["sample.aex", "unrelated.aex"])

    def test_missing_artifact_preserves_installed_plugin(self):
        self.existing_plugin()
        self.assertEqual(install.main([]), 1)
        self.assertEqual(self.final.read_bytes(), b"previous plugin")

    def test_copy_and_replace_failures_preserve_installed_file(self):
        self.existing_plugin()
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(b"new build")
        for operation in ("shutil.copy2", "os.replace"):
            with self.subTest(operation=operation), patch(
                f"scripts.install_plugin.{operation}", side_effect=PermissionError("in use")
            ):
                self.assertEqual(install.main([]), 1)
            self.assertEqual(self.final.read_bytes(), b"previous plugin")
            self.assertEqual(sorted(p.name for p in self.destination.iterdir()), ["sample.aex", "unrelated.aex"])

    def test_invalid_destination_fails_before_build(self):
        for destination in ("", "build output/release"):
            with self.subTest(destination=destination):
                self.assertEqual(install.main([destination]), 1)
        self.build.assert_not_called()

    def test_bundle_destination_cannot_be_inside_the_build_artifact(self):
        with patch.object(install.platform, "system", return_value="Darwin"):
            self.assertEqual(install.main(["build output/release/sample.plugin/nested"]), 1)
        self.build.assert_not_called()

    def test_macos_install_selects_the_plugin_bundle(self):
        bundle = self.artifact.with_suffix(".plugin")

        def build_bundle(*args, **kwargs):
            bundle.mkdir(parents=True)
            (bundle / "Info.plist").write_bytes(b"bundle metadata")

        self.build.side_effect = build_bundle
        with patch.object(install.platform, "system", return_value="Darwin"):
            self.assertEqual(install.main([]), 0)
        self.assertEqual((self.destination / "sample.plugin/Info.plist").read_bytes(), b"bundle metadata")
        self.assertFalse(self.final.exists())

    def test_bundle_update_replaces_the_whole_bundle(self):
        source = self.root / "output/sample.plugin"
        final = self.root / "installed/sample.plugin"
        source.mkdir(parents=True)
        final.mkdir(parents=True)
        (source / "new").write_bytes(b"current bundle")
        (final / "obsolete").write_bytes(b"old bundle")
        install.install_artifact(source, final)
        self.assertEqual([p.name for p in final.iterdir()], ["new"])
        self.assertEqual((final / "new").read_bytes(), b"current bundle")
        self.assertEqual([p.name for p in final.parent.iterdir()], ["sample.plugin"])

    def test_bundle_replace_failure_restores_previous_bundle(self):
        source = self.root / "output/sample.plugin"
        final = self.root / "installed/sample.plugin"
        source.mkdir(parents=True)
        final.mkdir(parents=True)
        (source / "new").write_bytes(b"current bundle")
        (final / "old").write_bytes(b"previous bundle")
        replace = os.replace

        def fail_new_bundle(src, dst):
            if Path(src).name == "sample.plugin" and Path(dst) == final:
                raise PermissionError("cannot replace")
            return replace(src, dst)

        with patch.object(install.os, "replace", side_effect=fail_new_bundle):
            with self.assertRaises(PermissionError):
                install.install_artifact(source, final)
        self.assertEqual((final / "old").read_bytes(), b"previous bundle")
        self.assertEqual([p.name for p in final.parent.iterdir()], ["sample.plugin"])


if __name__ == "__main__":
    unittest.main()
