# Issue #857 Community Node consent fixed-surface rerun

## 結論

#857 の再監査で固定した残件を実装し、Community Node 同意境界を有限の監査面で閉じた。設定済み Node、active local consent、公開 current required policy bundle の順に確認する共通 preflight を追加し、保存 token、manual Refresh、report、起動時 connectivity、Dome hosting が同じ判定を利用する。

未同意・撤回・同一 version で異なる snapshot の場合、公開 policy 取得以外の認証、consent status、heartbeat、bootstrap、rendezvous、report 本文送信、保護 API は開始しない。Node 設定と保存済み `resolved_urls` は保持するが、Node 由来 relay / seed は明示的な session 成立後にだけ runtime へ適用する。

## 実装

- `consent_preflight_support.rs` に副作用のない共通 preflight を追加した。URL 正規化、設定 membership、active local consent、公開 policy catalog、required 文書の `(policy_slug, policy_version, policy_snapshot_revision)` 完全一致を順に検証する。
- `ensure_community_node_session` は保存 token の有効期限にかかわらず preflight を先行する。不成立時は token 読み込み前に `Idle`、再同意状態、Node connectivity 無効化へ遷移する。manual Refresh は同じ処理の強制 refresh mode を利用する。
- report は endpoint 形式・設定 membership・same-origin 検証後、本文構築・POST より前に preflight する。policy 更新直後でも旧 snapshot のまま本文を送らない。
- runtime 起動時は保存済み Community Node relay / seed を Iroh の初期入力へ渡さない。利用者が設定した discovery seed は維持し、server consent を含む session 成立後に既存 scheduler から Node connectivity を再適用する。
- session record がない status は常に `Idle` とし、保存 token や `resolved_urls` から `Ready` を合成しない。
- Settings の Refresh は既知の不同意・撤回・再同意待ちでは保護 refresh を呼ばず、既存の対象 Node 同意ダイアログを開く。refresh 中に policy 更新を検出した場合も同じダイアログへ戻す。
- caller のない `get_community_node_consent_status` / `getCommunityNodeConsentStatus` IPC を runtime、Tauri command / 登録、TypeScript interface / implementation、mock から削除した。互換 alias は残していない。
- Dome hosting は共通 preflight の判定本体だけを共有し、既存の target / consent error contract と固定済み 9 application command の挙動を維持した。

## テスト先行の再現

実装前に、次の不足を回帰テストで失敗させた。

- session record のない status が保存状態から `Ready` を合成した。
- 有効な保存 token が公開 policy preflight を迂回した。
- 同一 version・異 snapshot の policy 更新後も report 本文が POST された。
- 保存済み Node relay / seed が runtime 起動直後から有効になった。
- Settings の Refresh が同意ダイアログへ戻らなかった。

修正後は未同意、撤回、同一 version・異 snapshot、current consent の各 fixture で policy、auth challenge / verify、consent status、heartbeat、bootstrap、rendezvous、report POST の hit 数を個別に固定した。

## 固定 inventory

- consent-status IPC 削除後の `commands::community_node::*` 登録: 47 件。
- 既存 Dome hosting application command: 9 件。共通 GET / request helper の preflight contract 6 件で未同意、撤回、policy 更新、未設定、current consent、401 再認証を維持。
- 通常 UI: Settings Refresh と既存同意ダイアログ。
- background / startup: runtime 初期化、session scheduler、manual Refresh、report。
- 削除対象名 `get_community_node_consent_status|getCommunityNodeConsentStatus`: 0 件。
- 上記固定面の未分類経路: 0 件。固定面外の将来 route、独自 client、policy cache 最適化は #857 の Close 条件へ追加しない。

## 検証

- Community Node 対象 test: session 7件、report 9件、config 13件、metadata 10件、Dome 6件、admission 7件、connectivity 6件、index query 11件、tester feedback 3件、trust relation 4件、scheduler 5件、合計 85件すべて成功。
- `cargo xtask doctor`: 成功。
- `cargo xtask check`: format、clippy `-D warnings`、Tauri check、frontend lint / typecheck すべて成功。
- `cargo xtask oversized-files`: 成功（既存 baseline warning のみ。変更対象 `runtimeApi.ts` は縮小）。
- `cargo xtask ipc-types --check`: 成功。
- `cargo xtask rust-test`: workspace 769件、harness 22件、doc test すべて成功（workspace 3件 skip）。
- `cargo xtask test`: Rust 769件、harness 22件、frontend 1089件すべて成功（Rust 3件 skip）。
- `cargo xtask desktop-ui-check`: lint、typecheck、unit 1089件、Storybook build、browser 58件、visual 14件すべて成功。
- `cargo xtask tauri-check`: 成功。
- `cargo xtask e2e-smoke`: `desktop_smoke_post_persist` 6 step 成功。
- `cargo xtask scenario community_node_public_connectivity`: 15 step 成功。
- `git diff --check`: 成功。

## Scope boundary

#860 が所有する Community Node server 側の heartbeat、auth verify、trust pull、DB / API contract は変更していない。#857 は本書の固定 inventory と検証結果をもって Close 可能であり、親 #853 の残否は #860 を含む他の既知 child 条件だけで判定する。

関連: #853、#857、#860
