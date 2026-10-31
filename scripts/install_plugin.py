"""最適化ビルド後にプラグインを配置する。予定表示では書き込まない。"""

import argparse
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import struct
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def validate_pipl(data, *, windows):
    """固定版piplの属性境界を読み、名前や入口を前方一致で受け入れない。"""
    order = "<" if windows else ">"
    position = 2 if windows else 0
    version, count = struct.unpack_from(order + "II", data, position)
    if version != 0 or count > 1000:
        raise ValueError("PiPL形式が不正です")
    position += 8
    properties = {}
    for _ in range(count):
        vendor, key, ident, size = struct.unpack_from(order + "4s4sII", data, position)
        position += 16
        if windows:
            vendor, key = vendor[::-1], key[::-1]
        if vendor != b"8BIM" or ident != 0 or key in properties or position + size > len(data):
            raise ValueError("PiPL属性が不正です")
        properties[key] = data[position:position + size]
        position += size if windows else (size + 3) & ~3
    if position != len(data):
        raise ValueError("PiPL属性長が一致しません")
    expected = {
        b"name": b"RGBA Contrast", b"eMNA": b"HANAYUKI RGBA Contrast",
    }
    for key, value in expected.items():
        value = bytes([len(value)]) + value
        if windows:
            value += b"\0" * (-len(value) % 4)
        if properties.get(key) != value:
            raise ValueError("PiPLの製品識別が一致しません")
    for key in ((b"8664",) if windows else (b"mi64", b"ma64")):
        value = b"EffectMain\0\0" if windows else b"\x0aEffectMain"
        if properties.get(key) != value:
            raise ValueError("PiPLの公開入口が一致しません")


def inspect_windows(data):
    """PEのCPU・DLL形式・実エクスポートとPiPL 16000を検査する。"""
    def u16(offset):
        return struct.unpack_from("<H", data, offset)[0]

    def u32(offset):
        return struct.unpack_from("<I", data, offset)[0]

    pe = u32(0x3c)
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0" or u16(pe + 4) != 0x8664:
        raise ValueError("Windows x86_64のPEではありません")
    count, opt_size = u16(pe + 6), u16(pe + 20)
    opt = pe + 24
    if u16(opt) != 0x20b or not u16(pe + 22) & 0x2000:
        raise ValueError("64ビットDLLではありません")

    def offset(rva):
        for index in range(count):
            section = opt + opt_size + 40 * index
            size, start, raw_size, raw = struct.unpack_from("<4I", data, section + 8)
            if start <= rva < start + max(size, raw_size):
                delta = rva - start
                if delta < raw_size and raw + delta < len(data):
                    return raw + delta
        raise ValueError("PEの領域が不正です")

    export = offset(u32(opt + 112))
    names = u32(export + 24)
    if names > 10000:
        raise ValueError("エクスポート数が不正です")
    name_table = offset(u32(export + 32))
    entry_found = False
    for index in range(names):
        start = offset(u32(name_table + 4 * index))
        end = data.find(b"\0", start, start + 256)
        if end < 0:
            raise ValueError("公開入口名が不正です")
        if data[start:end] == b"EffectMain":
            ordinal = u16(offset(u32(export + 36)) + 2 * index)
            if ordinal >= u32(export + 20):
                raise ValueError("公開入口の序数が不正です")
            function = u32(offset(u32(export + 28)) + 4 * ordinal)
            if u32(opt + 112) <= function < u32(opt + 112) + u32(opt + 116):
                raise ValueError("EffectMainが別DLLへの転送になっています")
            offset(function)
            entry_found = True
    if not entry_found:
        raise ValueError("EffectMainが公開されていません")

    resource = offset(u32(opt + 128))

    def entries(directory):
        named, ids = struct.unpack_from("<HH", data, resource + directory + 12)
        return [struct.unpack_from("<II", data, resource + directory + 16 + 8 * index)
                for index in range(named + ids)]

    types = []
    for name, child in entries(0):
        if name & 0x80000000:
            start = resource + (name & 0x7fffffff)
            length = u16(start)
            if data[start + 2:start + 2 + length * 2].decode("utf-16-le").upper() == "PIPL":
                types.append(child)
    if len(types) != 1 or not types[0] & 0x80000000:
        raise ValueError("PiPL資源がありません")
    resources = [child for ident, child in entries(types[0] & 0x7fffffff) if ident == 16000]
    if len(resources) != 1 or not resources[0] & 0x80000000:
        raise ValueError("PiPL 16000がありません")
    languages = entries(resources[0] & 0x7fffffff)
    if not languages:
        raise ValueError("PiPLデータがありません")
    for _, child in languages:
        if child & 0x80000000:
            raise ValueError("PiPL資源の階層が不正です")
        rva, size = struct.unpack_from("<II", data, resource + child)
        start = offset(rva)
        validate_pipl(data[start:start + size], windows=True)


