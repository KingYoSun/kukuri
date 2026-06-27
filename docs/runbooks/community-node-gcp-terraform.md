# Community Node GCP Terraform Deploy

最終更新日: 2026-06-27

## 目的

- community node（`cn-user-api` + `cn-iroh-relay` + Postgres + Valkey）を GCP に
  Terraform でデプロイする（Issue #381）。
- 初期から deployment profile を切り替えられる（`low-cost` / `managed-db` / `ha`）。
- third-party community node operator が低コストで始められる `low-cost` profile を標準入口にする。

実装は `infra/terraform/`。この runbook は実行手順、設計の根拠は同 `README.md` と
`docs/architecture/p2p-first-community-node-responsibility-boundary.md` を参照する。

> 注意: この runbook は法的助言ではない。日本国内で relay を運用する場合の電気通信事業の
> 届出要否や記載内容は、最終的に operator 自身と総合通信局・専門家への確認が必要。region 既定の
> `asia-northeast1`（東京）も法的保証ではなく、単なる既定値。

## アーキテクチャ

- 全 profile で `cn-user-api` + `cn-iroh-relay` は **Compute Engine VM** 上で動かす。
  Cloud Run は relay の UDP/QUIC（`7842/udp`）を扱えないため。
- profile ごとに変わるのは data/cache/blob/backup 階層。
- `low-cost` は単一 VM 上で既存 community node スタックを GHCR image から動かし、
  Caddy が API/relay-HTTP の HTTPS を終端、QUIC は VM の `7842/udp` で直接公開する。

```text
client ──https://api_domain      ─▶ Caddy(:443) ─▶ cn-user-api(:8080)
client ──https://relay_domain    ─▶ Caddy(:443) ─▶ cn-iroh-relay(:3340)
client ──relay_domain :7842/udp  ─────────────────▶ cn-iroh-relay QUIC(:7842)
VM 内: cn-postgres / cn-valkey は private（公開しない）
```

### TLS / 証明書

- Caddy 内部 auto-TLS は使わない。`cn-iroh-relay` の QUIC が PEM ファイルを直接読むため、
  ACME companion（certbot）が `api_domain` / `relay_domain` の PEM を `/var/lib/kukuri/certs` に
  発行・更新し、Caddy と relay が共有する。
- 初回は startup script が `certbot --standalone` で発行（Caddy が :80 を bind する前）。
- 更新は systemd timer（daily）が webroot mode で renew し、更新時のみ Caddy reload +
  relay restart で新 PEM を反映する。

## データ境界

| データ | 置き場所 | backup |
|---|---|---|
| auth/consent, admission mode, invite/allowlist/ban, report metadata, operator config | Postgres（control-plane data） | low-cost: pg_dump→GCS / managed-db,ha: Cloud SQL |
| topic rendezvous, presence, short-lived connection hints | Valkey（TTL ephemeral） | 対象外 |
| blob/media 本体 | local cache / iroh blobs / object storage（**Postgres に置かない**） | 対象外（rebuildable cache） |
| community_index / moderation / community_local_trust | Phase B（未提供）。canonical DB に同居させない | 対象外 |

## 人手で先に用意するもの

GCP / GitHub の以下は Terraform の前に手動で用意する。`gcloud` 例:

```bash
# 1) project / billing / API
gcloud config set project YOUR_PROJECT
gcloud services enable \
  compute.googleapis.com iam.googleapis.com iamcredentials.googleapis.com \
  secretmanager.googleapis.com storage.googleapis.com \
  dns.googleapis.com serviceusage.googleapis.com cloudresourcemanager.googleapis.com

# 2) Terraform state 用 GCS backend bucket
gsutil mb -l asia-northeast1 gs://YOUR_TF_STATE_BUCKET
gsutil versioning set on gs://YOUR_TF_STATE_BUCKET

# 3) Secret Manager に secret を作成（payload は Terraform に渡さない）
#    JWT secret は 32 byte 以上、placeholder 文字列(change-me 等)を含めない。
printf '%s' "$(openssl rand -hex 32)" | \
  gcloud secrets create kukuri-cn-jwt-secret --data-file=-
printf '%s' "$(openssl rand -hex 24)" | \
  gcloud secrets create kukuri-cn-postgres-password --data-file=-

# PowerShell で secret file を作る場合は BOM/改行が混入しないよう、ASCII + NoNewline にする。
# 例: Set-Content -Encoding ascii -NoNewline secret.txt <hex-string>
# BOM/改行が混じると Postgres DSN / JWT secret として読めず、起動時 migration が失敗する。

# 4) GHCR の public image を用意（cn-user-api / cn-iroh-relay / cn-cli）
#    public image 前提なので VM 側の認証は不要。
```

GHCR image は `.github/workflows/kukuri-cn-images.yml` が `docker/cn/Dockerfile` を使って 3 binary 分 build/publish する。

