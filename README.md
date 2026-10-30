# RGBA Contrast
After Effects / Premiere Pro用プラグイン。

RGB・HSL・HSV・YUV（BT.709）・OKLab・OKLch の各軸とアルファを、コントラストとガンマで調整できます。
## License
MIT License Copyright 2025 花雪(X: @Yuk1_Works)
## パラメーター

`Color Space` で色空間を選びます。操作項目は次のグループに分かれています。

- **Contrast**：方式、各軸とアルファのコントラスト、対応するPivot。初期状態で展開。
- **Gamma**：各軸とアルファのガンマ、色相ガンマの起点。初期状態で折りたたみ。
- **Options**：入力RGBの解釈と、色とアルファの分離。初期状態で折りたたみ。

色相ガンマの起点はHSL・HSV・OKLchで使用できます。軸名は選択した色空間に応じて変わります。

**Red / Green / Blue / Alpha**

それぞれのコントラストを調整できます。

**R/G/B/A/Pivot**

コントラストを調整する原点を変更できます。

**Mode**

- Modern: より滑らかでモダンなコントラスト調整を行います。
- Legacy: 直線的でレガシーなコントラスト調整を行います。

Modernは黒と白を保つように正規化したシグモイド曲線で調整します。コントラストを上げる場合、Pivotは曲線の変曲位置を指定し、一般に入力値を固定する点ではありません。下げる場合はその逆変換を使います。32ビットの範囲外の値は、逆関数の対数の定義域を超える場合に内部値を制限します。

整数画素は最も近い値へ丸めます。計算精度と丸めの修正により、既存プロジェクトの出力が変わる場合があります。


**Color Space / Input RGB**

色空間と、入力 RGB の伝達特性を選びます。HSL/HSV は符号化済み sRGB、YUV は BT.709、OKLab/OKLch は線形 sRGB を計算基準にし、調整後に入力の表現へ戻します。入力設定は自動判別されません。

各軸の Contrast、Pivot、Gamma を調整できます。Gamma の既定値は 1 です。H/h のガンマは Hue Gamma Origin を起点として色相を再配分します。色相のコントラストは周期的な曲線を使います。

HSL で色を調整すると、入力の負値と 1 を超える値が制限されます。HSV は負値を制限し、1 を超える明るさを扱います。無調整およびアルファのみの調整では色の範囲制限を行いません。範囲外の画素は、無調整から色の調整を始めたときに急に変化する場合があります。

**Separate Color and Alpha**

オンの場合、色とアルファを分離して処理します。新規適用時はオン、旧プロジェクトではオフが既定です。アルファ 0 の乗算済み画素では色を除算できないため、元の色を復元できません。ホストごとの画素形式と既存プロジェクトの読み込みは実機での確認が必要です。

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

3. 次の検証を実行し、After Effects / Premiere Pro で読み込みと描画を確認します。macOS の配布用バンドルは `NO_INSTALL=1 just release` で作成できます。

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
