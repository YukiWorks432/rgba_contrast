"""ローカルとCIで同じ版の文書整形器を使う。"""

import subprocess
import sys

try:
    result = subprocess.run(["dprint", "--version"], capture_output=True, text=True, check=True)
except FileNotFoundError:
    sys.exit("dprint 0.60.1を導入してください。README.mdの「整形」を参照。")
if result.stdout.strip() != "dprint 0.60.1":
    sys.exit("dprint 0.60.1を導入してください。README.mdの「整形」を参照。")
