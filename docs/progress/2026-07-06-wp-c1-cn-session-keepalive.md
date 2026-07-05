# 2026-07-06 WP-C1 CN セッション維持のスケジューラ化(完了報告)

参照: `.claude/plans/2026-07-02-refactoring_master_plan.md` Phase 2 / WP-C1(finding E-1)、
個別プラン `.claude/plans/2026-07-05-fix-c1-cn-session-keepalive.md`、`REFACTORING.md` 検証マトリクス
(desktop-runtime / src-tauri 行)。PR: #473(スケジューラ導入)/ #474(getter 読み取り専用化)。

## 問題(finding E-1)

CN セッション維持(auth 再認証・consent 確認・bootstrap heartbeat・metadata/rendezvous 同期・
connectivity self-heal)は getter(`get_sync_status` / `get_community_node_statuses`)の副作用として
UI の 3 秒ポーリングからのみ駆動されていた。フロントは `document.visibilityState === 'hidden'` で
ポーリングを停止し、アプリはトレイ常駐設計(window close → hide)のため、トレイ常駐中は keepalive が
完全停止し、CN 側の bootstrap 登録(TTL 90 秒)と rendezvous presence(TTL 45 秒)が失効していた。

## 実装した範囲

- **#473(T1〜T3)**: `crates/desktop-runtime/src/community_node/scheduler_support.rs` に常駐
  スケジューラを新設。tick 本体 `run_community_node_session_maintenance_once` は設定済み全 node へ
  既存の冪等な `refresh_community_node_registration_if_due`(deadline ゲート済み・Err→retry state 変換)
  を回し、続けて self-heal を実行するだけで、新しい retry/backoff 機構は持たない。
  - 起動は **opt-in 明示 start**(`start_community_node_session_scheduler`、tick 15 秒 =
    `COMMUNITY_NODE_SESSION_SCHEDULER_TICK_SECONDS`。heartbeat 次回期限がサーバ TTL 90 秒 − 30 秒の
    ため tick < 30 秒が必須)。production は `build_desktop_state` の `Arc::new` 直後に 1 回だけ起動。
    task は `Weak<DesktopRuntime>` 保持(Arc 循環なし・shutdown 忘れでもリークしない)、二重 start は
    no-op、テストは interval 注入版 / tick 直接呼び出しで決定化。
  - `DesktopRuntime::shutdown` 冒頭で abort + join(2 秒 timeout。`AppService::shutdown` と同型)。
    コンストラクタでは起動しないため、スケジューラを使わない既存テスト・harness は挙動不変。
  - failing test 先行(fix 規律): getter を一切呼ばずに heartbeat 継続 / near-expiry 再認証を観測する
    2 本を先置きし、現行コードで赤を実測(`heartbeat_hits` / `verify_hits` が 0 のまま timeout)→
    実装後 green。
- **#474(T4)**: `get_sync_status` を `app_service.get_sync_status()` への純委譲に縮退(registration
  refresh ループと self-heal の両副作用を除去。どちらもスケジューラ tick が引き取り済み)。
  - read-only contract テスト先行: heartbeat 期限 due の状態で getter を呼んでも mock への HTTP・
    session phase / deadline 状態の変化が発生しないことを固定(縮退前に赤を実測)。
  - getter を refresh ドライバに使っていた `tests/community_node/metadata.rs` の 5 テストを
    tick 直接呼び出しへ書き換え(変換規則 1 つ、assert 不変)。テスト削除ゼロ + 新規 3 本
    (keepalive 継続 / near-expiry 再認証 / read-only contract)。connectivity.rs と metadata.rs:820 の
    `get_sync_status` は純読み取り(endpoint id・接続状態の観測)のため無変更。
  - `SyncStatus` のスキーマ・値は不変(views_wire_snapshot 無変更)。TS / mock 側の変更ゼロ
    (desktopApiMock は元から純粋 getter で、real/mock の挙動が一致する方向)。

## スコープ判断(マスタープランからの補正)

- **E-1 の訂正**: 副作用付き getter は `get_sync_status` だけでなく `get_community_node_statuses` にも
  存在する(UI は両方を 3 秒毎に呼ぶ)。C1 はマスタープラン記載どおり前者のみを読み取り専用化し、
  **statuses 側の副作用は意図的に残置**(可視時の即時 establish UX と statuses 駆動テスト 6 本の維持。
  除去は Q2 の CommunityNodeSessionManager 統合と同時に行う)。トレイ常駐時は両 getter とも停止する
  ため、この段階案でも fix は成立する。
- **rendezvous TTL 45 秒ギャップはスコープ外**: rendezvous 更新は heartbeat 便乗(実効約 60 秒毎)の
  ため、フォアグラウンドでも毎サイクル約 15 秒失効している既存問題。読者は他クライアントの heartbeat
  応答のみで、bootstrap seed(TTL 90 秒)がフラット合成で併載されるため実害は限定的。修正は挙動改善に
  当たるため Q2 のイベント push 基盤導入時か別 issue で扱う。
- keepalive が使う HTTP は既存 endpoint のみ(auth challenge/verify・consents・bootstrap
  heartbeat/nodes・rendezvous heartbeat)。cn-user-api 契約(凍結境界)への変更なし。

## 検証

- `cargo xtask rust-test`(両 PR)/ `cargo xtask scenario community_node_public_connectivity`(両 PR)/
  `cargo xtask tauri-check` + `cargo xtask e2e-smoke`(#473)すべて green。
- **Windows 実機確認(受入条件)**: ローカル CN(compose)+ dev 起動で認証 → ウィンドウを閉じて
  トレイ常駐 5 分。`cn_bootstrap.peer_registrations` の `last_seen_at` が正確に 60 秒間隔で前進し
  `expires_at > NOW()` を維持(fix 前は約 90〜120 秒で失効する挙動)。クライアント側でも
  `refreshing community-node bootstrap heartbeat` が `next_due_at` ちょうどに出続けることを確認。
  タイムスタンプ付き記録は PR #474 のコメントに残した。

## 残課題(別 WP)

- Q2: `get_community_node_statuses` の読み取り専用化、CN セッション状態の
  CommunityNodeSessionManager への統合、イベント push 基盤(ポーリングの fallback 格下げ)。
- CN 用 HTTP client の timeout 未設定(tick 内 HTTP が接続タイムアウトまでブロックしうる点は
  現行 getter 駆動と同じ挙動のまま)— Q2 の typed 化と合わせて検討。
