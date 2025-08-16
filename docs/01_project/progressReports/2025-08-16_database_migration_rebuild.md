# データベースマイグレーション完全再構築
作成日: 2025年08月16日

## 概要
E2Eテストの認証機能で発生していたデータベースエラーを解決するため、マイグレーションを完全に再構築した。

## 問題の背景
### 発見された問題
1. **二重のデータベース接続プール**
   - `Database::initialize`と`ConnectionPool::new`で別々のプールが作成
   - 片方のみでマイグレーションが実行されていた

2. **テーブルスキーマの不整合**
   - 旧アーキテクチャ: `profiles`テーブル
   - 新アーキテクチャ: `users`テーブル（`npub`カラム必須）
   - `reactions`テーブルのカラム名不一致

3. **マイグレーション履歴の不整合**
   - 古いマイグレーション記録が残存
   - 新旧混在による互換性問題

## 実施内容

### 1. マイグレーションの完全リセット
```bash
# 既存マイグレーションとデータベースを削除
rm -rf migrations/*.sql
rm -rf data/kukuri.db
rm -rf .sqlx
rm -f "C:/Users/kgm11/AppData/Roaming/com.kukuri.app/kukuri.db"
```

### 2. 新しい統合マイグレーションの作成
```bash
# マイグレーションファイル生成
DATABASE_URL="sqlite:data/kukuri.db" sqlx migrate add -r initial_schema
```

#### 作成したスキーマ（主要部分）
```sql
-- ユーザーテーブル（新アーキテクチャ対応）
CREATE TABLE IF NOT EXISTS users (
    npub TEXT PRIMARY KEY NOT NULL,
    pubkey TEXT NOT NULL UNIQUE,
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- リアクションテーブル（カラム名修正）
CREATE TABLE IF NOT EXISTS reactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_event_id TEXT NOT NULL,
    reactor_pubkey TEXT NOT NULL,
    reaction_content TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    UNIQUE(reactor_pubkey, target_event_id)
);
```

### 3. SQLxオフライン用メタデータ生成
```bash
# データベース作成とマイグレーション実行
DATABASE_URL="sqlite:data/kukuri.db" sqlx database create
DATABASE_URL="sqlite:data/kukuri.db" sqlx migrate run

# オフライン用メタデータ生成
DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare
```

## 結果
- ✅ `pnpm tauri dev`でデータベース接続成功
- ✅ 認証機能でのデータベースエラー解消
- ✅ SQLxオフラインモード対応完了

## 残課題
### E2Eテストの問題
- **症状**: `Tauri API not available when running "execute/sync"`エラー
- **原因**: WebDriverとTauri APIの統合問題
- **対応**: 別途調査・対応が必要

## 学習事項
### SQLxのマイグレーション管理
1. **自動生成の限界**
   - `sqlx migrate add`はファイルテンプレートのみ生成
   - スキーマ定義は手動で記述必要

2. **オフラインモード**
   - `.sqlx`ディレクトリにメタデータを保存
   - CIでのビルドに必須
   - Gitにコミットする必要がある

3. **マイグレーション履歴**
   - `_sqlx_migrations`テーブルで管理
   - 不整合時は完全リセットが必要な場合がある

## 次のステップ
1. E2EテストのTauri API問題の調査
2. WebDriverIO設定の見直し
3. テスト環境の改善

## 関連ファイル
- `/kukuri-tauri/src-tauri/migrations/20250816044844_initial_schema.up.sql`
- `/kukuri-tauri/src-tauri/.sqlx/`（メタデータディレクトリ）