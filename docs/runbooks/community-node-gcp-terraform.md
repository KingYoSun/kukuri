# Community Node GCP Terraform Deploy

最終更新日: 2026-08-04

## 目的

- community node（`cn-user-api` + `cn-iroh-relay` + Postgres + Valkey）を GCP に
  Terraform でデプロイする（Issue #381）。
- `deploy_indexer_stack=true` で index / moderation stack（`cn-indexer` + ArcadeDB +
  relation 定期解析 + moderation secrets）を同じ VM に追加する（Issue #615）。
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
deploy_indexer_stack=true 時（#615）:
VM 内: cn-arcadedb(:2480) / cn-indexer(:8630) も private（compose network 内のみ。
       firewall / Caddy へ新規公開 port は追加しない）
VM 外: cn-indexer ─outbound HTTPS─▶ Project Arachnid Shield
       cn-indexer ─private 経路（WireGuard 等）─▶ self-host VLM endpoint
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
| blob/media 本体 | local cache / iroh blobs / object storage（**Postgres に置かない**。恒久保存しない） | 対象外（rebuildable cache） |
| index 真実源（supported set / indexing request / channel secret）、moderation verdict / artifact / risk signal | Postgres（#615。既存 backup にそのまま含まれる） | low-cost: pg_dump→GCS |
| index 投影 + relation graph | ArcadeDB（**rebuildable projection。canonical store ではない**） | 対象外（空からの再構築手順を後述） |
| cn-indexer の iroh state（endpoint 同一性 / docs replica / blob store） | `indexer_data_disk_gb > 0` で専用 PD | 対象外（再同期で復元可。PD で VM 置換に耐える） |

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

# 3b) index / moderation stack（#615、deploy_indexer_stack=true のとき）の runtime secrets。
#     値は tfvars / metadata / repo に書かず、Secret Manager だけに置く。
# channel secret key（32 byte 以上、placeholder 不可。cn-user-api と cn-indexer が共有）
printf '%s' "$(openssl rand -hex 32)" | \
  gcloud secrets create kukuri-cn-channel-secret-key --data-file=-
# ArcadeDB root password（8 文字以上）
printf '%s' "$(openssl rand -hex 24)" | \
  gcloud secrets create kukuri-cn-arcadedb-password --data-file=-
# moderation event signing key（secp256k1 秘密鍵 hex。issuer node の鍵として公開鍵が event に載る）
printf '%s' "$(openssl rand -hex 32)" | \
  gcloud secrets create kukuri-cn-safety-signing-key --data-file=-
# Project Arachnid Shield credentials（operator 自身が https://projectarachnid.com で取得）
printf '%s' 'YOUR_ARACHNID_USERNAME' | \
  gcloud secrets create kukuri-cn-arachnid-username --data-file=-
printf '%s' 'YOUR_ARACHNID_PASSWORD' | \
  gcloud secrets create kukuri-cn-arachnid-password --data-file=-
# 任意: VLM API key（self-host の無認証 endpoint なら作成不要）
# printf '%s' 'YOUR_VLM_API_KEY' | gcloud secrets create kukuri-cn-vlm-api-key --data-file=-

# PowerShell で secret file を作る場合は BOM/改行が混入しないよう、ASCII + NoNewline にする。
# 例: Set-Content -Encoding ascii -NoNewline secret.txt <hex-string>
# BOM/改行が混じると Postgres DSN / JWT secret として読めず、起動時 migration が失敗する。

