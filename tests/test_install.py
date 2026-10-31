import contextlib
import io
import os
import plistlib
from pathlib import Path
import subprocess
import struct
import tempfile
import unittest
from unittest.mock import patch

from scripts import install_plugin as install


def windows_plugin(*, display=b"RGBA Contrast", match=b"HANAYUKI RGBA Contrast", payload=b""):
    """独立した最小PE例。公開入口とPiPL 16000を別の領域に持つ。"""
    image = bytearray(0x1200)
    image[:2] = b"MZ"
    struct.pack_into("<I", image, 0x3c, 0x80)
    image[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<HH", image, 0x84, 0x8664, 1)
    struct.pack_into("<HH", image, 0x94, 0xf0, 0x2000)
    struct.pack_into("<H", image, 0x98, 0x20b)
    struct.pack_into("<II", image, 0x108, 0x1000, 0x80)
    struct.pack_into("<II", image, 0x118, 0x1200, 0x200)
    struct.pack_into("<4I", image, 0x190, 0x1000, 0x1000, 0x1000, 0x200)
    struct.pack_into("<I", image, 0x214, 1)
    struct.pack_into("<I", image, 0x21c, 0x1060)
    struct.pack_into("<I", image, 0x224, 0x1064)
    struct.pack_into("<I", image, 0x260, 0x1100)
    struct.pack_into("<I", image, 0x218, 1)
    struct.pack_into("<I", image, 0x220, 0x1040)
    struct.pack_into("<I", image, 0x240, 0x1050)
    image[0x250:0x25b] = b"EffectMain\0"
    # 資源階層: 名前PiPL -> 番号16000 -> 言語1033 -> データ。
    struct.pack_into("<HHII", image, 0x40c, 1, 0, 0x80000080, 0x80000020)
    struct.pack_into("<HHII", image, 0x42c, 0, 1, 16000, 0x80000040)
    struct.pack_into("<HHII", image, 0x44c, 0, 1, 1033, 0x60)
    image[0x480:0x48a] = b"\x04\0P\0i\0P\0L\0"
    properties = bytearray(b"\x01\x00" + struct.pack("<II", 0, 3))
    for key, value in ((b"eman", bytes([len(display)]) + display),
                       (b"ANMe", bytes([len(match)]) + match),
                       (b"4668", b"EffectMain\0")):
        value += b"\0" * (-len(value) % 4)
        properties += b"MIB8" + key + struct.pack("<II", 0, len(value)) + value
    struct.pack_into("<II", image, 0x460, 0x1300, len(properties))
    image[0x500:0x500 + len(properties)] = properties
    return bytes(image) + payload


class InstallContractTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.artifact = self.root / "build output/release/rgba_contrast.aex"
        self.destination = self.root / "test plugins"
        self.final = self.destination / "rgba_contrast.aex"
        self.previous = windows_plugin(payload=b"previous plugin")
        self.current = windows_plugin(payload=b"current build")
        self.output = io.StringIO()
        self.enterContext(patch.object(install, "ROOT", self.root))
        self.enterContext(patch.object(install.platform, "system", return_value="Windows"))
        self.enterContext(patch.dict(os.environ, {
            "PluginName": "rgba_contrast",
            "CARGO_TARGET_DIR": "build output",
            "AE_PLUGIN_DIR": "test plugins",
        }, clear=True))
        self.enterContext(contextlib.redirect_stdout(self.output))
        self.enterContext(contextlib.redirect_stderr(self.output))
        self.build = self.enterContext(patch.object(install.subprocess, "run"))

    def existing_plugin(self):
        self.destination.mkdir()
        self.final.write_bytes(self.previous)
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
        self.assertIn(str(explicit / "rgba_contrast.aex"), self.output.getvalue())
        self.assertNotIn(str(self.final), self.output.getvalue())
        self.assertFalse(explicit.exists())

    def test_build_failure_preserves_installed_plugin_and_does_not_copy_stale_artifact(self):
        self.existing_plugin()
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(b"stale artifact")
        self.build.side_effect = subprocess.CalledProcessError(7, ["just", "release"])
        self.assertEqual(install.main([]), 7)
        self.assertEqual(self.final.read_bytes(), self.previous)
        self.assertEqual((self.destination / "unrelated.aex").read_bytes(), b"other plugin")

    def test_successful_install_uses_current_build_and_updates_only_its_file(self):
        self.existing_plugin()

        def build_current(*args, **kwargs):
            self.artifact.parent.mkdir(parents=True)
            self.artifact.write_bytes(self.current)

        self.build.side_effect = build_current
        self.assertEqual(install.main([]), 0)
        self.assertEqual(self.final.read_bytes(), self.current)
        self.assertEqual((self.destination / "unrelated.aex").read_bytes(), b"other plugin")
        self.assertEqual(sorted(p.name for p in self.destination.iterdir()), ["rgba_contrast.aex", "unrelated.aex"])

    def test_missing_artifact_preserves_installed_plugin(self):
        self.existing_plugin()
        self.assertEqual(install.main([]), 1)
        self.assertEqual(self.final.read_bytes(), self.previous)

    def test_copy_and_replace_failures_preserve_installed_file(self):
        self.existing_plugin()
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(self.current)
        for operation in ("shutil.copy2", "os.replace"):
            with self.subTest(operation=operation), patch(
                f"scripts.install_plugin.{operation}", side_effect=PermissionError("in use")
            ):
                self.assertEqual(install.main([]), 1)
            self.assertEqual(self.final.read_bytes(), self.previous)
            self.assertEqual(sorted(p.name for p in self.destination.iterdir()), ["rgba_contrast.aex", "unrelated.aex"])

    def test_foreign_or_invalid_source_does_not_create_destination(self):
        variants = [
            b"not a plugin",
            windows_plugin(display=b"Other Effect"),
            windows_plugin(match=b"HANAYUKI RGBA Contrast extra"),
            windows_plugin().replace(b"EffectMain", b"OtherEntry"),
        ]
        wrong_cpu = bytearray(windows_plugin())
        struct.pack_into("<H", wrong_cpu, 0x84, 0xaa64)
        variants.append(bytes(wrong_cpu))
        not_dll = bytearray(windows_plugin())
        struct.pack_into("<H", not_dll, 0x96, 0)
        variants.append(bytes(not_dll))
        self.artifact.parent.mkdir(parents=True)
        for data in variants:
            with self.subTest(data=data[:16]):
                self.artifact.write_bytes(data)
                self.assertEqual(install.main([]), 1)
                self.assertFalse(self.destination.exists())

    def test_foreign_existing_plugin_is_preserved(self):
        self.existing_plugin()
        foreign = windows_plugin(match=b"another product")
        self.final.write_bytes(foreign)
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(self.current)
        self.assertEqual(install.main([]), 1)
        self.assertEqual(self.final.read_bytes(), foreign)
        self.assertEqual(sorted(p.name for p in self.destination.iterdir()),
                         ["rgba_contrast.aex", "unrelated.aex"])

    def test_corrupt_existing_plugin_is_preserved(self):
        self.existing_plugin()
        self.final.write_bytes(b"unidentified existing file")
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(self.current)
        self.assertEqual(install.main([]), 1)
        self.assertEqual(self.final.read_bytes(), b"unidentified existing file")

    def test_source_os_and_public_entry_are_checked_separately_from_product_strings(self):
        image = bytearray(windows_plugin())
        image[0x250:0x25a] = b"OtherEntry"
        self.artifact.parent.mkdir(parents=True)
        self.artifact.write_bytes(image)
        self.assertEqual(install.main([]), 1)
        self.assertFalse(self.destination.exists())

    def test_invalid_destination_fails_before_build(self):
        for destination in ("", "build output/release"):
            with self.subTest(destination=destination):
                self.assertEqual(install.main([destination]), 1)
        self.build.assert_not_called()

    def test_bundle_destination_cannot_be_inside_the_build_artifact(self):
        with patch.object(install.platform, "system", return_value="Darwin"):
            self.assertEqual(install.main(["build output/release/rgba_contrast.plugin/nested"]), 1)
        self.build.assert_not_called()

    def test_macos_install_selects_the_plugin_bundle(self):
        bundle = self.artifact.with_suffix(".plugin")

        def build_bundle(*args, **kwargs):
            bundle.mkdir(parents=True)
            (bundle / "Info.plist").write_bytes(b"bundle metadata")

        self.build.side_effect = build_bundle
        with patch.object(install.platform, "system", return_value="Darwin"), patch.object(install, "inspect_artifact"):
            self.assertEqual(install.main([]), 0)
        self.assertEqual((self.destination / "rgba_contrast.plugin/Info.plist").read_bytes(), b"bundle metadata")
        self.assertFalse(self.final.exists())

    def test_macos_identity_and_each_architecture_entry_are_required(self):
        bundle = self.root / "rgba_contrast.plugin"
        contents = bundle / "Contents"
        (contents / "Resources").mkdir(parents=True)
        info = {"CFBundleIdentifier": "com.hanayuki.AfterEffects.rgba_contrast",
                "CFBundlePackageType": "eFKT"}
        (contents / "Info.plist").write_bytes(plistlib.dumps(info))
        # 古典的な資源フォークの独立例。PiPL 16000を資源マップから参照する。
        properties = bytearray(struct.pack(">II", 0, 4))
        for key, value in ((b"name", b"\x0dRGBA Contrast"),
                           (b"eMNA", b"\x16HANAYUKI RGBA Contrast"),
                           (b"mi64", b"\x0aEffectMain"), (b"ma64", b"\x0aEffectMain")):
            properties += b"8BIM" + key + struct.pack(">II", 0, len(value)) + value
            properties += b"\0" * (-len(value) % 4)
        resource = bytearray(0x350)
        struct.pack_into(">4I", resource, 0, 0x100, 0x300, 4 + len(properties), 0x50)
        struct.pack_into(">I", resource, 0x100, len(properties))
        resource[0x104:0x104 + len(properties)] = properties
        resource[0x300:0x310] = resource[:16]
        struct.pack_into(">H", resource, 0x318, 28)
        struct.pack_into(">H4sHH", resource, 0x31c, 0, b"PiPL", 0, 10)
        struct.pack_into(">hHII", resource, 0x326, 16000, 0xffff, 0, 0)
        resource_path = contents / "Resources/rgba_contrast.rsrc"
        resource_path.write_bytes(resource)

        def apple_tool(command, **kwargs):
            return {"lipo": "x86_64 arm64", "otool": "MH_MAGIC_64 DYLIB",
                    "nm": "00000000 T _EffectMain"}[command[1]]

        with patch.object(install.subprocess, "check_output", side_effect=apple_tool):
            install.inspect_artifact(bundle, "Darwin")
            with patch.object(install.subprocess, "check_output", side_effect=lambda command, **kwargs:
                              "00000000 T _OtherEntry" if command[1] == "nm" and "arm64" in command
                              else apple_tool(command, **kwargs)):
                with self.assertRaises(ValueError):
                    install.inspect_artifact(bundle, "Darwin")
            resource_path.write_bytes(resource.replace(b"HANAYUKI RGBA Contrast", b"Unrelated Product XYZ"))
            with self.assertRaises(ValueError):
                install.inspect_artifact(bundle, "Darwin")
            resource_path.write_bytes(resource)
            info["CFBundleIdentifier"] = "another.product"
            (contents / "Info.plist").write_bytes(plistlib.dumps(info))
            with self.assertRaises(ValueError):
                install.inspect_artifact(bundle, "Darwin")

    def test_bundle_update_replaces_the_whole_bundle(self):
        source = self.root / "output/rgba_contrast.plugin"
        final = self.root / "installed/rgba_contrast.plugin"
        source.mkdir(parents=True)
        final.mkdir(parents=True)
        (source / "new").write_bytes(b"current bundle")
        (final / "obsolete").write_bytes(b"old bundle")
        install.install_artifact(source, final)
        self.assertEqual([p.name for p in final.iterdir()], ["new"])
        self.assertEqual((final / "new").read_bytes(), b"current bundle")
        self.assertEqual([p.name for p in final.parent.iterdir()], ["rgba_contrast.plugin"])

    def test_bundle_replace_failure_restores_previous_bundle(self):
        source = self.root / "output/rgba_contrast.plugin"
        final = self.root / "installed/rgba_contrast.plugin"
        source.mkdir(parents=True)
        final.mkdir(parents=True)
        (source / "new").write_bytes(b"current bundle")
        (final / "old").write_bytes(b"previous bundle")
        replace = os.replace

        def fail_new_bundle(src, dst):
            if Path(src).name == "rgba_contrast.plugin" and Path(dst) == final:
                raise PermissionError("cannot replace")
            return replace(src, dst)

        with patch.object(install.os, "replace", side_effect=fail_new_bundle):
            with self.assertRaises(PermissionError):
                install.install_artifact(source, final)
        self.assertEqual((final / "old").read_bytes(), b"previous bundle")
        self.assertEqual([p.name for p in final.parent.iterdir()], ["rgba_contrast.plugin"])


if __name__ == "__main__":
    unittest.main()
