-- #1058: operator が審査・運用是正で確定した risk signal に印を付け、構成変更後の再 scan が
-- 集約更新（#1050）で値を上書きしたり、元の鍵で新しい行を作ったりしないようにする。
--
-- operator_adjusted_at: 最後に operator が値を確定した時刻（NULL = scanner 由来の未訂正行）。
-- operator_origin_category: 最初の訂正前の scanner 由来 category。category を変える訂正の後も、
--   元の category の再 scan を同じ判定として抑止するために使う（印のある行では NOT NULL）。
ALTER TABLE cn_safety.risk_signals
    ADD COLUMN IF NOT EXISTS operator_adjusted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS operator_origin_category TEXT;

ALTER TABLE cn_safety.risk_signals
    DROP CONSTRAINT IF EXISTS ck_cn_safety_risk_signals_operator_origin;
ALTER TABLE cn_safety.risk_signals
    ADD CONSTRAINT ck_cn_safety_risk_signals_operator_origin
    CHECK (operator_adjusted_at IS NULL OR operator_origin_category IS NOT NULL);

-- 審査経路の過去の訂正を監査記録から復元する（cn-cli の過去操作は監査記録が無く復元できない）。
-- 編集 → 再発行、再発行 → 再審査の連鎖で元の category を引き継ぐため、発生順に 1 件ずつ適用する。
-- 冪等: 元の category は未設定の場合だけ入れ、時刻は大きい方を残す。
DO $$
DECLARE
    rec RECORD;
    origin TEXT;
BEGIN
    FOR rec IN
        SELECT target_id, action AS kind, occurred_at, before_json, after_json
        FROM cn_admin.operator_actions
        WHERE target_kind = 'risk_signal_appeal'
          AND action IN ('appeal.edit_detection', 'appeal.reissue_correction')
        ORDER BY occurred_at, id
    LOOP
        SELECT COALESCE(s.operator_origin_category, rec.before_json ->> 'category')
        INTO origin
        FROM cn_safety.risk_signals s
        WHERE s.id = rec.target_id;
        origin := COALESCE(origin, rec.before_json ->> 'category');
        IF origin IS NULL THEN
            CONTINUE;
        END IF;

        IF rec.kind = 'appeal.edit_detection' THEN
            UPDATE cn_safety.risk_signals
            SET operator_adjusted_at = GREATEST(operator_adjusted_at, rec.occurred_at),
                operator_origin_category = COALESCE(operator_origin_category, origin)
            WHERE id = rec.target_id;
        ELSE
            UPDATE cn_safety.risk_signals
            SET operator_adjusted_at = GREATEST(operator_adjusted_at, rec.occurred_at),
                operator_origin_category = COALESCE(operator_origin_category, origin)
            WHERE id = rec.after_json -> 'reissued_risk_signal' ->> 'id';
        END IF;
    END LOOP;
END
$$;

-- 再 scan 時の保護判定（issuer / target / target_id / basis で印のある行を引く）。
CREATE INDEX IF NOT EXISTS idx_cn_safety_risk_signals_operator_adjusted
    ON cn_safety.risk_signals (issuer_node_id, target, target_id, basis)
    WHERE operator_adjusted_at IS NOT NULL;
