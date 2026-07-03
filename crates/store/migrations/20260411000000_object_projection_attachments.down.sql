-- 旧 down の「intentionally a no-op」を WP-S6 T1 で意図的に上書きする
-- (production に undo 経路は無く、migration round-trip テスト成立のため)。
ALTER TABLE object_index_cache
    DROP COLUMN attachments_json;
