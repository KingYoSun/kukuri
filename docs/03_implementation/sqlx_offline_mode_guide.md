# SQLx オフラインモード実装ガイド

**作成日**: 2025年8月9日

## 概要
SQLxのquery!マクロはコンパイル時にデータベーススキーマを検証する機能を持つが、オフライン環境では動作しない。本ガイドでは、この問題の解決方法と実装方針を記載する。

## 問題の詳細

### エラー内容
```
error: set DATABASE_URL to use query macros online
```

### 発生原因
- SQLxのquery!マクロはコンパイル時にデータベース接続を必要とする
- DATABASE_URL環境変数が設定されていない、またはデータベースに接続できない環境ではコンパイルエラーが発生
- CI/CD環境や他の開発者の環境でビルドが失敗する可能性がある

## 解決方法

### 方法1: sqlx::queryを使用（暫定対応）✅実装済み

#### 実装例
```rust
// Before (コンパイルエラー)
let rows = sqlx::query!(
    r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE kind = 1
    LIMIT ? OFFSET ?
    "#,
    limit,
    offset
)
.fetch_all(pool.as_ref())
.await?;

// After (動作する)
let rows = sqlx::query(
    r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE kind = 1
    LIMIT ? OFFSET ?
    "#
)
.bind(limit)
.bind(offset)
.fetch_all(pool.as_ref())
.await?;

// 手動でのマッピング
for row in rows {
    let event_id: String = row.get("event_id");
    let public_key: String = row.get("public_key");
    let content: String = row.get("content");
    let created_at: i64 = row.get("created_at");
    let tags_json: String = row.get("tags");
    // ...
}
```

#### メリット
- 即座に実装可能
- 環境依存がない
- CI/CDで安定動作

#### デメリット
- コンパイル時の型チェックが失われる
- 手動でのResultマッピングが必要
- カラム名のタイポがランタイムエラーになる

### 方法2: SQLx Offlineモード（推奨）

#### セットアップ手順

1. **SQLxCLIのインストール**
```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

2. **DATABASE_URLの設定**
```bash
# Windows
set DATABASE_URL=sqlite:./kukuri-tauri/src-tauri/data/kukuri.db

# Unix/Mac
export DATABASE_URL=sqlite:./kukuri-tauri/src-tauri/data/kukuri.db
```

3. **スキーマファイルの生成**
```bash
cd kukuri-tauri/src-tauri
cargo sqlx prepare
```

これにより`.sqlx/`ディレクトリにスキーマ情報が保存される。

4. **オフラインモードの有効化**
```toml
# Cargo.toml
[dependencies]
sqlx = { 
    version = "0.8", 
    features = ["runtime-tokio", "sqlite", "offline"] 
}
```

5. **.gitignoreの更新**
```gitignore
# SQLx offline mode
.sqlx/
```

#### CI/CDでの使用

```yaml
# GitHub Actions example
- name: Setup SQLx
  run: |
    cargo install sqlx-cli --no-default-features --features sqlite
    cargo sqlx database setup
    cargo sqlx prepare --check
```

#### メリット
- コンパイル時の型安全性を維持
- IDEの補完が効く
- スキーマ変更の検出が可能

#### デメリット
- 初期セットアップが必要
- スキーマ変更時に再生成が必要
- .sqlxディレクトリの管理が必要

## 実装状況

### Phase 2で実装済み（方法1）
- ✅ `post/commands.rs`: get_posts, create_post, delete_post
- ✅ `topic/commands.rs`: get_topics, create_topic, update_topic, delete_topic

### 今後の対応
1. [ ] SQLx CLIのセットアップドキュメント作成
2. [ ] CI/CDパイプラインへのSQLx prepare統合
3. [ ] 開発者向けセットアップガイドの更新
4. [ ] オフラインモードへの移行（方法2）

## トラブルシューティング

### よくあるエラー

#### 1. "no rows returned by a query that expected to return at least one row"
```rust
// 問題のあるコード
let row = sqlx::query!("SELECT * FROM users WHERE id = ?", id)
    .fetch_one(&pool)
    .await?;

// 解決策
let row = sqlx::query!("SELECT * FROM users WHERE id = ?", id)
    .fetch_optional(&pool)
    .await?;
```

#### 2. "mismatched types"
```rust
// SQLiteでは整数型がi64で返される
let count: i64 = row.get("count"); // OK
let count: i32 = row.get("count"); // エラー
```

#### 3. JSON型の扱い
```rust
// SQLiteではJSONはTEXT型として保存
let tags_json: String = row.get("tags");
let tags: Vec<Tag> = serde_json::from_str(&tags_json)?;
```

## ベストプラクティス

1. **開発時**: query!マクロを使用して型安全性を確保
2. **CI/CD**: オフラインモードで事前生成されたスキーマを使用
3. **本番**: queryマクロで柔軟に対応
4. **テスト**: インメモリデータベースで高速実行

## 参考リンク
- [SQLx Documentation](https://docs.rs/sqlx/latest/sqlx/)
- [SQLx Offline Mode](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query)
- [SQLx CLI](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)