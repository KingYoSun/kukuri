# cn-cli 統合（bootstrap / relay）計画

**作成日**: 2026年01月22日  
**最終更新日**: 2026年02月13日  
**対象**: `./kukuri-community-node/crates/cn-cli` → `./kukuri-community-node`

## ゴール（要件）

- `cn-cli` を `kukuri-community-node` に統合し、`bootstrap` / `relay` としてサービス化できる
- 既存 CLI の有用なサブコマンドは維持しつつ、daemon 起動にも対応する

## 統合方針（提案）

1. **Cargo workspace 化**
   - `kukuri-community-node` を Rust workspace にし、既存 `cn-cli` の crate を移植
2. **ライブラリ抽出**
   - `bootstrap` / `relay` に相当する処理を `cn-bootstrap` / `cn-relay` の library に分離
   - CLI は薄いラッパとして残す（`cn-cli` から呼ぶ）
3. **サービス起動方式（現行仕様）**
   - `cn bootstrap` / `cn relay` がフォアグラウンド常駐起動コマンド（daemon サブコマンドは持たない）
   - `cn bootstrap daemon` / `cn relay daemon` は現行実装では未サポート（必要になった場合は互換方針を別途定義する）
4. **Compose 対応**
   - profile `bootstrap` / `relay` の service として起動できるよう、設定（env/DB/鍵/ポート）を整備

## 注意点（補完）

- 既存 `kukuri-tauri` から `cn-cli` を呼んでいる箇所がある場合、互換性を壊さない移行手順を用意する
- 署名鍵（Node Key）を CLI で生成/ローテーションできるようにし、監査ログを残す
