# envs/managed-db (extension point)

`managed-db` profile は `cn-user-api` / `cn-iroh-relay` を Compute Engine VM 上に置いたまま、
data/cache 階層を managed サービスへ寄せる構成。

- Postgres: Cloud SQL for PostgreSQL（自動バックアップ / PITR / メンテナンスをクラウドへ）
- Valkey/rendezvous: Memorystore (Redis 互換)
- blob/media 本体: Postgres に保存しない（local cache / iroh blobs。object storage は ha で）

初期実装ではこの root は **拡張点**であり、`terraform validate` まで対応する。
apply 完成（Cloud SQL private IP / Serverless VPC Access connector / Cloud SQL Auth Proxy の
配線、DB user の Secret Manager 連携など）は後続タスク。

## 検証

```bash
terraform -chdir=infra/terraform/envs/managed-db init -backend=false
terraform -chdir=infra/terraform/envs/managed-db validate
```
