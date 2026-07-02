-- #415: community node trust / relation foundation（ADR 0026）。
--
-- relation の「見えない」opt-out（§2.6 / §6.3 Decision）を保持する。user は自分を
-- (a) 他者の discovery / surfacing、(b) 他者から見た relation read の双方から外せる。
-- node-local な選択であり、可逆（行の削除で解除）。social graph canonical の削除ではなく、
-- trust には影響しない（troll 判定回避の手段にしない — trust read は本テーブルを見ない）。
--
-- trust / relation の scoring 状態はここに持たない: trust は risk signal（cn_safety.risk_signals）
-- からの再計算、relation graph は ArcadeDB（derived projection、index 投影に相乗り）であり、
-- どちらも再構築可能な derived state（ADR 0026 Feature Data Classification）。

CREATE SCHEMA IF NOT EXISTS cn_trust;

-- relation opt-out（「見えない」選択）の真実源。
CREATE TABLE cn_trust.relation_optouts (
    -- opt-out した user の pubkey（正規化済み hex）。
    pubkey TEXT PRIMARY KEY,
    -- opt-out した時刻（説明可能性: いつから見えなくなったか）。
    opted_out_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