# 4) GHCR の public image を用意（cn-user-api / cn-iroh-relay / cn-cli / cn-indexer）
#    public image 前提なので VM 側の認証は不要。
```

GHCR image は `.github/workflows/kukuri-cn-images.yml` が `docker/cn/Dockerfile` を使って 4 binary 分 build/publish する。

| binary | package | GHCR image |
|---|---|---|
| `cn-user-api` | `kukuri-cn-user-api` | `ghcr.io/kingyosun/kukuri-cn-user-api:<tag>` |
| `cn-iroh-relay` | `kukuri-cn-iroh-relay` | `ghcr.io/kingyosun/kukuri-cn-iroh-relay:<tag>` |
| `cn-cli` | `kukuri-cn-cli` | `ghcr.io/kingyosun/kukuri-cn-cli:<tag>` |
| `cn-indexer` | `kukuri-cn-indexer` | `ghcr.io/kingyosun/kukuri-cn-indexer:<tag>` |

`cn-indexer` image（#614）は production feature 構成（Project Arachnid Shield + OpenAI-compatible VLM。mock は選択不能）で build され、workflow が publish 前に `validate-config` smoke（provider 解決 / slot 制約 / credential env 欠落 / credential 非漏出）を通す。credential / endpoint / 署名鍵は image に含まれないため、デプロイ時に runtime secret（env）として注入する。手元でも次で構成だけを検証できる:

```bash
docker run --rm -e COMMUNITY_NODE_DATABASE_URL=... -e COMMUNITY_NODE_CHANNEL_SECRET_KEY=... \
  -e COMMUNITY_NODE_INDEXER_EXTERNAL_RELAY_URLS=... \
  ghcr.io/kingyosun/kukuri-cn-indexer:<tag> validate-config
```

`cn-indexer` は low-cost Terraform stack へ `deploy_indexer_stack=true`（#615）で追加する。手順は後述の「index / moderation stack のデプロイ」を参照。

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
cn_indexer_image    = "ghcr.io/kingyosun/kukuri-cn-indexer:latest"
```

本番 apply では `latest` より digest 固定（例: `ghcr.io/kingyosun/kukuri-cn-user-api@sha256:...`）を推奨する。
全 kukuri image の digest は次で確認できる（`cn_*_image` 変数は `@sha256:` 参照をそのまま受け付ける）:

```bash
docker buildx imagetools inspect ghcr.io/kingyosun/kukuri-cn-indexer:latest --format '{{println .Manifest.Digest}}'
```

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

## index / moderation stack のデプロイ（#615）

`deploy_indexer_stack=true` で `cn-indexer` + ArcadeDB + relation 定期解析を同じ VM の compose に
追加する。既定は false（従来の API / relay のみ構成）。

### 前提

- 「人手で先に用意するもの」の 3b) の runtime secrets（channel key / ArcadeDB password /
  signing key / Arachnid credentials / 任意 VLM key）を Secret Manager に作成済みであること。
  secret の accessor 権限は Terraform が VM service account へ自動付与する。
- `machine_type` は既定 `e2-medium`（ArcadeDB(JVM) + cn-indexer の同居前提）。`e2-small` では
  memory が不足する可能性が高い。既存 VM を e2-small から上げる場合、apply が VM を
  stop/start する点に注意。
- 本番では `indexer_data_disk_gb > 0`（専用 PD）を推奨。cn-indexer の iroh endpoint 同一性と
  docs replica が VM 置換に耐える。

### 有効化（operator-config 経由を推奨）

operator-config.yaml の `deploy:` に次を追加し、`generate-tfvars` で tfvars を再生成する
（値は例。secret は **ID のみ**）:

```yaml
safety:
  events:
    emit_signed_moderation_events: true
    signing_key_secret_id: kukuri-cn-safety-signing-key
  providers:
    known_csam:
      provider: project-arachnid-shield
      required: true
    general:
      provider: openai-compatible-vlm
    unknown_csam:
      provider: openai-compatible-vlm
deploy:
  # ...既存の設定...
  deploy_indexer_stack: true
  cn_indexer_image: ghcr.io/kingyosun/kukuri-cn-indexer:latest   # 本番は digest 固定
  indexer_data_disk_gb: 10
  relation_analyze_interval_minutes: 60
  channel_secret_key_secret_id: kukuri-cn-channel-secret-key
  arcadedb_password_secret_id: kukuri-cn-arcadedb-password
  arachnid_username_secret_id: kukuri-cn-arachnid-username
  arachnid_password_secret_id: kukuri-cn-arachnid-password
  vlm_api_base_url: http://192.0.2.10:8000   # self-host VLM は private 経路の先のアドレス
  vlm_model: inclusionAI/SingGuard-2b
  vlm_response_format: guard
  # vlm_api_key_secret_id: kukuri-cn-vlm-api-key   # 無認証 self-host endpoint なら省略
```

