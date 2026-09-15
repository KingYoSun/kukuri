-- #1050: 同一鍵 (issuer_node_id, target, target_id, category, basis) の活性 risk signal
-- （appeal_status が cleared 以外 かつ expires_at 無し）を 1 行に圧縮し、以後の重複を部分 UNIQUE
-- index で構造的に防ぐ。
--
-- 生存行の選び方（鍵ごと）: cn_admin.reports から参照される行 > appeal_status = 'disputed' の行
-- > persisted_at が最古の行 > id 昇順。生存行には鍵内の最古 persisted_at、最新の severity /
-- confidence / visibility、いずれかが disputed なら disputed を反映する。
-- 参照される行は削除しない。鍵内に参照行が 2 件以上ある場合、2 件目以降は削除せず expires_at で
-- 失効させる（FK と appeal 履歴を保つ）。
-- 冪等: 圧縮後に再実行しても rn > 1 の行が無く、index は IF NOT EXISTS。

CREATE TEMP TABLE risk_signal_dedupe ON COMMIT DROP AS
WITH active AS (
    SELECT
        s.id,
        s.issuer_node_id,
        s.target,
        s.target_id,
        s.category,
        s.basis,
        s.severity,
        s.confidence,
        s.visibility,
        s.appeal_status,
        s.persisted_at,
        EXISTS (
            SELECT 1 FROM cn_admin.reports r WHERE r.appeal_risk_signal_id = s.id
        ) AS referenced
    FROM cn_safety.risk_signals s
    WHERE s.appeal_status IS DISTINCT FROM 'cleared'
      AND s.expires_at IS NULL
)
SELECT
    id,
    referenced,
    ROW_NUMBER() OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
        ORDER BY referenced DESC,
                 (appeal_status = 'disputed') DESC NULLS LAST,
                 persisted_at ASC,
                 id ASC
    ) AS rn,
    MIN(persisted_at) OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
    ) AS oldest_persisted_at,
    FIRST_VALUE(severity) OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
        ORDER BY persisted_at DESC, id DESC
    ) AS newest_severity,
    FIRST_VALUE(confidence) OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
        ORDER BY persisted_at DESC, id DESC
    ) AS newest_confidence,
    FIRST_VALUE(visibility) OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
        ORDER BY persisted_at DESC, id DESC
    ) AS newest_visibility,
    BOOL_OR(appeal_status = 'disputed') OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
    ) AS any_disputed,
    COUNT(*) OVER (
        PARTITION BY issuer_node_id, target, target_id, category, basis
    ) AS group_size
FROM active;

-- 生存行へ鍵内の情報を集約する（重複が無い鍵は自分自身の値になるため変化しない）。
UPDATE cn_safety.risk_signals s
SET persisted_at = d.oldest_persisted_at,
    severity = d.newest_severity,
    confidence = d.newest_confidence,
    visibility = d.newest_visibility,
    appeal_status = CASE WHEN d.any_disputed THEN 'disputed' ELSE s.appeal_status END
FROM risk_signal_dedupe d
WHERE s.id = d.id
  AND d.rn = 1
  AND d.group_size > 1;

-- 参照されない重複行は削除する。
DELETE FROM cn_safety.risk_signals s
USING risk_signal_dedupe d
WHERE s.id = d.id
  AND d.rn > 1
  AND NOT d.referenced;

-- 参照される重複行は削除せず失効させる（活性鍵から外す）。
UPDATE cn_safety.risk_signals s
SET expires_at = to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
FROM risk_signal_dedupe d
WHERE s.id = d.id
  AND d.rn > 1
  AND d.referenced;

-- 以後、同一鍵の活性 signal は 1 行に限る。cleared 済み行と失効済み行は対象外なので、
-- appeal 審査の訂正版再発行（旧行 cleared → 新行 insert）と cn-cli の再発行
-- （旧行 expires_at → 新行 insert）は従来どおり通る。
CREATE UNIQUE INDEX IF NOT EXISTS uq_cn_safety_risk_signals_active_key
    ON cn_safety.risk_signals (issuer_node_id, target, target_id, category, basis)
    WHERE appeal_status IS DISTINCT FROM 'cleared' AND expires_at IS NULL;
