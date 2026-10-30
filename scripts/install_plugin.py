"""最適化ビルド後にプラグインを配置する。予定表示では書き込まない。"""

import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


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
