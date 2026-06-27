# envs/ha (extension point)

`ha` / `production` profile は明示的に **high-cost** 構成。third-party operator の標準ではない。

- Cloud SQL: REGIONAL (Multi-AZ HA)、必要に応じて read replica
- Memorystore: STANDARD_HA
- blob/media: object storage (GCS) + 将来の CDN（canonical DB には混ぜない rebuildable cache）
- 将来: load balancer / 複数 VM / autoscaling

初期実装ではこの root は **拡張点**であり、`terraform validate` まで対応する。
LB / 複数 VM / CDN / read replica の apply 完成は後続タスク。

## 検証

```bash
terraform -chdir=infra/terraform/envs/ha init -backend=false
terraform -chdir=infra/terraform/envs/ha validate
```
