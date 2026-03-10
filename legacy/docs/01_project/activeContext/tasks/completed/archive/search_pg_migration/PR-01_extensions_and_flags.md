# PR-01: 拡張導入と移行フラグ基盤
最終更新: 2026年02月16日

## 目的
- PostgreSQL 内で全文検索・曖昧検索・グラフ探索を成立させる基盤を整備する。
- 検索経路をフラグで切替可能にし、即時ロールバックを可能にする。

## 変更内容
- Postgres イメージに PGroonga を導入（既存 AGE は維持）。
- migration で `pg_trgm`、`pgroonga`、`age` を明示的に有効化。
- `cn_search` スキーマとランタイムフラグテーブルを追加。
- `cn-user-api`/`cn-index` で参照する検索バックエンドフラグを追加。

## DDL/インデックス
```sql
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS pgroonga;
CREATE EXTENSION IF NOT EXISTS age;

CREATE SCHEMA IF NOT EXISTS cn_search;

CREATE TABLE IF NOT EXISTS cn_search.runtime_flags (
    flag_name TEXT PRIMARY KEY,
    flag_value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT NOT NULL
);

INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) VALUES
('search_read_backend', 'meili', 'migration'),
('search_write_mode', 'meili_only', 'migration'),
('suggest_read_backend', 'legacy', 'migration'),
('shadow_sample_rate', '0', 'migration')
ON CONFLICT (flag_name) DO NOTHING;
```

## 移行/バックフィル手順
1. `kukuri-community-node/docker/postgres-age/Dockerfile` に PGroonga 導入手順を追加する。
2. migration を適用し、拡張が有効化されることを確認する。
3. サービス起動時は `search_read_backend=meili` のままにして挙動変更を防ぐ。
4. `cn_admin.service_configs` か `cn_search.runtime_flags` のどちらを正本にするかを決め、読取実装を統一する。

## ロールバック
- `search_read_backend=meili`、`suggest_read_backend=legacy` に戻す。
- 新拡張は残したまま read/write 経路のみ旧実装に固定する。
- 追加スキーマ/テーブルは削除せず、次回再実行に備えて温存する。

## テスト/計測
- migration テストで拡張の存在確認。
- 起動時ヘルスで DB ready のみでなく拡張ロード可否も確認。
- `SELECT extname FROM pg_extension WHERE extname IN ('pg_trgm','pgroonga','age');` を検証項目化。

## 運用監視
- フラグ値をログとメトリクスに出す（起動時と定期ポーリング時）。
- 拡張ロード失敗時の起動失敗アラートを追加。
- 設定変更監査ログに `flag_name/old/new/actor` を残す。

