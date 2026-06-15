# Changelog

All notable changes to kukuri are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Releases use the preview tag scheme `vX.Y.Z-preview.N`.

Per-release sections under this header are generated automatically by the
`changelog` job in `.github/workflows/kukuri-release.yml`, which runs
`scripts/release/update-changelog.ps1` against the git history between the
previous release tag and the release tag, links each entry to its pull
request, and commits the result. See `docs/runbooks/release.md` for the
release workflow.

Changes released in `v0.1.1-preview.1` and earlier are tracked in the
[GitHub Releases](https://github.com/KingYoSun/kukuri/releases) instead of this
file; automated changelog entries start from the next preview release.

## [Unreleased]

## [v0.1.2-preview.1] - 2026-06-15

### Features

- リリースごとのCHANGELOG自動生成・運用を追加 ([#342](https://github.com/KingYoSun/kukuri/pull/342), [#344](https://github.com/KingYoSun/kukuri/pull/344))
- topic一覧にsearch/filter/sort機能を追加 ([#340](https://github.com/KingYoSun/kukuri/pull/340), [#343](https://github.com/KingYoSun/kukuri/pull/343))
- topic/channelごとのGossip接続トグルを追加 ([#305](https://github.com/KingYoSun/kukuri/pull/305), [#341](https://github.com/KingYoSun/kukuri/pull/341))
- リポスト・リプライ・スレッドのUIを改善 ([#307](https://github.com/KingYoSun/kukuri/pull/307), [#337](https://github.com/KingYoSun/kukuri/pull/337))
- アプデ通知を改善（バナー廃止・更新時のみDLボタン表示・リリース設定を整理） ([#333](https://github.com/KingYoSun/kukuri/pull/333))
- add Japanese font fallback and a monospace token ([#328](https://github.com/KingYoSun/kukuri/pull/328))

### Fixes

- changelog ジョブを main 直push からPR作成方式へ変更 ([#348](https://github.com/KingYoSun/kukuri/pull/348))
- third-party notices のソートをオーディナル化しCI差異を解消 ([#347](https://github.com/KingYoSun/kukuri/pull/347))
- 接続エラー復帰後に community-node エラー表示が消えない問題を修正 ([#312](https://github.com/KingYoSun/kukuri/pull/312), [#335](https://github.com/KingYoSun/kukuri/pull/335))
- OS通知をRustバックエンド経由に変更しWindowsで発火するように修正 ([#313](https://github.com/KingYoSun/kukuri/pull/313), [#334](https://github.com/KingYoSun/kukuri/pull/334))
- unify shell breakpoints to the 759/899/900/1099/1100 system ([#331](https://github.com/KingYoSun/kukuri/pull/331))
- resolve undefined CSS custom-property references in shell styles ([#327](https://github.com/KingYoSun/kukuri/pull/327))

### Other

- regenerate third-party notices for 0.1.2 ([#346](https://github.com/KingYoSun/kukuri/pull/346))
- bump preview version to 0.1.2 ([#345](https://github.com/KingYoSun/kukuri/pull/345))
- AGENTS.local.mdを追加 ([#339](https://github.com/KingYoSun/kukuri/pull/339))
- @ ([#338](https://github.com/KingYoSun/kukuri/pull/338))
- codegraph導入 ([#336](https://github.com/KingYoSun/kukuri/pull/336))
- tokenize elevation, blur, and the metaverse canvas color ([#332](https://github.com/KingYoSun/kukuri/pull/332))
- tokenize spacing and radius into --space-* / --radius-* scales ([#330](https://github.com/KingYoSun/kukuri/pull/330))
- consolidate font-sizes into a --text-* type scale ([#329](https://github.com/KingYoSun/kukuri/pull/329))
- Rework DESIGN.md into a concrete visual design spec ([#326](https://github.com/KingYoSun/kukuri/pull/326))
- Update release readiness manual items ([#324](https://github.com/KingYoSun/kukuri/pull/324))
- Add startup database error screen ([#323](https://github.com/KingYoSun/kukuri/pull/323))
- Add store migration fixture ([#322](https://github.com/KingYoSun/kukuri/pull/322))
- Generate third-party notices ([#321](https://github.com/KingYoSun/kukuri/pull/321))
- Add updater error guidance ([#320](https://github.com/KingYoSun/kukuri/pull/320))
- Fix community node settings notice link ([#317](https://github.com/KingYoSun/kukuri/pull/317))

