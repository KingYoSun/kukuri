# ADR 0009: Community-Node Relay/Auth Data Classification

## Status
Accepted

## Feature Data Classification
- Feature 名: community-node relay/auth
- Durable / Transient: Durable server state + local durable desktop config
- Canonical Source: server `Postgres` + desktop local `community-node.json` + desktop secure token storage
- Replicated?: Server state is not client-replicated; desktop config is local only
- Rebuildable From: server `Postgres` migrations/seed data + desktop local config + secure token storage
- Public Replica / Private Replica / Local Only: public bootstrap metadata, private auth/consent state in `Postgres`, local desktop config/token storage
- Gossip Hint 必要有無: No
- Blob 必要有無: No
- SQLite projection 必要有無: Desktop local only; server does not use SQLite
- 必須 contract:
  - `community_node_auth_verify_rejects_wrong_relay_tag`
  - `community_node_bootstrap_requires_auth_when_rollout_is_required`
  - `community_node_relay_rollout_respects_existing_connection_grace`
  - `desktop_runtime_restores_community_node_config_and_tokens_after_restart`
  - `transport_custom_relay_mode_connects_when_community_node_relay_urls_are_configured`
- 必須 scenario:
  - `community_node_public_connectivity`

## Decision
- community-node server persistence は Phase6 から `Postgres` に固定する。
- current desktop canonical data plane は `docs + blobs + hints + DHT` のまま維持し、community-node は接続基盤と auth/control plane を担う。
- desktop は multi-node list を保持し、token は keyring/file fallback へ保存する。
- `iroh_relay_urls` の反映は startup-only とし、変更時は desktop restart を要求する。

## Consequences
- server 側の query/migration/testing は最初から `Postgres` 前提に揃える。
- migration/seed の標準入口は `cn-cli prepare` とし、`cn-user-api` / `cn-relay` は prepared DB を既定前提に fail-fast 起動する。
- `docs/blobs/gossip/SQLite` の既存責務を community-node 導入で変更してはならない。
- desktop の community-node 設定変更は `iroh` endpoint 再生成が必要な場合に限り `restart_required` を返す。