def inspect_macos(path):
    """既存の生成方式を維持し、Appleのツールでバンドルを確認する。"""
    contents = path / "Contents"
    info = plistlib.loads((contents / "Info.plist").read_bytes())
    if (info.get("CFBundleIdentifier") != "com.hanayuki.AfterEffects.rgba_contrast"
            or info.get("CFBundlePackageType") != "eFKT"):
        raise ValueError("RGBA Contrastのバンドルではありません")
    resource = (contents / "Resources/rgba_contrast.rsrc").read_bytes()
    data_offset, map_offset = struct.unpack_from(">II", resource)
    type_offset = map_offset + struct.unpack_from(">H", resource, map_offset + 24)[0]
    count = struct.unpack_from(">H", resource, type_offset)[0] + 1
    found = []
    for index in range(count):
        kind, length, refs = struct.unpack_from(">4sHH", resource, type_offset + 2 + 8 * index)
        if kind != b"PiPL":
            continue
        for item in range(length + 1):
            ident, _, offset, _ = struct.unpack_from(">hHII", resource, type_offset + refs + 12 * item)
            if ident == 16000:
                start = data_offset + (offset & 0xffffff)
                size = struct.unpack_from(">I", resource, start)[0]
                found.append(resource[start + 4:start + 4 + size])
    if len(found) != 1:
        raise ValueError("PiPL 16000がありません")
    validate_pipl(found[0], windows=False)
    binary = contents / "MacOS/rgba_contrast"
    arches = subprocess.check_output(["xcrun", "lipo", "-archs", str(binary)], text=True).split()
    if not arches or not set(arches) <= {"x86_64", "arm64"}:
        raise ValueError("macOSのCPU構成が不正です")
    for arch in arches:
        header = subprocess.check_output(["xcrun", "otool", "-arch", arch, "-hv", str(binary)], text=True)
        if "DYLIB" not in header:
            raise ValueError("Mach-Oの動的ライブラリーではありません")
        symbols = subprocess.check_output(["xcrun", "nm", "-arch", arch, "-gU", str(binary)], text=True)
        if "_EffectMain" not in {line.split()[-1] for line in symbols.splitlines() if line.split()}:
            raise ValueError("EffectMainが公開されていません")
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(path)], check=True)


def inspect_artifact(path, system):
    try:
        if path.is_symlink():
            raise ValueError(f"リンクは配置できません: {path}")
        if system == "Windows":
            inspect_windows(path.read_bytes())
        else:
            inspect_macos(path)
    except (struct.error, IndexError, UnicodeError, plistlib.InvalidFileException) as error:
        raise ValueError(f"成果物が短すぎるか破損しています: {path}") from error


def install_artifact(source, final):
    """複写を完了してから置換し、失敗した場合は既存の成果物を保持する。"""
    if final.is_symlink():
        raise ValueError(f"配置対象がシンボリックリンクです: {final}")
    final.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".rgba-install-", dir=final.parent))
    staged = staging / source.name
    backup = staging / "previous"
    installed = False
    try:
        if source.is_dir():
            shutil.copytree(source, staged, symlinks=True)
            if final.exists():
                os.replace(final, backup)
        else:
            shutil.copy2(source, staged)
        try:
            os.replace(staged, final)
        except OSError:
            if backup.exists():
                os.replace(backup, final)
            raise
        installed = True
    finally:
        if installed or not backup.exists():
            # この呼び出しで配置先の直下に作った一時領域だけを消す。
            assert staging.resolve().parent == final.parent.resolve()
            shutil.rmtree(staging)
        else:
            print(f"復元できなかった旧バンドルを保持しました: {backup}", file=sys.stderr)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", nargs="?", help="配置先フォルダー。環境変数より優先")
    parser.add_argument("--dry-run", action="store_true", help="ビルド・書き込みをせず予定を表示")
    args = parser.parse_args(argv)
    system = platform.system()
    if system not in ("Windows", "Darwin"):
        parser.error("プラグインの導入はWindowsとmacOSに対応しています")
    try:
        name = os.environ["PluginName"]  # Justfileを製品名の正本とする。
        target_value = os.environ.get("CARGO_TARGET_DIR", "target")
        default_dir = (
            r"C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore"
            if system == "Windows"
            else "/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore"
        )
        destination_value = (
            args.destination if args.destination is not None
            else os.environ.get("AE_PLUGIN_DIR", default_dir)
        )
        if not target_value.strip() or not destination_value.strip():
            raise ValueError("生成先と配置先に空文字列は指定できません")
        target = (ROOT / target_value).resolve()
        destination = (ROOT / destination_value).resolve()
        suffix = ".aex" if system == "Windows" else ".plugin"
        artifact = target / "release" / (name + suffix)
        final = destination / artifact.name
        if destination == artifact.parent or destination.is_relative_to(artifact):
            raise ValueError("配置先は生成先以外のフォルダーを指定してください")
        if destination.exists() and not destination.is_dir():
            raise ValueError("配置先にはフォルダーを指定してください")
        print(f"ビルド: just release\n成果物: {artifact}\n配置先: {final}", flush=True)
        if args.dry_run:
            print("ビルド・配置: 未実施（書き込みなし）")
            return 0
        subprocess.run(["just", "--justfile", str(ROOT / "Justfile"), "release"], cwd=ROOT, check=True)
        if not (artifact.is_file() if system == "Windows" else artifact.is_dir()):
            raise ValueError(f"成果物が生成されていません: {artifact}")
        inspect_artifact(artifact, system)
        if final.exists() or final.is_symlink():
            inspect_artifact(final, system)
        install_artifact(artifact, final)
        print(f"配置完了: {final}")
        return 0
    except subprocess.CalledProcessError as error:
        return error.returncode
    except (OSError, ValueError) as error:
        print(f"導入失敗: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