`validate-config` が runtime の起動 gate（必須 secret ID / relay / provider credential /
署名鍵）を apply 前に fail-closed で検査する。tfvars を手書きする場合も同名の変数を設定する。

### rollout 順序

apply 1 回で startup script が次の順序を machine 的に守る（compose の depends_on / healthcheck）:

1. `cn-migrate`（migration。冪等）
2. Postgres / ArcadeDB ready（ArcadeDB は空 database `kukuri_index` を初回起動時に冪等作成）
3. `cn-indexer` 起動（ArcadeDB schema を `ensure_schema` で冪等作成。provider / 署名鍵の構成不備は
   fail-closed で起動失敗）
4. relation analyze timer 有効化

ユーザー向け read surface はこの後も **既定 false** のまま:
`index_query_enabled` / `trust_read_enabled` は full-stack E2E（後続 issue）完了までは
有効化しない。無効の間、`GET /v1/index/*` / `/v1/trust/*` / `/v1/relation/*` は 404。

### 確認

```bash
terraform output ssh_iap_command   # IAP SSH
# VM 上で:
cd /var/lib/kukuri/community-node
docker ps                                        # cn-arcadedb / cn-indexer が healthy
docker exec $(docker ps -qf name=cn-indexer) curl -fsS http://127.0.0.1:8630/healthz
docker exec $(docker ps -qf name=cn-indexer) curl -fsS http://127.0.0.1:8630/v1/status  # last ingest / backoff 等
systemctl list-timers kukuri-relation-analyze.timer
systemctl status kukuri-relation-analyze.service # relation analyze の last success / failure
journalctl -u kukuri-relation-analyze.service -n 50
```

外部からの port scan で 80/443/`relay_quic_port`/udp 以外（2480 / 8630 / 5432 / 6379）が
閉じていることも確認する（firewall には新規 ingress を追加していない）。

### VLM の private 経路（self-host endpoint）

- self-host VLM（DGX Spark 等の OpenAI-compatible endpoint）は public internet へ直接公開しない。
- GCP VM から WireGuard / VPN / private tunnel 等で到達させ、`vlm_api_base_url` には tunnel 先の
  private アドレスを指定する（例: `http://192.0.2.10:8000`）。WireGuard peer の設定は
  `docs/runbooks/community-node-self-host-vps.md` の WireGuard 節が参考になる（COS では
  Terraform 管理外の手動設定。VM 置換時は再設定が必要）。
- API key が必要な external VLM を使う場合は `vlm_api_key_secret_id` で注入する。
- VLM 不達時は cn-indexer が fail-closed（scan 失敗 = hold。allow に落ちない）。
- Project Arachnid Shield への通信は outbound HTTPS のみで、callback / inbound port は無い。

### ArcadeDB を空から再構築する

ArcadeDB は rebuildable projection（真実源は Postgres + docs replica）であり、canonical store
ではない。data が失われた / 壊れた場合:

```bash
cd /var/lib/kukuri/community-node
/var/lib/toolbox/kukuri/bin/docker-compose stop cn-indexer cn-arcadedb
docker volume rm community-node_cn-arcadedb-data   # volume 名は docker volume ls で確認
/var/lib/toolbox/kukuri/bin/docker-compose up -d cn-arcadedb
/var/lib/toolbox/kukuri/bin/docker-compose up -d cn-indexer
# cn-indexer が起動時に schema を作成し、全件見直し（poll interval、既定 300 秒）で再投影する。
# relation graph は次回 relation analyze 実行で再構築される（手動なら:
#   /var/lib/toolbox/kukuri/bin/docker-compose run --rm cn-relation-analyze ）
```

