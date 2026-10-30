# リリースで使うjust 1.42系のPythonスクリプト実行を有効にする。
set unstable

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

export TargetDir := env_var_or_default("CARGO_TARGET_DIR", "target")
export CARGO_TARGET_DIR := TargetDir
export PYTHONUTF8 := "1"

[windows]
build:
    cargo build --locked

[windows]
release:
    cargo build --locked --release
    Copy-Item -LiteralPath (Join-Path $env:TargetDir 'release/{{BinaryName}}.dll') -Destination (Join-Path $env:TargetDir 'release/{{PluginName}}.aex') -Force -ErrorAction Stop

[macos]
build:
    cargo build --locked
    just -f {{quote(justfile())}} create_bundle debug {{quote(TargetDir)}}

[macos]
release:
    cargo build --locked --release
    just -f {{quote(justfile())}} create_bundle release {{quote(TargetDir)}}

[macos]
[positional-arguments]
create_bundle profile target_dir:
    #!/usr/bin/env bash
    set -euo pipefail
    profile="$1"
    target_dir="$2"
    plugin_path="$target_dir/$profile/$PluginName.plugin"
    rm -rf "$plugin_path"
    mkdir -p "$plugin_path/Contents/Resources" "$plugin_path/Contents/MacOS"

    if [ "$profile" == "release" ]; then
        rustup target add aarch64-apple-darwin x86_64-apple-darwin
        cargo build --locked --release --target x86_64-apple-darwin
        cargo build --locked --release --target aarch64-apple-darwin
        cp "$target_dir/x86_64-apple-darwin/release/{{BinaryName}}.rsrc" "$plugin_path/Contents/Resources/$PluginName.rsrc"
        cp "$target_dir/x86_64-apple-darwin/release/{{BinaryName}}_PkgInfo" "$plugin_path/Contents/PkgInfo"
        cp "$target_dir/x86_64-apple-darwin/release/{{BinaryName}}_Info.plist" "$plugin_path/Contents/Info.plist"
        lipo "$target_dir/x86_64-apple-darwin/release/lib{{BinaryName}}.dylib" "$target_dir/aarch64-apple-darwin/release/lib{{BinaryName}}.dylib" -create -output "$plugin_path/Contents/MacOS/$PluginName"
    else
        cp "$target_dir/$profile/{{BinaryName}}.rsrc" "$plugin_path/Contents/Resources/$PluginName.rsrc"
        cp "$target_dir/$profile/{{BinaryName}}_PkgInfo" "$plugin_path/Contents/PkgInfo"
        cp "$target_dir/$profile/{{BinaryName}}_Info.plist" "$plugin_path/Contents/Info.plist"
        cp "$target_dir/$profile/lib{{BinaryName}}.dylib" "$plugin_path/Contents/MacOS/$PluginName"
    fi
    /usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier "{{BundleIdentifier}}"' "$plugin_path/Contents/Info.plist"

    DEV_CERT=$(security find-identity -v -p codesigning | awk '/Apple Development/ {print $2; exit}')
    if [ -n "$DEV_CERT" ]; then
        codesign --options runtime --timestamp -strict --sign "$DEV_CERT" "$plugin_path"
    else
        codesign --options runtime --timestamp -strict --sign - "$plugin_path"
        echo "開発用のad-hoc署名を使用しました。一般配布用の署名・公証は別途必要です。"
    fi

# 最適化ビルド後にプラグインを配置する。
[positional-arguments]
[script("uv", "run", "--no-project", "--no-python-downloads", "python")]
install *args:
    import runpy, sys
    sys.argv = ["scripts/install_plugin.py", *sys.argv[1:]]
    runpy.run_path("scripts/install_plugin.py", run_name="__main__")

# ビルド・書き込みをせず、生成先と配置先を表示する。
[positional-arguments]
[script("uv", "run", "--no-project", "--no-python-downloads", "python")]
install-plan *args:
    import runpy, sys
    sys.argv = ["scripts/install_plugin.py", "--dry-run", *sys.argv[1:]]
    runpy.run_path("scripts/install_plugin.py", run_name="__main__")
