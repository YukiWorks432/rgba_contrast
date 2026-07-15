---
name: rgba-contrast-release
description: RGBA Contrast の次回リリースを安全に準備する。Cargo.toml の SemVer、既存タグ、ローカル検証の sign-off、develop から main へのリリース PR を確認・準備する必要があるときに使用する。タグ作成や GitHub Release の手動公開には使用しない。
---

# RGBA Contrast のリリース準備

## 役割

リリース前の検査と PR 準備を行う。`main` へのマージ後は GitHub Actions が Windows / macOS の成果物、タグ、GitHub Release を自動作成するため、手動でタグ・Release を作成しない。

## 手順

1. `git status --short` を確認し、無関係な変更や未確認の変更がある場合は停止する。
2. `Cargo.toml` の `package.version` を正本として読み取り、有効な SemVer であることを確認する。リリース番号を上げる必要があれば、`develop` を基点とした変更で更新する。`main` で直接更新しない。
3. `v<version>` と `<version>` の両方を確認する。いずれかが既に存在する場合、そのバージョンを再公開しない。既存の `1.0.0` は互換対象であり、`1.0.0` は新規 Release を生成しない。
4. 利用者の sign-off として、ローカルで `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` を実行済みか確認する。未実行なら実行するか、実行を依頼して結果を PR に残す。
5. バージョン更新、変更内容、検証結果を含む `develop → main` の PR を準備する。Release ノートは GitHub が自動生成するため、PR 本文には利用者に影響する変更を簡潔に記載する。
6. macOS 成果物は universal `.plugin` であり、ad-hoc 署名済みだが Developer ID 署名・notarization は未実施であることを確認する。
7. `main` へのマージ後、Release workflow の完了を確認する。Windows / macOS ZIP、`SHA256SUMS.txt`、タグ、GitHub Release のすべてが作成されたことを確認する。

## 失敗時の扱い

- `main` 向け PR の `ci` が失敗した場合、原因を `develop` 側で修正してから再実行する。
- Release workflow が失敗した場合、同じバージョンのタグや Release を手動で作り直さない。失敗段階と既存タグ・Release の有無を確認し、冪等性を保つ修正 PR を作成する。
- Developer ID 署名または notarization が必要になった場合、この Skill の範囲外として別 Issue を作成する。
