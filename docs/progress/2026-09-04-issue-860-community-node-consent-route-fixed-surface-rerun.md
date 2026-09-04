# Issue #860 Community Node consent route fixed-surface rerun

## 結論

#860 の再監査で固定した server 側の残件 3 経路を、既存の current consent contract へ揃えた。bootstrap heartbeat は peer mutation 前、認証済み trust pull は subscriber audience 選択前に current consent を要求する。auth verify は認証と peer 広告を分離し、bootstrap peer の登録を同意後 heartbeat に一本化した。

公開 listener 55 method-path と、別 bind の IAP 管理 listener 9 method-path を同じ分類基準で再集計した。今回の 3 経路以外に同種の未対処経路はなく、将来 route、既存 peer の即時 purge、匿名 Public trust pull、IAP 管理 listener を追加の Close 条件にはしていない。

## 実装

- `POST /v1/bootstrap/heartbeat` は bearer 認証後かつ endpoint 検証・peer row 更新前に既存 `require_consents` を実行する。未同意または旧 snapshot は `403 CONSENT_REQUIRED` となり、peer row を作成・更新しない。
- `POST /v1/auth/verify` から bootstrap peer の prune / upsert を除去した。challenge 消費、admission、subscriber 有効化、`endpoint_id` の検証と JWT claim binding は維持し、wire 上の `addr_hint` も変更していない。
- `GET /v1/trust/pull/{pubkey}` は匿名または bearer 認証不成立なら従来どおり `Public` audience を返す。有効な bearer で `SubscribedNodes` audience を要求する場合だけ current consent を検証する。
- policy snapshot 更新 fixture と heartbeat helper を integration test support に追加し、通常の policy sync と公開 API を通じて旧 snapshot / current snapshot の境界を検証する。

## テスト先行の再現

実装前に追加した回帰 contract は、次の不足でそれぞれ失敗した。

- auth verify 直後に bootstrap peer row が 1 件作成され、他 subscriber の seed 候補になった。
- 未同意の bootstrap heartbeat が `200` となり、peer row を変更した。
- 未同意の有効 bearer による trust pull が `200` となり、`SubscribedNodes` audience を返した。

修正後は、auth verify 直後に peer row と seed 候補がないこと、current consent 後の最初の heartbeat で初めて登録されることを固定した。heartbeat は未同意・旧 snapshot で `403` かつ DB 不変、current consent で登録成功を検証した。trust pull は匿名 Public、未同意・旧 snapshot bearer の `403`、current consent bearer の `SubscribedNodes` を検証した。

## 固定 route inventory

- 公開 API router: 46 method-path。
- manifest router: 9 method-path。
- 公開 listener 合計: 55 method-path。
- 別 bind の IAP 管理 listener: 9 method-path。
- 今回変更した route 登録: 0 件。固定数は変更前後で同一。
- 今回の対象: heartbeat mutation、auth verify peer side effect、認証済み trust pull subscriber audience の 3 件。
- 上記固定面の未分類・未対処経路: 0 件。

## 検証

- 対象 integration test: `contract_auth` 18 件、`contract_bootstrap` 9 件、`trust_relation` 6 件、すべて成功。
- `cargo xtask doctor`: 成功。
- `cargo xtask oversized-files`: 成功（既存 baseline warning のみ）。
- `cargo xtask cn-check`: 成功。
- `cargo xtask cn-test`: Community Node 全 package / integration / doc test 成功。
- `cargo xtask cn-e2e`: 成功。
- `cargo xtask rust-test`: workspace 769 件、harness 22 件、doc test すべて成功（workspace 3 件 skip）。
- `cargo xtask check`: format、clippy `-D warnings`、Tauri check、frontend lint / typecheck すべて成功。
- `cargo xtask test`: Rust 769 件、harness 22 件、frontend 1089 件すべて成功（Rust 3 件 skip）。
- `cargo xtask e2e-smoke`: `desktop_smoke_post_persist` 6 step 成功。
- `cargo xtask scenario community_node_public_connectivity`: 15 step 成功。
- `git diff --check`: 成功。

## Scope boundary

#857 が所有する desktop client の consent preflight、UI、report、runtime connectivity は変更していない。今回の固定条件は server 側 3 経路だけであり、90 秒 TTL で失効する既存 peer の即時削除や、将来の独自 client / route を再帰的な完了条件へ追加しない。本書の実装・contract・固定 inventory により #860 は Close 可能であり、#853 は既存の固定完了条件と child 状態だけで最終判定する。

関連: #853、#857、#860