| binary | package | GHCR image |
|---|---|---|
| `cn-user-api` | `kukuri-cn-user-api` | `ghcr.io/kingyosun/kukuri-cn-user-api:<tag>` |
| `cn-iroh-relay` | `kukuri-cn-iroh-relay` | `ghcr.io/kingyosun/kukuri-cn-iroh-relay:<tag>` |
| `cn-cli` | `kukuri-cn-cli` | `ghcr.io/kingyosun/kukuri-cn-cli:<tag>` |

初回 publish は workflow を default branch（通常 `main`）へ merge した後に手動 dispatch する。workflow file がまだ default branch に無い状態では `workflow_dispatch` が見つからないため、PR 中は build-only 検証に留める:

```bash
gh workflow run kukuri-cn-images.yml -f image_tag=latest -f push=true
```

既に workflow が default branch に存在する場合は、必要に応じて `--ref <branch-or-tag>` を付けて特定 ref の image を publish する。

workflow は PR では build のみ、`main` push / `develop` push / `v*` tag push / manual dispatch では GHCR に push する。
`main` push は `latest` と `sha-<12桁>`、`develop` push は `develop` と `sha-<12桁>`、tag push は tag 名と `sha-<12桁>` を publish する。

初回 publish 後、GitHub Packages の各 package visibility が private の場合は、Terraform で使う前に GitHub UI で public に変更する（VM は GHCR 認証なしで pull する前提）。
`terraform.tfvars` には、例えば以下を指定する:

```hcl
cn_user_api_image   = "ghcr.io/kingyosun/kukuri-cn-user-api:latest"
cn_iroh_relay_image = "ghcr.io/kingyosun/kukuri-cn-iroh-relay:latest"
cn_cli_image        = "ghcr.io/kingyosun/kukuri-cn-cli:latest"
```

本番 apply では `latest` より digest 固定（例: `ghcr.io/kingyosun/kukuri-cn-user-api@sha256:...`）を推奨する。

## low-cost deploy

```bash
cd infra/terraform/envs/low-cost
cp terraform.tfvars.example terraform.tfvars   # 値を埋める（secret VALUE は書かない）
cp backend.hcl.example backend.hcl             # state bucket/prefix を埋める

terraform init -backend-config=backend.hcl
terraform plan
terraform apply
```

- `manage_cloud_dns=false`（既定）の場合は `terraform output static_ip` の IP に対して
  `api_domain` / `relay_domain` の A レコードを手動で設定する。
- `manage_cloud_dns=true` の場合は既存 Cloud DNS zone（`dns_zone_name`）に A レコードを作成する。
- 単独 operator が GCS backend を使わず始める場合は、`backend.tf` の `backend "gcs" {}` を
  コメントアウトして `terraform init`（local backend）でもよい。

### 確認

```bash
curl -fsS https://<api_domain>/healthz
curl -fsS https://<relay_domain>/ping
terraform output ssh_iap_command   # IAP 経由 SSH
```

VM 内のサービスは `/var/lib/kukuri/community-node` の docker compose で動く。SSH は IAP のみ
（`22/tcp` は GCP IAP レンジからのみ許可）。

### admission / 運用

入会制御（招待 / whitelist / ban）は `cn-cli admission` を VM 上の compose で実行する。
`docs/runbooks/community-node-self-host-vps.md` の admission 節と同じ操作。

```bash
# VM へ IAP SSH 後
cd /var/lib/kukuri/community-node
docker compose run --rm cn-migrate   # 既に起動時に実行済み（再実行は冪等）
# admission 用に cn-cli image を直接使う例:
docker run --rm --network kukuri-community-node_default \
  --env-file ./community-node.env -e COMMUNITY_NODE_DATABASE_URL="$(grep COMMUNITY_NODE_DATABASE_URL .env | cut -d= -f2-)" \
  ghcr.io/<owner>/kukuri-cn-cli:latest admission show
```

## backup / restore

- low-cost の backup は systemd timer（`kukuri-backup.timer`）が `pg_dump -Fc` を取り、
  GCS backup bucket（`terraform output backup_bucket`）へアップロードする。
- Valkey と blob cache は backup 対象外。

restore 例（VM 上）:

```bash
cd /var/lib/kukuri/community-node
# 取得した dump を cn-postgres に流し込む（事前に cn-user-api 停止推奨）
docker compose stop cn-user-api
cat cn-postgres.dump | docker compose exec -T cn-postgres \
  sh -lc 'pg_restore --clean --if-exists --no-owner -U "$POSTGRES_USER" -d "$POSTGRES_DB"'
docker compose start cn-user-api
```

## managed-db / ha（拡張点）

`managed-db` は Cloud SQL + Memorystore、`ha` は HA DB/cache + object storage を使う。
初期実装では root は `terraform validate` まで対応する拡張点で、apply 完成は後続。

```bash
terraform -chdir=infra/terraform/envs/managed-db init -backend=false
terraform -chdir=infra/terraform/envs/managed-db validate
terraform -chdir=infra/terraform/envs/ha init -backend=false
terraform -chdir=infra/terraform/envs/ha validate
```

注意点（apply 時）:

- Cloud SQL は private services access（VPC peering）経由の private IP で VM から接続する。
  network module が `enable_private_services_access=true` で peering を作成する。初回 apply は
  `servicenetworking.googleapis.com` の有効化が必要。
