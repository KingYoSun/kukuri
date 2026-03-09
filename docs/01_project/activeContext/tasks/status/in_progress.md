[title] 作業中タスク（in_progress）

最終更新日: 2026年03月09日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク
- P2Pトピック同期の不具合調査と修正（直結 multi-peer / IPv6 条件でのリアルタイム反映、reply/thread 導線の差分切り分け）
- Community Node 実機UXの未解決項目の実機確認（profile 伝播、Windows reload stability、Admin UI runtime 表示）
- Community Node relay 公開構成の VPS + WireGuard edge 化（Cloudflare Tunnel 依存の除去、Home bind 制御、運用スクリプト整備）

### Community Node 実機UXの未解決項目
- プロフィール伝播不具合
  - 現状: `useP2PEventListener` / profile save toast 修正と `community-node.profile-propagation.spec.ts` の live-path E2E は PASS。
  - 検証（2026年03月09日）: `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.profile-propagation.spec.ts ./scripts/test-docker.ps1 e2e-community-node` を再実行し PASS。
  - 未解決点: 実機 multi-node 構成で success toast のみ表示され、相手側 timeline/thread の表示名・avatar が反映されるか未確認。
  - 次アクション: 実機 multi-node で metadata(kind=0) 伝播を再確認する。
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

### 直結 multi-peer / IPv6 条件で未解決の P2P トピック同期
- リアルタイム差分更新
  - 現状: Community Node 経路では修正済み。`p2p.direct-peer.regression.spec.ts` の 2026年03月09日再実行では `renderedWithoutReload` の期待は既定モード / `E2E_DIRECT_PEER_CONNECT_MODE=direct` の両方で通過し、現行の multi-peer E2E では stale render は未再現だった。
  - 未解決点: 2026年03月02日に再現した IPv6 強制条件の stale render を再確認する専用の実機相当テストはまだない。
  - 次アクション: IPv6-only 強制条件を現行 E2E に再導入して再確認する。
- reply 投稿失敗 / thread 導線不整合
  - 現状: community-node E2E では未再現だが、direct multi-peer / IPv6 条件との差分が未整理。
  - テスト状況（2026年03月09日確認）: `community-node.thread-preview-replies.spec.ts` など Community Node 経路の E2E はあるが、direct multi-peer / IPv6 条件で reply/thread 導線を確認する実機相当 E2E は未整備。
  - 次アクション: `reply_to` キャッシュ解決と thread route 更新の再現条件を分離して確認する。

### Community Node relay 公開構成の VPS + WireGuard edge 化
- 現状: `VPS + WireGuard edge` 方針、bind 制御、セットアップスクリプト、運用ドキュメント整備までは完了。
- テスト状況（2026年03月09日確認）: `docs/03_implementation/community_nodes/home_vps_wireguard_edge.md` と関連セットアップスクリプトは整備済みだが、VPS + WireGuard edge を対象にした実機相当の自動テストは未整備。
- 未解決点: `cn-iroh-relay` を `7842/udp` 含みで本番相当に公開する経路と、実機 stack での gossip join / health 遷移確認が未了。
- 次アクション: relay 本番構成を適用し、live stack で疎通と health を確認する。
