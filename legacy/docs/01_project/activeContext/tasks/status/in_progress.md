[title] 作業中タスク（in_progress）

最終更新日: 2026年03月10日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク
- P2Pトピック同期の不具合調査と修正（直結 multi-peer / IPv6 条件でのリアルタイム反映、reply/thread 導線の差分切り分け）
- Community Node 実機UXの未解決項目の実機確認（profile 伝播、Windows reload stability、Admin UI runtime 表示）
- Community Node relay 公開構成の VPS + WireGuard edge 化（Cloudflare Tunnel 依存の除去、Home bind 制御、運用スクリプト整備）
- Linux-first rebuild foundation の実装（`next/` workspace、`cargo xtask`、core/store/transport/harness、desktop shell、`next-fast.yml` / `next-nightly.yml`）

### Community Node 実機UXの未解決項目
- 公開 Community Node relay 警告の切り分け
  - 実機検証（2026年03月10日）: `https://api.kukuri.app` を設定したデスクトップ実機で、peer 間接続、投稿伝播、realtime mode での timeline 自動更新は確認できた。
  - 実機ログ（2026年03月09日 16:17:13Z / 16:17:15Z）: `wss://api.kukuri.app/relay` への接続は `HTTP error: 404 Not Found` で失敗し、`No configured Nostr relay connected within 3s` warning も出ている。ただし P2P 経路の post 伝播自体は継続している。
  - 実装・検証（2026年03月10日）: 原因は 2 つあり、Community Node 側の bootstrap descriptor seed が `BOOTSTRAP_DESCRIPTOR_*` 未設定時に `localhost` 固定へ落ちていたことと、クライアント側が bootstrap node でも `base_url + /relay` fallback を使っていたことだった。`cn-core::admin` で descriptor `http/ws` を `PUBLIC_BASE_URL` / `RELAY_PUBLIC_URL` から自動導出しつつ既存 bootstrap config の descriptor endpoint を部分更新するよう修正し、Tauri 側では bootstrap node の `base_url + /relay` fallback を廃止し、Community Node 認証時の relay 同期失敗も best-effort 化した。`./scripts/test-docker.ps1 rust` / `e2e-community-node` と `gh act --workflows .github/workflows/test.yml --job format-check` / `native-test-linux` / `community-node-tests` は PASS。
  - 未解決点: 修正後コードで `https://api.kukuri.app` 実機を再検証していないため、公開 descriptor が `wss://relay.kukuri.app/relay` を返し、認証 warning が解消したこと自体は未確認。
  - 次アクション: `api.kukuri.app` の live-path で Community Node 認証を再実施し、relay status が `relay.kukuri.app` を向くことと `wss://api.kukuri.app/relay` 404 / auth warning が消えることを確認する。
- プロフィール伝播不具合
  - 実機検証（2026年03月10日）: 両クライアントで事前に自分自身のプロフィールをローカル保存した状態から投稿すると、peer 間接続・投稿伝播・realtime timeline 更新は成功する一方、相手側 timeline/thread の表示名・avatar は反映されなかった。
  - テスト監査（2026年03月10日確認）: `community-node.profile-propagation.spec.ts` は local docker の `http://127.0.0.1:18080` / `ws://127.0.0.1:18082/relay` を前提に、profile update アクション実行後の `getAuthSnapshot()` と `waitForPeerHarnessSummary()` だけを見ている。今回の「profile update なしで既存プロフィールを解決する」実機シナリオは担保していない。
  - 実装・検証（2026年03月10日）: 投稿時に保存済みローカル profile metadata を topic へ自動伝播するよう Rust 側を修正し、`community-node.profile-resolution.spec.ts` で profile update なしの 2-client 実機相当 E2E を追加した。受信側 timeline/thread の display name / avatar を直接 assertion し、`./scripts/test-docker.ps1 rust` / `ts` / `e2e-community-node` と `gh act --workflows .github/workflows/test.yml --job format-check` / `native-test-linux` / `community-node-tests` は PASS。
  - 未解決点: 修正後コードで `https://api.kukuri.app` 実機を再検証していないため、公開 Community Node 上でプロフィール伝播が解消したこと自体は未確認。
  - 次アクション: `https://api.kukuri.app` を使った実機再検証で profile 伝播を確認し、失敗が残る場合は relay warning と切り分けて追加調査する。
- Windows Tauri リロードクラッシュ
  - 現状: `community-node.reload-stability.spec.ts` と関連 Rust tests は PASS。`IrohGossipService` の peer hint 再joinは idempotent 化済み。
  - 検証（2026年03月09日）: `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.reload-stability.spec.ts ./scripts/test-docker.ps1 e2e-community-node` を再実行し PASS。
  - 未解決点: 上記は Linux desktop E2E であり、実機 Windows で `iroh-quinn ... PoisonError` が再発しないことの確認は未了。
  - 次アクション: Windows 実機で連続 reload と panic log 監視を行う。
- Admin UI connected users 表示不整合
  - 現状: contract test `node_subscriptions_list_does_not_treat_active_subscriptions_as_current_connected_users` と Admin Console page tests は PASS。
  - テスト状況（2026年03月09日確認）: `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs` と `kukuri-community-node/apps/admin-console/src/pages/BootstrapPage.test.tsx` に coverage はあるが、Admin UI 実機相当 E2E は未整備。
  - 未解決点: 実機 Admin UI で current connected users 表示へ置換されたことの確認が未了。
  - 次アクション: 接続・切断を伴う実機 UI 確認を行う。
- Admin UI health 不具合
  - 現状: build 失敗、services API、compose profile の各修正は反映済み。
  - テスト状況（2026年03月09日確認）: `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs` と `kukuri-community-node/apps/admin-console/src/pages/ServicesPage.test.tsx` に coverage はあるが、health 遷移を確認する実機相当 E2E は未整備。
  - 未解決点: 実機 stack で `trust` が `healthy` へ遷移することの確認が未了。
  - 次アクション: `trust` profile を含む実機 stack で health を確認する。

### Community Node relay 公開構成の VPS + WireGuard edge 化
- 現状: `VPS + WireGuard edge` 方針、bind 制御、セットアップスクリプト、運用ドキュメント整備までは完了。
- 対応メモ（2026年03月09日）: `kukuri-community-node/docker-compose.yml` の `relay -> cn-iroh-relay` 依存を外し、`.env.example` の既定を `RELAY_IROH_RELAY_MODE=default` に修正。`docker compose build` / `docker compose up -d` と `--profile bootstrap` 付き起動が通ることを確認した。
- テスト状況（2026年03月09日確認）: `docs/03_implementation/community_nodes/home_vps_wireguard_edge.md` と関連セットアップスクリプトは整備済みだが、VPS + WireGuard edge を対象にした実機相当の自動テストは未整備。
- 未解決点: `cn-iroh-relay` を `7842/udp` 含みで本番相当に公開する経路と、実機 stack での gossip join / health 遷移確認が未了。
- 次アクション: relay 本番構成を適用し、live stack で疎通と health を確認する。