- DB password は 2 か所で扱う: Cloud SQL user 作成用に `TF_VAR_database_password`（state には
  sensitive 保持、VM metadata には焼かない）、VM が boot 時に取得する用に同じ値を Secret Manager
  へ登録し `database_password_secret_id` を指定する。VM は起動時にこの secret から password を
  取得し、DSN へ URL-encode して組み立てる。
- VM への DB password 注入は startup script の metadata に平文を残さない（low-cost の JWT/PG
  secret と同じ Secret Manager fetch 方式）。

## local Postgres のデータ永続化（low-cost）

- `postgres_data_disk_gb=0`（既定）では Postgres data は boot disk 上の docker volume に置く。
  startup script の変更などで VM が置換されると boot disk ごとデータが消える可能性がある。
- 本番運用や long-lived node では `postgres_data_disk_gb` を 1 以上にして専用 persistent disk
  （`prevent_destroy`、VM 置換でも残る）に Postgres data を置くことを推奨する。
- 専用 disk 利用時は ext4 の `lost+found` で `initdb` が失敗しないよう、compose が `PGDATA` を
  mount 直下のサブディレクトリ（`/var/lib/postgresql/data/pgdata`）に設定する。docker volume
  利用時（既定）は `lost+found` が無いため従来どおり mount 直下を `PGDATA` とする。
- backup（pg_dump→GCS）は別途有効。disk 永続化と backup は併用する。
- `enable_disk_snapshots=true` の場合、boot disk と `postgres_data_disk_gb>0` で作成した Postgres data disk の両方に snapshot schedule を attach する。

## CI（GitHub Actions）

`.github/workflows/kukuri-terraform.yml`:

- `fmt-validate`: `terraform fmt -check -recursive` + 全 env root の `init -backend=false` +
  `validate`。credentials 不要。
- `low-cost-plan`: Workload Identity Federation で GCP に認証し、`low-cost` の `terraform plan`。
  `vars.GCP_WORKLOAD_IDENTITY_PROVIDER` が未設定、または fork PR の場合は skip する（PR 側の Terraform/workflow へ GCP OIDC を渡さない）。

### CI 用 GCP / GitHub セットアップ（人手）

```bash
# Workload Identity Federation（GitHub OIDC）
gcloud iam workload-identity-pools create github --location=global
gcloud iam workload-identity-pools providers create-oidc github \
  --location=global --workload-identity-pool=github \
  --issuer-uri="https://token.actions.githubusercontent.com" \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
  --attribute-condition="assertion.repository=='KingYoSun/kukuri'"

# provider が既に作成済みの場合は create-oidc の代わりに update-oidc を使う:
# gcloud iam workload-identity-pools providers update-oidc github \
#   --location=global --workload-identity-pool=github \
#   --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
#   --attribute-condition="assertion.repository=='KingYoSun/kukuri'"

# CI 用 service account（plan に必要な read/metadata 権限を付与）
gcloud iam service-accounts create kukuri-tf-ci
# 例: roles/compute.viewer, roles/storage.admin(state), roles/secretmanager.viewer,
#     roles/iam.serviceAccountViewer など。plan で参照する範囲に合わせる。
```

GitHub repository variables（Settings → Secrets and variables → Actions → Variables）に登録:

- `GCP_WORKLOAD_IDENTITY_PROVIDER`: `projects/900604885452/locations/global/workloadIdentityPools/github/providers/github` のような provider resource name（短い `github` だけでは不可）
- `GCP_SERVICE_ACCOUNT`: `kukuri-tf-ci@kukuri-cn.iam.gserviceaccount.com` のような service account email（短い `kukuri-tf-ci` だけでは不可）
- `GCP_PROJECT_ID`, `GCP_REGION`, `GCP_ZONE`
- `CN_API_DOMAIN`, `CN_RELAY_DOMAIN`, `CN_ACME_EMAIL`
- `CN_USER_API_IMAGE`, `CN_IROH_RELAY_IMAGE`, `CN_CLI_IMAGE`
- `CN_JWT_SECRET_ID`, `CN_POSTGRES_PASSWORD_SECRET_ID`
- `TF_BACKEND_BUCKET`, `TF_BACKEND_PREFIX`
- 任意: `CN_MANAGE_CLOUD_DNS`, `CN_DNS_ZONE_NAME`

> CI は `plan` まで。`apply` は実行しない。

## deployment profile と cn-operator capability profile

- `low-cost` / `managed-db` / `ha` は **インフラ**のコスト/データ階層の軸（この Terraform）。
- `minimal` / `relay-enabled` / `full-service` は **cn-operator** の開示・manifest 用 capability の軸
  （`docs/runbooks/community-node-operator-docs.md`）。
- 両者は独立。operator docs / manifest 生成は `cn-operator init` で別途行い、この Terraform は
  その前提を壊さない。

## 関連

- `infra/terraform/README.md`
- `docs/runbooks/community-node-self-host-vps.md`（VPS edge の手動 self-host）
- `docs/runbooks/community-node-operator-docs.md`（cn-operator の文書生成）
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
