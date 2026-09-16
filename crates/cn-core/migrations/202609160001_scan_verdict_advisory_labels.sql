-- #1054 (ADR 0028 §8.6 / ADR 0025 §7.1): nsfw / objectionable の suspected を `allow` で index する
-- ときに client へ配信する content advisory を、対象ごとの最新 verdict 行に保持する。
-- 要素は issuer_node_id / subject_kind / subject_id / category / label / confidence / signal_id /
-- basis を持つ JSON object。post 行には本文 text と参照 blob の advisory の和集合が入る。
-- 旧行は既定の `[]`（advisory 無し）で、policy_version v3 への再 scan で埋まる。
-- index 真実源（cn_index.index_entries）の CHECK / FK は変えない（INVAR-1）。
ALTER TABLE cn_safety.scan_verdicts
    ADD COLUMN IF NOT EXISTS advisory_labels JSONB NOT NULL DEFAULT '[]'::jsonb;
