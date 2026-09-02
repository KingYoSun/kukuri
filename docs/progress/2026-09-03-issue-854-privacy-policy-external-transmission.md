# Issue #854 プライバシーポリシーと外部送信表示

参照: Issue #854（親: #853）

実施日: 2026-09-02〜2026-09-03

## 完了内容

- クライアント、P2P／DHT／relay、自動更新、Community Node、診断レポートについて、保存先、送信先、送信項目、目的、保持・削除境界を `docs/legal/app-data-flow-inventory.md` に突合表として固定した。
- プライバシーポリシーと利用規約を legal bundle version 4、施行日 2026-09-02 とし、日本語正文、参考訳の位置付け、管理主体・窓口、変更履歴を揃えた。
- `docs/legal/external-transmission-notice.md` を追加し、GitHub Releases、Mainline DHT、P2P 接続相手、relay、Community Node を送信先・契機・目的・項目・保持主体ごとに整理した。
- 公開プロフィール、フォロー、投稿、リアクションを端末内だけとする旧記述を廃止し、public／private／DM の audience と P2P 複製、第三者 copy／cache の完全回収を保証できない境界を明記した。
- 診断レポートに含まれ得る接続・更新・Community Node 状態と、秘密鍵、認証 token、DM 本文等を既定で含めない境界を具体化した。現行アプリが既定で行動分析、広告 tracking、自動 crash report 送信を行わないことも明記した。
- app consent を v4 へ更新し、既存 v3 同意では network runtime を開始せず再同意を要求する。施行日、日本語正文、重要変更の概要を同意画面と設定画面へ表示する。
- 配布主体・窓口を `apps/desktop/src-tauri/distribution/legal.json` から同意 IPC と ja／en／zh-CN 文面へ注入し、配布物固有値と operator-neutral な UI 実装を分離した。
- 設定のリリース情報に、GitHub 更新確認、P2P／DHT／relay、診断レポートの外部送信表示を追加した。Community Node 固有の文書は既存どおり各 manifest から動的に表示する。

## 契約

- failing-first: canonical legal document test を v4 と実データフローの必須句へ更新し、旧 v3 文書で失敗することを確認してから文書・runtime を更新した。
- Tauri の updater endpoint と30分間隔、配布 Community Node URL が外部送信表示に含まれることを backend contract で固定した。
- distribution legal metadata の値が canonical privacy／external transmission 文書と一致し、同意 status から frontend へ渡ることを固定した。
- `LegalDocumentView.test.tsx` と `App.test.tsx` で v4、施行日、正文／参考訳、重要変更、再同意 payload、外部送信表示を固定した。
- `xtask` の operator-neutrality allowlist は配布専用 `distribution/legal.json` だけを許可し、product source への固有窓口の直書きは禁止し続ける。

## 検証

- `cargo xtask check`
- `cargo xtask test`（workspace 742 tests、harness 22 tests、frontend 140 files / 1074 tests）
- `cargo xtask tauri-check`
- `cargo xtask ipc-types --check`
- `cargo xtask desktop-ui-check`（Storybook build、browser 58 tests、visual smoke 14 tests）
- `cargo xtask scenario community_node_public_connectivity`（14 steps）
- `cargo xtask e2e-smoke`（desktop_smoke_post_persist、6 steps）
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml state::tests -- --nocapture`（16 tests）
- locale JSON 27 files の parse、targeted legal／consent／i18n tests 73件
- `git diff --check`

## 境界

- Phase A は確認できる現行挙動との整合を対象とし、専門家レビュー済みであることや将来の事業化要件の完成を Issue #854 の完了条件にしない。
- 第三者 Community Node の保持・問い合わせ・外部送信は各 Node の manifest と公開文書に従い、kukuri 運営者と Node 運営者が同一であるとは限らない。
- 年齢自己申告の意味・保存範囲は変えていないため `AGE_ATTESTATION_VERSION` は1のままとし、legal bundle v4 の文書だけを再同意対象とする。
