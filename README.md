# RGBA Contrast
After Effects / Premiere Pro用プラグイン。

RGBAごとにコントラストを調節できます。
## License
MIT License Copyright 2025 花雪(X: @Yuk1_Works)
## パラメーター
**Red / Green / Blue / Alpha**

それぞれのコントラストを調整できます。

**R/G/B/A/Pivot**

コントラストを調整する原点を変更できます。

**Mode**

- Modern: より滑らかでモダンなコントラスト調整を行います。
- Legacy: 直線的でレガシーなコントラスト調整を行います。

Modernは黒と白を保つように正規化したシグモイド曲線で調整します。コントラストを上げる場合、Pivotは曲線の変曲位置を指定し、一般に入力値を固定する点ではありません。下げる場合はその逆変換を使います。32ビットの範囲外の値は、逆関数の対数の定義域を超える場合に内部値を制限します。

整数画素は最も近い値へ丸めます。計算精度と丸めの修正により、既存プロジェクトの出力が変わる場合があります。

## 開発用の依存関係

`after-effects` とビルド時に使う `pipl` は、[上流リポジトリ](https://github.com/virtualritz/after-effects)の同じコミットを `Cargo.toml` の `rev` に指定しています。`Cargo.lock` も管理し、通常のビルドでは `--locked` を付けて検証済みの依存関係を使います。初回の取得には GitHub と crates.io への接続が必要です。

上流の修正を取り込む場合は、リポジトリのルートで次の手順を実施してください。

1. 上流の変更内容を確認し、2つの `rev` を取り込みたい同じコミットの完全な SHA に変更します。
2. 次のコマンドで関連する依存関係を更新し、`Cargo.lock` の差分を確認します。

   ```sh
   cargo update -p after-effects -p pipl
   cargo tree --locked -i after-effects
   cargo tree --locked -i pipl
   ```

3. 次の検証を実行し、After Effects / Premiere Pro で読み込みと描画を確認します。macOS の配布用バンドルは `just release` で作成できます。

   ```sh
   cargo fmt --check
   cargo clippy --locked -- -D warnings
   cargo test --locked
   cargo test --release --locked
   cargo build --release --locked
   ```

4. `Cargo.toml` と `Cargo.lock` を同じ PR に含め、取り込む修正、API 対応、検証結果と未確認の環境を記録します。

`rev` を固定しているため、ビルドや `cargo update` だけで別の上流コミットへ切り替わることはありません。

## 開発とリリース

通常の作業ブランチは `main` から作り、変更は `main` 向けの PR で統合します。`main` への直接プッシュは行いません。

リリース時は `main` から `release/v<version>` ブランチを作り、`Cargo.toml` と必要な `Cargo.lock` の版番号を更新した PR を作成します。PR を `main` にマージすると、GitHub Actions がそのマージコミットから Windows / macOS の成果物を構築し、成功後にタグと GitHub Release を作成します。通常の開発 PR では公開されません。詳しい検証と失敗時の対応は `.codex/skills/rgba-contrast-release/SKILL.md` を参照してください。

## ビルドとインストール

Rustに加えて `just`、`uv`、Python 3.11以上を用意し、リポジトリのルートで実行してください。導入補助はPythonの標準ライブラリだけを使います。`uv`によるPythonの自動ダウンロードは行いません。

| コマンド | 動作 |
| --- | --- |
| `just build` | 開発用ビルドを生成します。 |
| `just release` | 最適化ビルドを生成します。Windowsは `.aex`、macOSは署名付きユニバーサル `.plugin` を作成します。 |
| `just install-plan [配置先]` | ビルド・書き込みをせず、生成先と配置先を表示します。 |
| `just install [配置先]` | `just release` の成功後、対象プラグインを検査・配置して完了を待ちます。 |
| `just test-tools` | 配置とリリース補助の契約を検証します。 |

`build`と`release`からの自動インストールは廃止しました。インストールには`just install`を使用してください。`NO_INSTALL`の指定は不要です。

配置先の優先順位は、コマンドの引数、環境変数 `AE_PLUGIN_DIR`、OS既定のMediaCoreフォルダーです。Windowsの既定値は `C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore`、macOSは `/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore` です。生成先は `CARGO_TARGET_DIR` に従い、未指定時は `target` を使います。相対パスはリポジトリのルートを基準にします。

例えば、書き込み可能な検証用フォルダーへ配置できます。空白を含むパスは引用符で囲んでください。

```sh
just install-plan "target/test plugins"
just install "target/test plugins"
```

配置前に、生成物と同名の配置済みプラグインのOS形式、公開入口 `EffectMain`、PiPLの表示名 `RGBA Contrast` とMatch Name `HANAYUKI RGBA Contrast` を検査します。別製品や破損ファイルは上書きしません。macOSではバンドル識別子と署名も検証するため、Xcodeのコマンドラインツールが必要です。

ビルドに失敗した場合は配置せず、複写や置換に失敗した場合も既存ファイルを保持します。macOSのバンドルは一時領域で複写してから置換するため、旧版だけに存在したファイルが残りません。書き込み権限が必要な配置先は、権限を持つ端末から実行してください。使用中のプラグインを更新できない場合はAfter Effects／Premiere Proを終了して再実行してください。

配置の成功とAdobeホスト上の読み込み・描画確認は別です。
