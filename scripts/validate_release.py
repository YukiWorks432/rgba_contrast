import json
import re
import sys
import tomllib
from pathlib import Path


def validate_release(event, version, files, previous_version):
    pr = event["pull_request"]
    repository = event["repository"]["full_name"]
    identifier = r"(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    semver = rf"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-{identifier}(?:\.{identifier})*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    if not re.fullmatch(semver, version):
        raise ValueError("Cargo.toml の版が有効な SemVer ではありません")
    if (event.get("action") != "closed" or not pr["merged"] or
        pr["base"]["ref"] != "main" or
        pr["head"]["repo"]["full_name"] != repository or
        pr["head"]["ref"] != f"release/v{version}" or
        not re.fullmatch(r"[0-9a-f]{40}", pr["merge_commit_sha"] or "")):
        raise ValueError("公開対象のPR、版番号、マージSHAが一致しません")
    if version == previous_version:
        raise ValueError("版番号が変更されていません")
    names = {item["filename"] for page in files for item in page}
    if "Cargo.toml" not in names or not names <= {"Cargo.toml", "Cargo.lock"}:
        raise ValueError("版番号更新PRには Cargo.toml と Cargo.lock だけを含めてください")
    return version


def main():
    event = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    files = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    with open("Cargo.toml", "rb") as cargo:
        version = tomllib.load(cargo)["package"]["version"]
    with open(sys.argv[3], "rb") as previous:
        previous_version = tomllib.load(previous)["package"]["version"]
    print(f"version={validate_release(event, version, files, previous_version)}")


if __name__ == "__main__":
    main()
