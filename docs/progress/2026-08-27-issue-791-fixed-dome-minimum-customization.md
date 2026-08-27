# Issue #791: 固定規格Domeと最小customization

## 実装した境界

- Metaverse room payloadを後方互換なしで`world_version = 2` / `fixed_dome_v1`へ置き換えた。
- code-owned geometryとして、内半径20m・外半径22m・頂点高20mの真半球、4方向の幅5m／高さ10mのarch opening、奥行き15mのconnection zone、中央境界、隣接中心間57mを固定した。
- owner署名manifestへsurface material / texture、lighting、ambient、fog、-Y固定gravityの強度、persistent prop初期定義だけを保存する。
- 任意world mesh、script、geometry変更、gravity方向変更、physics disableをwire schemaから除外し、未知fieldと範囲外値を拒否する。
- avatar / propは明示colliderを優先し、欠落時はbounding boxを包含するY軸capsuleへfallbackする。
- desktop sceneで固定Dome、opening、connection zone、基本境界collision、texture/material、environment、persistent propを表示する。
- `grab` / `throw` / `push` / `sit`をtyped interactionとして単一clientで処理し、実行中transformはTransient room eventとして扱う。
- ownerにはdraft / save / cancel / pending / success / validation / backend errorを持つcustomization UI、non-ownerにはread-only表示を提供する。

## 後続Issueとの境界

- Spatial ContextとDome Instanceの所有・移動はIssue #789。
- Connection record、proposal、実際のDome間接続はIssue #792。
- authoritative host / physics simulationはIssue #788。
- guest prop、layout commit、physics snapshotはIssue #793。

## 検証

- `cargo test -p kukuri-core fixed_dome`
- `cargo test -p kukuri-app-api metaverse_room`
- `cargo test -p kukuri-store row_mapping_roundtrip_live_game`
- `cargo test -p kukuri-store backend_parity`
- `cargo xtask scenario desktop_smoke_metaverse_dome_persist`
- frontend targeted Vitest（Dome model、customization、room controls/view/panel/session）
- Storybook + in-app Browser visual review（1280px / 480px、overflowなし、主要action 48px）
- `cargo xtask check`
- `cargo xtask test`（Rust 609件、harness 19件、frontend 881件、doc test）
- `cargo xtask desktop-ui-check`（Storybook build、browser E2E 44件、visual regression 14件）
- `cargo xtask e2e-smoke`
- `cargo xtask oversized-files`（既存baseline 4件のみ）
- `git diff --check`

UI visual reviewの記録とpreviewは
`docs/ui-reviews/2026-08-27-issue-791-fixed-dome-customization.md`に保存した。
