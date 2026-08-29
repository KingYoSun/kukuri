# Issue #792 Dome Connection topology 実装記録

## 状態

実装済み。PR作成前のlocal validationは完了。

## 実装内容

- ADR 0037でproposal、receiver selection、双方署名agreement、lifecycle、競合裁定、component-local topologyを固定した。
- coreに4方向opposite、署名検証、proposal derived state、因果参照とrecord digestによる候補順序、slot / merge / cycle / coordinate collision拒否、revoke後のforest再構築を追加した。
- public topic / private channelのContext docsへ署名済みrecordとstate pointerを保存し、docsからtopology viewをhydrationするApp APIを追加した。
- SQLite / memory storeへ削除・再構築可能な`dome_connection_projection_cache`を追加した。
- desktop runtime / Tauri IPC / generated TypeScript contractと、4方向Connection管理panelを追加した。
- 3 ownerのA–B / B–C接続、A–C cycle拒否、restart復元、revoke分割を`desktop_smoke_metaverse_dome_connections`で固定した。
- endpoint owner間のblock入力で未accept proposalを`owners_blocked`として自動破棄し、block解除後も復元しないreconciliationを追加した。
- accept直前にも双方向blockを再検査し、競合するblockを観測した場合はConnection recordを保存せずproposalをterminal discardするようにした。

## 境界

- 隣接Dome sceneのprefetch、connection zoneの開閉、avatar transitionはIssue #790。
- block source、visitor access再検証、presence / spatial audioはIssue #795。
- componentはcanonical entityとして保存せず、global座標も定義しない。

## 検証記録

- `cargo test -p kukuri-core dome_connections`: pass（8 tests）
- `cargo test -p kukuri-app-api dome_connections`: pass（2 tests）
- `cargo test -p kukuri-store`: pass（110 tests）
- `cargo xtask scenario desktop_smoke_metaverse_dome_connections`: pass（9 steps）
- `cargo xtask check`: pass
- `cargo xtask test`: pass（Rust workspace 627、harness 21、desktop 884）
- `cargo xtask desktop-ui-check`: pass（Storybook build、browser 44、visual 14を含む）
- `cargo xtask tauri-check`: pass
- `cargo xtask e2e-smoke`: pass
- `cargo xtask oversized-files`: pass（既存baseline warningのみ）
- `git diff --check`: pass

### 2026-08-29 block proposal follow-up

- `cargo test -p kukuri-app-api dome_connections`: pass（6 tests）
- `cargo test -p kukuri-core dome_connections`: pass（9 tests）
- `cargo xtask rust-test`: pass（Rust workspace 694、harness 22、doc tests）
- `cargo xtask check`: pass
- `cargo xtask test`: pass（Rust workspace 694、harness 22、desktop 915）
- `cargo xtask ipc-types --check`: pass
- `cargo xtask oversized-files`: pass（既存baseline warningのみ）
- `git diff --check`: pass
