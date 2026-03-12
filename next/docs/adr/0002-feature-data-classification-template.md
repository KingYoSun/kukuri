# ADR 0002: Feature Data Classification Template

## Status
Accepted

## Decision
- 新しい feature を追加する前に、必ず以下を定義する。
  - `Feature 名`
  - `Durable / Transient`
  - `Canonical Source`
  - `Replicated`
  - `Rebuildable From`
  - `Public Replica / Private Replica / Local Only`
  - `Gossip Hint 必要有無`
  - `Blob 必要有無`
  - `SQLite projection 必要有無`
  - `必須 contract`
  - `必須 scenario`

## Template
```md
### Feature Data Classification
- Feature 名:
- Durable / Transient:
- Canonical Source:
- Replicated?:
- Rebuildable From:
- Public Replica / Private Replica / Local Only:
- Gossip Hint 必要有無:
- Blob 必要有無:
- SQLite projection 必要有無:
- 必須 contract:
- 必須 scenario:
```

## Consequences
- `docs / blobs / gossip / SQLite` の責務が未定義な feature 実装を禁止する。
- 実装より先に contract と scenario の必要集合を固定する。
