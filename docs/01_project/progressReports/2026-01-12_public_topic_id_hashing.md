# 進捗レポート: 公開トピックIDのハッシュ統一

**日付**: 2026年01月12日  
**作業者**: Codex  
**カテゴリ**: 仕様・実装

## 概要

公開トピックIDを非公開トピックと同じBLAKE3ハッシュ方式に統一し、tauri/cli双方のTopicId生成とデフォルト値を整合させました。

## 実施内容

- tauri/cliのTopicId生成を kukuri:tauri:<64桁hex> へ統一
- デフォルト公開トピックIDをハッシュ値へ更新（kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0）
- CLIの RELAY_TOPICS をハッシュ化して購読に適用
- 旧ID（public / kukuri:tauri:public）の正規化処理を追加
- 既存DBは破棄・再作成前提のため、トピックID変更のマイグレーションは追加せず仕様ドキュメントを更新

## 影響範囲

- P2P/Gossip/DHT のTopicId生成・購読
- デフォルト公開トピックと関連DBレコード
- docker-compose の RELAY_TOPICS デフォルト
