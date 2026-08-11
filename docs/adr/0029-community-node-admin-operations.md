# ADR 0029: Community Node admin operations

## Status

Accepted

## Context

Community Node の admin Web surface は GCP IAP TCP forwarding と IAM を公開境界にし、
稼働状態、supported topic、通報、readiness、Cloud Logging 導線を read-only で提供している。
運用時の定型操作を browser から行うには、CLI と同じ runtime state を更新しつつ、誰が何を
変更したかを改変不能な形で残し、cross-site request と意図しない即時適用を防ぐ必要がある。

一方、provider credential、LLM endpoint、capability、image revision は deployment state であり、
runtime DB の運用 state ではない。これらを admin process へ預けたり browser から直接変更すると、
`operator-config.yaml`、Terraform、secret manager、readiness activation の責任境界を迂回する。

## Decision

### Browser から適用できる操作

admin listener では、runtime Postgres が canonical source である次の操作だけを扱う。

- admission mode の変更
- public supported topic の追加・除去
- 受信済み report の状態変更

各操作は `preview -> confirm -> apply` の二段階にする。apply 時にも入力を再検証し、対象 state の
変更と append-only audit row の追加を同じ Postgres transaction で commit する。

### Browser から適用しない操作

次は引き続き reviewed `operator-config.yaml` / Terraform / secret management / `cn-cli readiness`
workflow でのみ変更する。

- provider / LLM / Project Arachnid の endpoint と credential
- capability availability / authority scope
- image digest / deployment revision
- private channel secret
- invite code、allowlist、ban のように秘密値または subscriber identity を扱う操作

admin UI は上記の設定状態と責任境界を表示してよいが、値や credential を表示・保存・変更しない。

### Actor と認可

- listener 自体は public user API と分離し、GCP firewall と IAP TCP forwarding / IAM の内側だけで公開する。
- browser write は `COMMUNITY_NODE_ADMIN_ACTOR` が非空で設定されたときだけ有効にする。
- audit の actor はこの deployment-controlled 値を使い、form input や任意 HTTP header は信用しない。
- actor 未設定時は dashboard を read-only に保ち、write endpoint は fail-closed にする。

IAP TCP forwarding は HTTP identity header を注入しないため、初期運用では deployment actor を
記録する。将来 HTTPS IAP が authenticated principal を保証できるようになった場合は、検証済み
principal へ置き換えられる。任意 header だけを根拠に actor を切り替えてはならない。

### CSRF と確認

- process 起動時に十分な entropy を持つ CSRF token を生成し、admin state のみに保持する。
- state-changing endpoint は POST のみとし、hidden token の一致を必須にする。
- preview は変更前後、影響、actor を表示し、apply は明示的な confirm action からのみ到達させる。
- apply は preview を信用せず、現在値と入力を再取得・再検証する。

### Audit

`cn_admin.operator_actions` は append-only とし、次を保存する。

- action id / occurred_at / actor
- action / target kind / target id
- before / after JSON

DB trigger で UPDATE / DELETE を拒否する。秘密、credential、report details、reporter contact、
private channel capability は audit payload に含めない。

### Feature Data Classification

- Feature 名: Community Node admin operations and audit
- Durable / Transient: operation result と audit は Durable、CSRF token と preview は Transient
- Canonical Source: runtime state と audit は Community Node Postgres、deployment state は `operator-config.yaml` / Terraform / secret manager
- Replicated?: しない
- Rebuildable From: runtime state は既存 operator workflow、audit は再構築不可
- Public Replica / Private Replica / Local Only: node operator private state（IAP 内部のみ）
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要
- 必須 contract: actor 未設定 fail-closed、CSRF 拒否、preview と apply の再検証、mutation と audit の transaction 原子性、audit UPDATE / DELETE 拒否、secret 非記録
- 必須 scenario: Postgres integration で admission / supported topic / report status と audit の同時反映を確認。production は IAP tunnel 経由の read/preview と harmless no-op apply を smoke する

## Consequences

- 日常的な runtime 操作は admin UI で確認と監査を伴って実行できる。
- deployment / credential の変更は browser write の対象外で、既存の review と readiness gate を維持する。
- IAP TCP forwarding 中は actor が deployment 単位であり、個人単位の IAM principal ではない。この制約は UI と runbook に表示する。
- admin listener を public Caddy / DNS へ載せてはならない。
