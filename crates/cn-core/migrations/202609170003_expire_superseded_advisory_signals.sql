-- #1109 (ADR 0028 §8.14): moderation 構成の変更後の再 scan で allow・advisory なしになった subject に
-- 残る、scanner 由来の nsfw / objectionable signal を失効させる。
--
-- 実行時は再 scan が同じ規則で失効させるが、この migration より前に再 scan 済みの subject は保存済み
-- verdict の再利用で再 scan されないため、既存行をここで揃える。
--
-- 対象: subject の最新 verdict 行が `allow` で、その行の advisory に同じ subject・category の要素が
-- 無く、signal が verdict 行の最終更新以前に保存された行。signal の issuer は問わない（この表へ挿入
-- するのは自 node の scanner と operator の訂正版だけであり、verdict 行は node の現在の判定）。
-- 対象外: operator 確定（#1058）、appeal_status が none 以外、appeal 通報から参照される行（棄却を含む）、
-- critical・spam 系、classifier_score 以外、失効済み。行は削除せず、signed moderation event も変えない。
-- 冪等: 失効済み行は条件から外れるため、再実行しても差分は出ない。

UPDATE cn_safety.risk_signals s
SET expires_at = to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
FROM cn_safety.scan_verdicts v
WHERE s.target IN ('post_id', 'blob_cid')
  AND v.subject_kind = CASE s.target WHEN 'post_id' THEN 'post' ELSE 'blob' END
  AND v.subject_id = s.target_id
  AND v.action = 'allow'
  AND s.persisted_at <= v.updated_at
  AND s.category IN ('nsfw', 'objectionable')
  AND s.basis = 'classifier_score'
  AND s.expires_at IS NULL
  AND COALESCE(s.appeal_status, 'none') = 'none'
  AND s.operator_adjusted_at IS NULL
  AND NOT EXISTS (
      SELECT 1 FROM cn_admin.reports r WHERE r.appeal_risk_signal_id = s.id
  )
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements(v.advisory_labels) AS advisory
      WHERE advisory ->> 'subject_kind' = s.target
        AND advisory ->> 'subject_id' = s.target_id
        AND advisory ->> 'category' = s.category
  );