### rollback（API / relay のみ構成へ戻す）

`deploy_indexer_stack=false` にして apply する（operator-config 経由なら
`deploy: deploy_indexer_stack: false` → `generate-tfvars` → apply）。

- cn-indexer / ArcadeDB / relation timer が構成から外れ、公開 surface は従来どおり
  API / relay のみになる（flag が既定 false のため index / trust API はもともと 404）。
- Postgres の永続 data（verdict / moderation artifact / index 真実源）は残る。
- `indexer_data_disk_gb > 0` の PD は Terraform 管理のため、変数を 0 にしない限り残る
  （再有効化時に endpoint 同一性を保てる）。

## backup / restore

- low-cost の backup は systemd timer（`kukuri-backup.timer`）が `pg_dump -Fc` を取り、
  GCS backup bucket（`terraform output backup_bucket`）へアップロードする。
  database 全体 dump のため、#615 で増えた永続 table（moderation verdict / artifact /
  risk signal / index 真実源）も追加設定なしで含まれる。
- Valkey と blob cache は backup 対象外。ArcadeDB も backup 対象外
  （rebuildable projection。前節の再構築手順で復元する）。raw blob / media は恒久保存しない。

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
- 両者は独立した軸だが、`operator-config.yaml` を単一の入力元にして両方を生成できる（#380）。
  operator-config の `deploy:` セクションがコスト軸、`profile` / `features` が capability 軸。

## operator-config から一気通貫でセットアップ（#380）

`cn-operator` を起点に docs / manifest と terraform env 設定の両方を生成し、デプロイ時に
operator-config.yaml を VM へ配置して public manifest endpoint を有効化できる。

```bash
cd infra/terraform/envs/low-cost

# operator-config.yaml を用意（deploy: セクションを含める）
cn-operator init --out operator-config.yaml
# operator-config.yaml を編集（server / features / deploy を埋める）
cn-operator validate-config --config operator-config.yaml

# 同じ config から terraform.tfvars を生成
cn-operator generate-tfvars --config operator-config.yaml --out terraform.tfvars

cp backend.hcl.example backend.hcl   # state bucket/prefix を埋める
terraform init -backend-config=backend.hcl
terraform plan
terraform apply
```

- `terraform.tfvars` の末尾コメントに従い `operator_config_path = "operator-config.yaml"`
  を有効化すると、Terraform が env dir からの相対 path を読み込み、起動時に
  operator-config.yaml が VM の `/etc/kukuri/operator-config.yaml` へ配置され、
  `cn-user-api` が `COMMUNITY_NODE_OPERATOR_CONFIG` で読み込む。
- これにより `GET /.well-known/kukuri/community-node.json` / `GET /v1/node/manifest` が応答し、
  `report_endpoint` capability を有効化した node では `POST /v1/report` が受理される。
- `operator_config_path` が空（既定）なら manifest endpoint は `404` のまま（従来挙動）。
- blob cache の on/off は operator-config の `features.blob_cache` が真実源で、tfvars の
  `blob_cache_enabled` はそこから導出される。`features.blob_cache: false` なのに
  `deploy.blob_cache_size_gb > 0` を指定すると `validate-config` / `generate-tfvars` が失敗する。
- tfvars 生成は `low-cost` のみ対応。`managed-db` / `ha` は拡張点。

## 関連

- `infra/terraform/README.md`
- `docs/runbooks/community-node-self-host-vps.md`（VPS edge の手動 self-host）
- `docs/runbooks/community-node-operator-docs.md`（cn-operator の文書生成）
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
