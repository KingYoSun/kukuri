-- #404: fail-closed community indexing の verdict state と index 真実源。
--
-- ADR 0025 §2.5「index entry は対応する safety verdict state を必ず伴う（verdict 無しの index
-- entry を作らない）」と、fail-closed 不変条件（非 allow / unscanned / scan_failed /
-- provider_unavailable / critical を index に入れない）を **DB 制約**として固定する。
--
--   - cn_safety.scan_verdicts: scan 対象（subject）ごとの最新 verdict。scan のたびに同一行を
--     更新する（id は据え置き）ため、index entry からの FK 参照は常に最新 verdict を指す。
--   - cn_index.index_entries: index された entry の真実源。ArcadeDB 投影（全文検索）は本テーブルの
--     derived な写像であり、query 境界は投影の hit を本テーブルと突合して fail-closed に返す。
--
-- 検索対象 text は保存しない（投影は replica の再 ingest + 再 scan で再構築する。ADR 0025 §2.1）。

-- scan 対象ごとの最新 safety verdict。
--
-- `allow` を含む全 scan 結果を記録する（signed moderation event / risk signal は非 allow 時のみ
-- 生成されるため、それらとは別に「最新の判定そのもの」を保持する）。scanned_at は verdict の
-- RFC3339 文字列をそのまま保持する（`cn-safety` の SafetyVerdict.scanned_at と同一表現）。
CREATE TABLE cn_safety.scan_verdicts (
    -- verdict record id（呼び出し側=runtime が採番。UUID v4）。index entry からの FK 参照先。
    id TEXT PRIMARY KEY,
    -- scan 対象種別（post / blob / user / peer）。
    subject_kind TEXT NOT NULL,
    -- scan 対象識別子（post id / blob CID 等）。
    subject_id TEXT NOT NULL,
    -- 最終 action（allow / hold / quarantine / exclude）。
    action TEXT NOT NULL,
    -- critical safety（CSAM / CSE / grooming）検知か。
    critical BOOLEAN NOT NULL,
    -- reason code（clean / csam_confirmed / scan_failed / provider_unavailable / ...）。
    reason_code TEXT NOT NULL,
    -- classifier confidence（0-100。任意）。
    confidence SMALLINT,
    -- 判定した provider 名（fail-closed 経路では NULL のことがある）。
    provider TEXT,
    -- policy バージョン。
    policy_version TEXT NOT NULL,
    -- scan 時刻（RFC3339 文字列。SafetyVerdict.scanned_at の原文）。
    scanned_at TEXT NOT NULL,
    -- この行を最後に更新した時刻。
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 対象ごとに最新 1 行のみ保持する（upsert の鍵）。
    UNIQUE (subject_kind, subject_id)
);

-- index された entry の真実源（fail-closed gate の権威レコード）。
--
-- 不変条件を DB 制約で保証する:
--   - verdict_id NOT NULL + FK: verdict 無しの index entry を作らない（unscanned は verdict 行が
--     存在し得ないため構造的に index できない）。
--   - CHECK (verdict_action = 'allow'): 非 allow（hold / quarantine / exclude）と fail-closed
--     経路（scan_failed / provider_unavailable / unscanned は action が allow にならない）の
--     entry は INSERT / UPDATE いずれでも書き込めない。
--   - CHECK (NOT critical): critical verdict は search / discovery / recommendation の
--     いずれにも入らない。
CREATE TABLE cn_index.index_entries (
    -- scope 種別（public_topic / private_channel）。
    scope_kind TEXT NOT NULL,
    -- scope 識別子（topic_id / channel_id）。de-index の単位。
    scope_id TEXT NOT NULL,
    -- 対象 object id（post id 等）。scope 内で一意。
    object_id TEXT NOT NULL,
    -- 著者 pubkey。
    author_pubkey TEXT NOT NULL,
    -- content の作成時刻（unix 秒）。新着列挙（discovery / recommendation）の並び順。
    created_at BIGINT NOT NULL,
    -- 由来の共有 replica id（監査用。ghost 注入でないことの追跡）。
    source_replica_id TEXT NOT NULL,
    -- 対応する verdict record（cn_safety.scan_verdicts）。行は対象ごとに upsert されるため、
    -- この参照は常に最新 verdict を指す（query 境界はこれを join して現在値を再確認する）。
    verdict_id TEXT NOT NULL REFERENCES cn_safety.scan_verdicts (id),
    -- index 時点の verdict action。CHECK で allow のみ許す。
    verdict_action TEXT NOT NULL CHECK (verdict_action = 'allow'),
    -- index 時点の critical フラグ。CHECK で false のみ許す。
    critical BOOLEAN NOT NULL CHECK (NOT critical),
    -- index へ入れた（更新した）時刻。
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (scope_kind, scope_id, object_id)
);

-- scope 単位の de-index / 新着列挙（created_at 降順）を支える index。
CREATE INDEX idx_cn_index_index_entries_scope_created_at
    ON cn_index.index_entries (scope_kind, scope_id, created_at DESC);

-- 横断（supported set 全体）の新着列挙用。
CREATE INDEX idx_cn_index_index_entries_created_at
    ON cn_index.index_entries (created_at DESC);
