# Issue #788 Dome Hosting 実装記録

## 完了した範囲

- owner-signed Hosting Lease と accept/activate/close の append-only contract、monotonic epoch、expiry、generation/manifest binding、split-brain fail-closed reducerを追加した。
- owner device と Community Node が共有する Rapier 3D session runtimeを追加し、参加者ゼロ時sleep、wall-clock guest TTL、再起動時initial-state resetを固定した。
- Context replicaをcanonical、SQLiteを再構築可能projection、PostgresをCN operational mirrorとして実装した。
- CNのopt-in capability、signing key/node_id検証、assign/activate/release/status、HTTP input/snapshotとWebSocket session streamを追加した。
- desktop IPC/UIからowner hosting、CNへの二段階委譲、renew/switch-back、close、署名snapshot適用を操作できるようにした。旧peer-authoritative avatar/object eventはworld version 4で廃止した。

## 後続Issueとの境界

prop layout commit/pin/GCは#793、resource budget/metricsは#794、visitor policyは#795、transitionは#790、退避/Return Homeは#797がこのlease/session contractを利用して実装する。
