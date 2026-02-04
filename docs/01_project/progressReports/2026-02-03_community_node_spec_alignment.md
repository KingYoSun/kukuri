# Community Node 仕様差分整理（community_node_plan / KIP-0001）
- 39022 を kind 一覧へ反映し、community_node_plan と KIP-0001 の一覧を揃えた
- Topic ID 形式を `kukuri:<64hex>` / `kukuri:global` に固定し、未決事項から除外
- NIP-44 実装方針を v2 前提に統一（nostr-sdk nip44::Version::V2、key.envelope は必須）
- Access Control の P2P-only 運用範囲を明記し、Node HTTP API には載せない方針を追記
