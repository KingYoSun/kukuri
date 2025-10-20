# Phase 4 DRY 適用フォローアップレポート
最終更新日: 2025年10月20日

## 概要
- `modules/event` / `modules/p2p` のテスト支援コードを `application/shared/tests` に移設し、Event/P2P 双方で共通フィクスチャ・モック・ロギングを再利用できる構成に統一。
- EventPublisher と DefaultTopicsRegistry を shared レイヤーに集約し、EventService / EventManager 間でのイベント生成および既定トピック管理の重複を解消。
- フロントエンドの Zustand ストアに共通 `withPersist` テンプレートと `config/persist.ts` を導入し、Map を扱うストアで `createMapAwareStorage` を適用。persist 設定の一元管理とテスト用モック（`setupPersistMock`）を整備。

## 変更詳細
- `application/shared/tests/{event,p2p}/` を新設し、既存テストから再エクスポートする形で Rust 側の重複ユーティリティを置き換え。
- `application/shared/nostr/publisher.rs` に EventPublisher を移動し、EventService/EventManager から `shared::nostr` を介して参照。`application/shared/default_topics.rs` で DefaultTopicsRegistry を shared 化。
- Zustand ストア（`authStore`, `draftStore`, `offlineStore`, `p2pStore`, `topicStore`）を `withPersist` + `create{Auth,Draft,Offline,P2P,Topic}PersistConfig` 経由の構成に更新し、テストでは `setupPersistMock` を利用して localStorage モックを簡素化。
- `docs/01_project/activeContext/tasks/status/in_progress.md` の Phase 4 サブタスク進捗を更新。

## 実施テスト
- TypeScript ストア周辺: `pnpm test --run src/stores`
- Rust 全体: `./scripts/test-docker.ps1 rust`
