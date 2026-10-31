# RGBA Contrast

After Effects / Premiere Pro用の、コントラストとガンマを調整するプラグインです。RGB・HSL・HSV・YUV（BT.709）・OKLab・OKLchの各軸とアルファを個別に調整できます。

## パラメーター

`Color Space` で色空間を選びます。既定値は `RGB` です。各軸の表示名は色空間に合わせて変わります。

| 色空間        | 調整する三軸                      |
| ------------- | --------------------------------- |
| RGB           | Red・Green・Blue                  |
| HSL           | H（色相）・S（彩度）・L（明度）   |
| HSV           | H（色相）・S（彩度）・V（明るさ） |
| YUV（BT.709） | Y（輝度成分）・U/V（色差成分）    |
| OKLab         | L（明度）・a/b（色成分）          |
| OKLch         | L（明度）・C（彩度）・h（色相）   |

操作項目は `Contrast`、`Gamma`、`Options` のグループに分かれています。`Contrast` は初期状態で展開され、残りのグループは折りたたまれています。

### コントラストとガンマ

`Contrast` には方式を選ぶ `Mode` と、三軸・アルファそれぞれのコントラストと `Pivot` があります。コントラストの範囲は−100〜100、既定値は0です。`Pivot` は0〜100%、既定値は50%です。

- `Modern`：各軸の基準範囲の端点を保つように正規化したシグモイド曲線を使います。コントラストを上げる場合、`Pivot` は曲線の変曲位置を指定し、一般にその入力値を固定する点ではありません。下げる場合は逆変換を使います。
- `Legacy`：`Pivot` を中心とした直線的なコントラスト調整を行います。

色相のH/hには、方式にかかわらず周期的なコントラスト曲線を使います。色相の `Pivot` は0〜100%が0〜360°に対応します。

`Gamma` はコントラストの後に適用します。各軸とアルファの既定値は1で、入力範囲は0.10〜10.00、スライダー範囲は0.25〜4.00です。`Hue Gamma Origin` は色相ガンマの起点を0〜360°で指定します。HSL・HSV・OKLchで使え、既定値は0°です。

すべてのコントラストが0、ガンマが1のときは、色空間の往復変換を行わず入力画素を保持します。

### 入力RGBと範囲外の値

`Options` の `Input RGB` は、ホストから受け取るRGB値の伝達特性を指定します。選択肢は `sRGB`、`Rec.709 Gamma 2.4`、`Rec.709`、`Linear sRGB / Rec.709` で、既定値は `sRGB` です。ホストの色管理設定は自動判別しないため、入力に合わせて選んでください。

HSL/HSVは符号化済みsRGB、YUVはBT.709、OKLab/OKLchは線形sRGBを計算基準にし、調整後に入力の表現へ戻します。RGBモードでは伝達特性の変換を行いません。

HSLで色を調整すると、入力の負値と1を超える値が制限されます。HSVは負値を制限し、1を超える明るさを扱います。無調整およびアルファのみの調整では、この色の範囲制限を行いません。範囲外の画素は、無調整から色の調整を始めたときに急に変化する場合があります。

Modernの逆変換では、対数の定義域を超える32ビットの値について内部値を制限します。整数画素への出力は最も近い値へ丸めます。

### アルファと既存プロジェクト

`Separate Color and Alpha` は色とアルファを分離して扱う設定です。新規適用時はオン、この項目を持たない旧プロジェクトの読み込み時は互換用のオフを補完します。現在のAE・Premiereの描画経路では、ホストのRGB値をアルファが乗算されていない値として扱います。

既存パラメーターの保存IDは維持していますが、計算精度と整数への丸めの修正によって以前の出力と差が出る場合があります。旧プロジェクトの復元、グループ表示、描画結果は対象のAdobeホストで確認してください。自動テストや配置の成功だけでは、実機での互換性確認にはなりません。

## ビルドと配置

リポジトリのルートで実行します。Rustの安定版（`stable`）ツールチェーン、`just`、`uv`、Python 3.11以上を用意してください。WindowsではMSVC向けのC++ビルド環境、macOSではXcodeのコマンドラインツールが必要です。配置とリリースの補助スクリプトはPythonの標準ライブラリだけを使い、`uv`によるPythonの自動ダウンロードは行いません。

| コマンド                     | 動作                                                                                                     |
| ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| `just build`                 | 開発用ビルドを生成します。                                                                               |
| `just release`               | 最適化ビルドを生成します。Windowsは `.aex`、macOSはIntel・Apple Silicon両対応の `.plugin` を作成します。 |
| `just install-plan [配置先]` | ビルド・書き込みをせず、生成先と配置先を表示します。                                                     |
| `just install [配置先]`      | `just release` の成功後、対象プラグインを検査・配置して完了を待ちます。                                  |
| `just test-tools`            | 配置とリリース補助のテストを実行します。                                                                 |

`just build` と `just release` はインストールしません。配置には `just install` を使います。`NO_INSTALL` の指定は不要です。

配置先の優先順位は、コマンドの引数、環境変数 `AE_PLUGIN_DIR`、OS既定のMediaCoreフォルダーです。

| OS      | 既定の配置先                                                       |
| ------- | ------------------------------------------------------------------ |
| Windows | `C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore`             |
| macOS   | `/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore` |

生成先は `CARGO_TARGET_DIR` に従い、未指定時は `target` です。相対パスはリポジトリのルートを基準にします。空白を含むパスは引用符で囲んでください。例えば、書き込み可能な検証用フォルダーへ配置できます。

