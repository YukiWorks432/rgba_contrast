export PluginName := "rgba_contrast"
BundleIdentifier := "com.hanayuki.AfterEffects." + PluginName
BinaryName       := lowercase(PluginName)

import "./AdobePlugin.just"

# Rustと文書・JSONを整形する。
fmt:
    uv run --no-project --no-python-downloads python -X utf8 scripts/check_dprint.py
    cargo fmt --all
    dprint fmt

# ファイルを変更せず、整形済みか検証する。
fmt-check:
    uv run --no-project --no-python-downloads python -X utf8 scripts/check_dprint.py
    cargo fmt --all -- --check
    dprint check

# 配置とリリース補助の契約を検証する。
test-tools:
    uv run --no-project --no-python-downloads python -X utf8 -m unittest discover -s tests -v
