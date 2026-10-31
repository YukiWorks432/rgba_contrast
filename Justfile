export PluginName := "rgba_contrast"
BundleIdentifier := "com.hanayuki.AfterEffects." + PluginName
BinaryName       := lowercase(PluginName)

import "./AdobePlugin.just"

# 配置とリリース補助の契約を検証する。
test-tools:
    uv run --no-project --no-python-downloads python -X utf8 -m unittest discover -s tests -v
