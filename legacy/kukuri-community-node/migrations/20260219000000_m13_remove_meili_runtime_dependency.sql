INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by)
VALUES
    ('search_read_backend', 'pg', 'migration'),
    ('search_write_mode', 'pg_only', 'migration')
ON CONFLICT (flag_name) DO UPDATE
SET flag_value = EXCLUDED.flag_value,
    updated_at = NOW(),
    updated_by = EXCLUDED.updated_by;
