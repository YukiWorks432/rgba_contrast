---
name: rgba-contrast-release
description: RGBA Contrast の版番号更新PRとリリース前検証を準備する。GitHub Releaseやタグの手動公開は行わない。
---

# RGBA Contrast のリリース準備

`main` を基点とし、手動で版番号更新PRを準備する。`release/v<version>` から `main` へマージされたPRだけが公開処理を開始する。手動でタグやReleaseを作成しない。

1. 作業ツリーと最新の `origin/main` を確認する。
2. `Cargo.toml` の `package.version` を正本として新しいSemVerを選び、`v<version>` と `<version>` の両タグが存在しないことを確認する。既存 `1.0.0` は再公開しない。
3. `main` から `release/v<version>` を作り、`Cargo.toml` と必要な `Cargo.lock` の版番号だけを更新する。ブランチ名と版番号を一致させる。
4. `cargo fmt --check`、`cargo clippy --locked -- -D warnings`、`cargo test --locked` を実行し、結果をPRに記載する。
5. `main` 向けDraft PRを作成し、利用者に影響する変更と検証結果を記載する。PRの準備・マージは既存の品質ゲートと利用者の許可に従う。
6. マージ後はRelease workflowを確認し、Windows / macOS ZIP、`SHA256SUMS.txt`、タグ、GitHub Releaseが作成されたことを確認する。macOS版はad-hoc署名であり、Developer ID署名・notarizationは未実施。

公開に失敗した場合は、対象のマージSHA、失敗段階と既存タグ・Releaseの有無を確認する。公開前の失敗は該当workflowを再実行できる。タグやReleaseが既に存在する場合は内容を確認し、自動で付け替え・上書きせず復旧方法を検討する。
