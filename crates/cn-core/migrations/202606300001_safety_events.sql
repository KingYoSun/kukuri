-- #405: signed moderation event / risk signal の永続化。
--
-- community node は自分の authority scope 内の判断を signed moderation event として保存・配布でき、
-- risk signal を trustness / relation 反映のために保存する。これは network-wide command ではなく
-- issuer node の advisory（docs/architecture/moderation-event-trust-semantics.md）。
-- visibility（local / subscribed_nodes / public）が配布境界を決める。suspected unknown CSAM/CSE は
-- local 既定であり、配布クエリは local を issuer node の外へ出さない。
CREATE SCHEMA IF NOT EXISTS cn_safety;

-- 署名済み moderation event。
--
-- body.created_at と signature の整合のため、created_at は scanner が与えた RFC3339 文字列を
-- **そのまま** TEXT で保存する（timestamptz へ正規化すると canonical_bytes が変わり、ロード後の
-- 署名検証が壊れるため）。配布境界の判定には visibility 列を使う。
CREATE TABLE cn_safety.signed_moderation_events (
    -- moderation event id（呼び出し側=runtime が採番。UUID v4 等）。冪等 insert の鍵。
    id TEXT PRIMARY KEY,
    -- 発行・署名した node の x-only 公開鍵 hex。
    issuer_node_id TEXT NOT NULL,
    -- 対象種別（post / blob / user / peer）。
    target_type TEXT NOT NULL,
    -- 対象識別子。
    target_id TEXT NOT NULL,
    -- moderation action（exclude / hold / quarantine / risk_label）。
    action TEXT NOT NULL,
    -- reason code（csam_confirmed / csam_suspected / ...）。
    reason_code TEXT NOT NULL,
    -- severity（critical / high / medium / low）。
    severity TEXT NOT NULL,
    -- 判定根拠（known_hash_match / provider_verdict / classifier_score / local_policy）。
    basis TEXT NOT NULL,
    -- 配布範囲（local / subscribed_nodes / public）。
    visibility TEXT NOT NULL,
    -- classifier confidence（0-100。任意）。
    confidence SMALLINT,
    -- policy バージョン。
    policy_version TEXT NOT NULL,
    -- 検知ラベル（SafetyLabel の配列）。
    labels JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- issuer node による署名（schnorr hex）。
    signature TEXT NOT NULL,
    -- body.created_at（RFC3339 文字列。署名対象なので原文を保持）。
    event_created_at TEXT NOT NULL,
    -- 永続化時刻（運営者の一覧表示用。署名対象ではない）。
    persisted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 対象ごとの参照・集約。
CREATE INDEX idx_cn_safety_events_target
    ON cn_safety.signed_moderation_events (target_type, target_id);

-- 配布境界クエリ（visibility で絞り persisted_at 降順で返す）と新着順一覧の双方を支える複合 index。
-- filter（visibility）→ sort（persisted_at DESC）の access path を 1 本でカバーする。
CREATE INDEX idx_cn_safety_events_visibility_persisted_at
    ON cn_safety.signed_moderation_events (visibility, persisted_at DESC);

-- trustness / relation 反映用の risk signal。
--
-- risk signal は署名対象ではない（advisory な根拠つきラベル）。expires_at は失効判定に使うため
-- RFC3339 文字列を保存しつつ、配布クエリでは ::timestamptz へキャストして比較する。
CREATE TABLE cn_safety.risk_signals (
    -- 永続化側が採番する id（UUID v4）。
    id TEXT PRIMARY KEY,
    -- この signal を保持する issuer node（authority scope の追跡用）。
    issuer_node_id TEXT NOT NULL,
    -- target 種別（user_pubkey / peer_node / post_id / blob_cid）。
    target TEXT NOT NULL,
    -- target 識別子。
    target_id TEXT NOT NULL,
    -- safety category（csam / cse / grooming / nsfw / spam / malware / phishing）。
    category TEXT NOT NULL,
    -- severity。
    severity TEXT NOT NULL,
    -- 判定根拠。
    basis TEXT NOT NULL,
    -- 配布範囲（local / subscribed_nodes / public）。
    visibility TEXT NOT NULL,
    -- confidence（0-100。任意）。
    confidence SMALLINT,
    -- 失効時刻（RFC3339 文字列。任意。NULL = 無期限）。
    expires_at TEXT,
    -- 異議申し立て状態（none / disputed / cleared。任意）。
    appeal_status TEXT,
    -- 永続化時刻。
    persisted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 対象ごとの参照。
CREATE INDEX idx_cn_safety_risk_signals_target
    ON cn_safety.risk_signals (target, target_id);

-- 配布境界クエリ（visibility で絞り persisted_at 降順で返す）と新着順一覧の双方を支える複合 index。
-- expires_at の ::timestamptz キャストは sargable ではないが、visibility 絞り込み + 新着順は本 index
-- でカバーする。
CREATE INDEX idx_cn_safety_risk_signals_visibility_persisted_at
    ON cn_safety.risk_signals (visibility, persisted_at DESC);
