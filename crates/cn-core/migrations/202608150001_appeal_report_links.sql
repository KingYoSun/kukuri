-- #680: 異議申し立て通報と審査対象のリスク判定を関連付ける。
-- 既存の通常通報は NULL のまま保ち、リスク判定単位で関連通報を集約できるようにする。
ALTER TABLE cn_admin.reports
    ADD COLUMN appeal_risk_signal_id TEXT
    REFERENCES cn_safety.risk_signals (id);

CREATE INDEX idx_cn_admin_reports_appeal_risk_signal
    ON cn_admin.reports (appeal_risk_signal_id, created_at DESC)
    WHERE appeal_risk_signal_id IS NOT NULL;
