# 2026-07-28 #572 topic rendezvous refresh の heartbeat 便乗からの独立

参照: `docs/progress/2026-07-06-wp-c1-cn-session-keepalive.md`(A-4 で記録・先送りされた
rendezvous TTL 45 秒ギャップ。本変更でその先送りをクローズする)

## 問題

topic rendezvous presence のサーバ TTL は 45 秒(`cn-core` の `TOPIC_RENDEZVOUS_TTL_SECONDS`)
だが、クライアントの rendezvous refresh は bootstrap heartbeat 成功後の便乗呼び出しのみで、
heartbeat の実効周期は「サーバ TTL 90 秒 − 30 秒 = 約 60 秒」。そのため毎サイクル約 15 秒間
presence が失効していた(#572)。bootstrap seed(TTL 90 秒)のフラット合成で実害は限定的だが、
rendezvous 経由の topic peer 発見だけに頼る経路では失効窓の間 candidate から漏れる。

## 実装した範囲(クライアント側のみ。cn-user-api 契約は凍結境界のため無変更)

- **fix 規律**: 先に characterization test
  `topic_rendezvous_refresh_fires_between_bootstrap_heartbeats`
  (`crates/desktop-runtime/src/tests/community_node/scheduler.rs`)を置き、修正前 red
  (heartbeat の合間に rendezvous POST が発火しない)を確認してから修正した。テスト支援として
  mock CN に `/v1/rendezvous/topics/heartbeat` route と `MockRendezvousCommunityNodeState`
  (rendezvous_hits カウンタ)を追加(`src/tests/support/mock_cn.rs`)。
- **独立 deadline 管理**(`crates/desktop-runtime/src/community_node/`)
  - `CommunityNodeSessionState` に `rendezvous_refresh_deadline`(default 0 = 即時 due)を追加。
  - `refresh_topic_rendezvous_with_token` が成功時にサーバ返却 `expires_in_seconds`(45 秒。
    従来はデコード後破棄)−
    `COMMUNITY_NODE_TOPIC_RENDEZVOUS_REFRESH_MARGIN_SECONDS`(20 秒)で次回期限を更新。
    heartbeat の「`expires_at` − マージン」管理と同型。便乗呼び出し(heartbeat 後 /
    metadata refresh 後)でも同じ場所で bump されるため、直後の冗長 POST は出ない。
  - `refresh_community_node_registration_with_token_if_due_once` の「heartbeat not-due skip」
    分岐で deadline を判定し、due なら独立に refresh する。駆動は既存のセッション維持
    スケジューラ tick(15 秒)のままで、新規タスク・新規設定は追加していない。

## 周期の根拠

マージン 20 秒は tick(15 秒)より大きいことが必須。deadline は「refresh 時刻 + 45 − 20 =
+25 秒」で、tick 量子化により実際の POST が最大 tick 分遅れても +40 秒 < TTL 45 秒に収まる。
失敗時は deadline を bump しない(過去のまま)ため次の maintenance pass で自然に再試行され、
エラーストームは既存の session retry state(30 秒)が抑制する。専用 retry 機構は追加しない。

## 維持した境界(本変更に含まない)

- サーバ側 TTL(`TOPIC_RENDEZVOUS_TTL_SECONDS = 45`)と wire 契約(`expires_in_seconds`)は
  無変更。
- topic subscribe/unsubscribe に連動した rendezvous `joins`/`leaves` ライフサイクル
  (現状どおり `refreshes` のみ)。
- WP-C1 doc に残る Q2 委譲項目(event push 基盤、`CommunityNodeSessionManager` 統合)。
