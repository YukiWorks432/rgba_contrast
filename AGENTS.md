# Repository Agent Instructions

運用プロファイル: hanayuki-personal

## ブランチとリリース

- `main` を通常開発の基点と統合先とし、直接プッシュせず PR を使う。作業ブランチは `feat/`、`fix/`、`chore/` などを使う。
- 版番号更新は `main` から作る短命な `release/v<version>` ブランチの PR で行う。リリース手順は `.codex/skills/rgba-contrast-release/SKILL.md` を参照する。

## 書式

- 変更後は `just fmt` で整形し、コミット前に `just fmt-check` を実行する。対象と導入手順はREADMEの「整形」を参照する。
