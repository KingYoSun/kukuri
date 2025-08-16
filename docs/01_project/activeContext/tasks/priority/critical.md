# 最重要タスク

**最終更新**: 2025年08月16日
**最大3個まで**

## 1. Distributed Topic Tracker実装（DHT基盤のP2P Discovery）

**目標**: BitTorrent Mainline DHTを用いた完全分散型ピア発見システムの実装
**優先度**: 最高（アーキテクチャの根幹変更）
**期限**: 2025年08月30日目標

### 次のアクション
- [ ] distributed-topic-tracker依存追加とCargo.toml更新
- [ ] P2Pモジュールにbootstrap.rs作成（AutoDiscovery統合）
- [ ] 共有シークレット管理とキーローテーション実装
- [ ] iroh-gossipとのイベントハンドリング統合
- [ ] DHT失敗時のフォールバック機構実装
- [ ] ユニットテストとE2Eテスト（Docker Compose）作成

### 背景
Cloudflare Workers依存を排除し、真の分散性を実現。検閲耐性とスケーラビリティを大幅に向上させる戦略的変更。詳細設計: `docs/01_project/activeContext/distributed-topic-tracker-plan.md`

---

## 2. ユニットテスト・統合テストの強化

**目標**: DHT実装を含むテストカバレッジの確保
**優先度**: 高（品質保証の基盤）
**期限**: 2025年09月05日目標（DHT実装後）

### 次のアクション
- [ ] DHT統合のモックテスト作成
- [ ] 認証機能のユニットテスト作成
- [ ] Service層の統合テスト実装
- [ ] Storeのモックを使用したフロントエンドテスト強化
- [ ] カバレッジ目標の設定（最低70%）

### 背景
DHT実装と並行してテスト体制を強化。分散システムの信頼性確保が重要。

---

## 3. v2アーキテクチャPhase 7: DHT統合後の最適化

**目標**: DHT統合を踏まえた新アーキテクチャの完全稼働
**優先度**: 中
**期限**: 2025年09月10日

### 次のアクション
- [ ] P2PService DHT統合（AutoDiscoveryGossip連携）
- [ ] OfflineService Repository層統合（11メソッドが基本実装のみ）
- [ ] DHT統計情報の追加（ピア数、レイテンシ）
- [ ] EventService DHT最適化

### 背景
DHT実装により、P2Pモジュール全体の再設計が必要。v2移行と統合して一貫性のあるアーキテクチャを構築。