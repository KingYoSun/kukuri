# Issue #927: Preset取得待ちによるDome一覧失敗

## 固定範囲と原因

- Scope revision `2026-09-08-872-H-04-v1`、区分C、before `ac9946d66aaf197204f669a10701fac8a13877e9`。
- Context replicaのroom rowとauthor replicaのPreset state/blobは別々に到着する（ADR 0035/0036）。`list_game_rooms_scoped`はroom row到着だけで`get_dome_hosting`を呼び、Preset取得が`None`の段階も一覧全体のerrorにしていた。
- 既存実iroh testはpeer ticket相互import→Dome作成→受信側room発見→event送受信を検証する。初回一覧のPreset未到達で141行がpanicし、event送信へ到達しなかった。fixtureの置換やwait/assertionの緩和は行わない。
- 修正はscope/mute/Active判定後、各DomeのPresetが取得・検証できるか確認し、`None`だけその回の一覧から除外する。保存rowを削除せず次回refreshで再評価。検証errorは`?`で伝播し、不正reference/signatureを取得待ちへ落とさない。通常gameはそのまま返す。
- `get_dome_hosting`とowner authorityの契約、wire/schema/署名形式、P2P三経路は変更しない。既存hosting_viewでも同Presetを検証するためcache済みの読み取りは重なるが、このfixへ構造整理を混ぜない。

## 作業とAC / INVAR

| 作業 | 対応 | 証拠 |
| --- | --- | --- |
| T1 原因と変更前再現 | AC-1、INV-1/2、TR-1 | Nightly2件、既存game.rs141のWindows再現、source call path、下記新規失敗test |
| T2 最小修正と回帰 | AC-2/3、INV-2/3、TR-1/2/3 | game.rsの取得待ち判定のみ、dome_listing4 tests、元iroh test全assertionをそのまま通す |
| T3 検証・独立監査・merge | AC-4、INVAR-1/2 | targeted/slow、必須Rust CI、connectivity、PR head独立監査、merge tree照合 |

## Inventoryと逆引き

- `list_game_rooms` → `list_game_rooms_scoped`の公開/privately scoped入口。Tauri `commands/live_game.rs` → runtime `sync_live_api.rs`、CLI登録handler、harness、testsから呼ばれる。
- `list_game_rooms_scoped` → scope subscriptions / allowed channel / muted author / Active instance filter → `fetch_dome_preset_manifest` → author docs query / blob fetch / state-ref一致 / manifest validation / envelope署名とowner一致 → `get_dome_hosting` → view。
- `fetch_dome_preset_manifest`の既存callerはowner hosting開始、hosting_view、Dome move。これらのerror policyは不変。新規callerはroom listingのみ。
- sensitive sinkは既存docs/blob読み取り・取得とhosting projection。新規の永続mutation・index削除・権限拡張は0。既存scope/filter後に判定し、拒否データを成功として表示しない。
- 再列挙: `rg -n 'list_game_rooms_scoped|list_game_rooms\(|fetch_dome_preset_manifest' crates apps/desktop/src-tauri`。worktreeはCodeGraph indexなし、rootのCodeGraphでsource確認後、worktreeの直接読取と完全symbol検索で補完。

## 変更前後の証拠

- 既存test before: `cargo test -p kukuri-app-api --features iroh-integration-tests metaverse_room_events_replicate_between_iroh_peers -- --nocapture` → exit101、1 FAIL/192 filtered、0.30秒。Nightly33988557825/34056551815と同じ `Dome preset manifest is unavailable`。
- 新規`dome_listing` before（製品未変更）: 4件中2 FAIL/2 PASS。state未到達とblob未到達が同じerrorを再現。不正reference/署名拒否は既にPASS。
- 最小修正後: 同4 tests PASS（0.02秒）、既存実iroh testは無変更でPASS（0.32秒、196 filtered）。新規テストによる件数増加以外の既存assertion/feature gate変更なし。
- `missing_preset_state_keeps_other_rooms_and_recovers_after_delivery` / `missing_preset_blob_keeps_other_rooms_and_recovers_after_delivery`: 通常gameを返し、保存room2件維持、配信後にDome hosting付きで2件へ復帰。
- `mismatched_preset_reference_is_an_error_not_pending` / `invalid_preset_signature_is_an_error_not_pending`: metadata改変・署名内容改変のerrorを維持。
- mandatory: `cargo xtask rust-test`、`cargo xtask app-api-slow-test`、`cargo xtask scenario community_node_public_connectivity`。全結果と監査対象SHAはPR/Issueへ記録し、未完をPASSへ読み替えない。

## 差し戻し

このfix PRのみrevert。データ/schema移行なし。追加の再現testは原因追跡の証拠として保持し、既存testのskip/削除/期待値緩和はしない。