```sh
just install-plan "target/test plugins"
just install "target/test plugins"
```

配置前に、生成物と同名の配置済みプラグインのOS形式、公開入口 `EffectMain`、PiPLの製品名を検査します。別製品や破損ファイルは上書きしません。macOSではバンドル識別子と署名も検証します。ビルド・複写・置換に失敗した場合は、配置済みのプラグインを保持します。

書き込み権限が必要な配置先では、権限を持つ端末から実行してください。使用中のプラグインを更新できない場合はAfter Effects／Premiere Proを終了して再実行してください。

macOSのローカルビルドでは、利用可能なApple Development証明書があればその証明書を使い、なければ開発用のアドホック署名を使います。一般配布用のDeveloper ID署名・公証は別途必要です。CIで生成するリリース成果物はアドホック署名で、Developer ID署名・公証は行いません。

## 開発

通常の作業ブランチは最新の `main` から作り、変更は `main` 向けのPRで統合します。`main` への直接プッシュは行いません。

`plans/` はローカルの検討メモ置き場としてGitの管理対象から外しています。確定した仕様や利用手順は、このREADMEなどGitで管理する文書へ反映します。

### 整形

Rustは `rustfmt`、Markdown・JSON・JSONCは `dprint` で整形します。Dither Lab／Impactと同じく、Rustは2024年版の書式・4スペース、文書とJSONは2スペースを基本とします。改行はLFに統一し、エディター向けの設定は `.editorconfig` で共有します。

`dprint` はImpactと同じ `0.60.1` に固定しています。Rustの `rustfmt` と合わせて、一度導入してください。

```sh
rustup component add rustfmt
cargo install dprint --version 0.60.1 --locked
```

| コマンド         | 動作                                             |
| ---------------- | ------------------------------------------------ |
| `just fmt`       | Rustと文書・JSONを整形します。                   |
| `just fmt-check` | ファイルを変更せず、同じ対象の書式を検証します。 |

`dprint.json` で対象ファイルとプラグインの版を固定しています。`target/`、`plans/` とGitの除外対象は整形しません。使用する `dprint` の版が異なる場合は、整形する前に停止して導入方法を案内します。

### 検証

次のコマンドで書式、静的解析、Rustの通常・最適化テスト、補助スクリプト、プラグインの生成を確認します。`just release` は配置を行いません。

```sh
just fmt-check
cargo clippy --locked -- -D warnings
cargo test --locked
cargo test --release --locked
just test-tools
just release
```

Windowsでは、生成済みDLLのPiPL構造と必須フラグも検査します。`CARGO_TARGET_DIR` を指定した場合は、DLLのパスを読み替えてください。

```sh
uv run --no-project --no-python-downloads python -X utf8 scripts/verify_pipl.py target/release/rgba_contrast.dll
```

CIではWindowsでRustと文書・JSONの書式検査、Rustの静的解析・テスト・DLL生成・PiPL検査、macOSでユニバーサルバンドルの生成・署名検査・一時フォルダーへの配置を行います。両OSで配置・リリース補助のテストも実行します。Adobeホスト上の読み込み・描画確認は別途必要です。

### 上流クレートの更新

`after-effects` とビルド時に使う `pipl` は、[上流リポジトリ](https://github.com/virtualritz/after-effects)の同じコミットを `Cargo.toml` の `rev` に指定しています。`Cargo.lock` も管理し、通常のビルドには `--locked` を付けます。初回の取得にはGitHubとcrates.ioへの接続が必要です。

1. 上流の変更内容を確認し、2つの `rev` を取り込みたい同じコミットの完全なSHAに変更します。
2. 次のコマンドで依存関係を更新し、`Cargo.lock` の差分を確認します。

   ```sh
   cargo update -p after-effects -p pipl
   cargo tree --locked -i after-effects
   cargo tree --locked -i pipl
   ```

3. [検証](#検証)を実行し、利用できるAfter Effects／Premiere Proで読み込みと描画を確認します。
4. `Cargo.toml` と `Cargo.lock` を同じPRに含め、取り込む修正、API対応、検証結果と未確認の環境を記録します。

`rev` を固定しているため、ビルドや `cargo update` だけで別の上流コミットへ切り替わることはありません。

## リリース

1. `main` から `release/v<version>` ブランチを作り、`Cargo.toml` と必要な `Cargo.lock` の版番号だけを更新したPRを作成します。ブランチ名の版番号と `Cargo.toml` の `package.version` を一致させます。
2. 検証後にそのPRを `main` へマージすると、GitHub Actionsの `Release` ワークフローがマージコミットからWindows／macOSのZIPを生成します。
3. 両OSのビルド成功後、`v<version>` タグ、GitHub Release、ZIP、`SHA256SUMS.txt` を公開します。既存のタグがある版は再公開しません。

通常の開発PRでは公開されません。`scripts/validate_release.py` は公開前に、PRのマージ状態・リポジトリ・基点・ブランチ名・版番号・変更ファイルがリリース条件に合うかを検証する補助スクリプトです。通常の手動ビルドでは実行する必要はありません。

準備と失敗時の対応は[リリース手順](.codex/skills/rgba-contrast-release/SKILL.md)を参照してください。

## ライセンス

[MIT License](LICENSE)。Copyright 2025 花雪（X: @Yuk1_Works）
