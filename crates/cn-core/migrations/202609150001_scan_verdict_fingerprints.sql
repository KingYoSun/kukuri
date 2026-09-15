-- #1050: 保存済み verdict の再利用鍵（内容 fingerprint + scan 構成 fingerprint）と、再利用時に
-- index text を再構成するための descriptive 検索タグを verdict 行に持たせる。
-- 旧行は fingerprint が NULL のままとなり、再利用判定では「再 scan」に倒れる（fail-closed 側）。
ALTER TABLE cn_safety.scan_verdicts
    ADD COLUMN source_fingerprint TEXT NULL,
    ADD COLUMN scan_config_fingerprint TEXT NULL,
    ADD COLUMN derived_tags JSONB NOT NULL DEFAULT '[]'::jsonb;
