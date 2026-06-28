variable "name_prefix" {
  description = "リソース名の接頭辞。"
  type        = string
}

variable "project_id" {
  description = "GCP project ID。"
  type        = string
}

variable "zone" {
  description = "VM を作成する zone。"
  type        = string
}

variable "machine_type" {
  description = "Compute Engine machine type。"
  type        = string
  default     = "e2-small"
}

variable "disk_size_gb" {
  description = "boot/persistent disk サイズ（GB）。Postgres data も同 disk 上に置く。"
  type        = number
  default     = 30
}

variable "boot_image" {
  description = "VM boot image。Docker を含む Container-Optimized OS 既定。"
  type        = string
  default     = "projects/cos-cloud/global/images/family/cos-stable"
}

variable "network_self_link" {
  description = "接続する VPC network の self_link。"
  type        = string
}

variable "subnet_self_link" {
  description = "接続する subnet の self_link。"
  type        = string
}

variable "static_ip" {
  description = "VM に割り当てる static external IP アドレス。"
  type        = string
}

variable "network_tags" {
  description = "firewall を適用する network tag 群。"
  type        = list(string)
  default     = ["kukuri-community-node"]
}

# --- public endpoints / TLS ---
variable "api_domain" {
  description = "cn-user-api の公開 hostname。"
  type        = string
}

variable "relay_domain" {
  description = "cn-iroh-relay の公開 hostname。"
  type        = string
}

variable "acme_email" {
  description = "ACME(Let's Encrypt) 登録に使う email。"
  type        = string
}

variable "relay_quic_port" {
  description = "cn-iroh-relay の QUIC/UDP ポート。"
  type        = number
  default     = 7842
}

# --- container images (public GHCR) ---
variable "cn_user_api_image" {
  description = "cn-user-api の公開 container image（GHCR、tag/digest 込み）。"
  type        = string
}

variable "cn_iroh_relay_image" {
  description = "cn-iroh-relay の公開 container image（GHCR、tag/digest 込み）。"
  type        = string
}

variable "cn_cli_image" {
  description = "cn-cli (migrate) の公開 container image（GHCR、tag/digest 込み）。"
  type        = string
}

variable "caddy_image" {
  description = "Caddy reverse proxy container image。"
  type        = string
  default     = "caddy:2"
}

variable "acme_image" {
  description = "ACME companion (certbot) container image。"
  type        = string
  default     = "certbot/certbot:latest"
}

# --- data tier (low-cost local containers) ---
variable "deploy_local_postgres" {
  description = "true なら VM 上に Postgres container を立てる（low-cost）。false なら external_database_url を使う。"
  type        = bool
  default     = true
}

variable "deploy_local_valkey" {
  description = "true なら VM 上に Valkey container を立てる（low-cost）。false なら external_redis_url を使う。"
  type        = bool
  default     = true
}

variable "postgres_image" {
  description = "local Postgres container image。"
  type        = string
  default     = "postgres:17-bookworm"
}

variable "valkey_image" {
  description = "local Valkey container image。"
  type        = string
  default     = "valkey/valkey:8-alpine"
}

variable "postgres_user" {
  description = "Postgres user 名。"
  type        = string
  default     = "cn"
}

variable "postgres_db" {
  description = "Postgres database 名。"
  type        = string
  default     = "cn"
}

variable "postgres_data_disk_gb" {
  description = "deploy_local_postgres=true のとき、Postgres data 用の専用 persistent disk サイズ（GB）。0 なら boot disk 上の docker volume を使う（VM 置換でデータ消失リスクあり）。"
  type        = number
  default     = 0
}

# 外部 DB（managed-db / ha 拡張点）。password は URL に焼かず、Secret Manager から
# boot 時に取得して URL-encode する。Terraform state / VM metadata に平文を残さない。
variable "external_db_host" {
  description = "deploy_local_postgres=false のときの外部 Postgres host。"
  type        = string
  default     = ""
}

variable "external_db_port" {
  description = "外部 Postgres port。"
  type        = number
  default     = 5432
}

variable "external_db_user" {
  description = "外部 Postgres user。"
  type        = string
  default     = ""
}

variable "external_db_name" {
  description = "外部 Postgres database 名。"
  type        = string
  default     = ""
}

variable "external_db_password_secret_id" {
  description = "外部 Postgres password を保持する Secret Manager secret ID（deploy_local_postgres=false のとき必須）。"
  type        = string
  default     = ""
}

variable "external_db_password_secret_version" {
  description = "外部 Postgres password の version。"
  type        = string
  default     = "latest"
}

variable "external_redis_url" {
  description = "deploy_local_valkey=false のときに使う外部 Redis/Valkey URL（managed-db 拡張点）。"
  type        = string
  default     = ""
}

# --- secrets (Secret Manager IDs, not payloads) ---
variable "jwt_secret_id" {
  description = "COMMUNITY_NODE_JWT_SECRET を保持する Secret Manager secret ID。"
  type        = string
}

variable "jwt_secret_version" {
  description = "JWT secret の version（既定 latest）。"
  type        = string
  default     = "latest"
}

variable "postgres_password_secret_id" {
  description = "Postgres password を保持する Secret Manager secret ID（local Postgres のとき必須）。"
  type        = string
  default     = ""
}

variable "postgres_password_secret_version" {
  description = "Postgres password の version（既定 latest）。"
  type        = string
  default     = "latest"
}

variable "accessor_secret_ids" {
  description = "VM service account に read 権限を付与する Secret Manager secret ID 群。VM 起動前に binding を確実に作るため、instance が depends_on する。"
  type        = list(string)
  default     = []
}

# --- cn-user-api tuning ---
variable "jwt_issuer" {
  description = "COMMUNITY_NODE_JWT_ISSUER。"
  type        = string
  default     = "kukuri-cn"
}

variable "jwt_ttl_seconds" {
  description = "COMMUNITY_NODE_JWT_TTL_SECONDS。"
  type        = number
  default     = 86400
}

variable "rendezvous_key_prefix" {
  description = "COMMUNITY_NODE_RENDEZVOUS_KEY_PREFIX。"
  type        = string
  default     = "cn:rendezvous:v1"
}

variable "rate_limit_enabled" {
  description = "COMMUNITY_NODE_RATE_LIMIT_ENABLED。"
  type        = bool
  default     = true
}

variable "rate_limit_per_second" {
  description = "COMMUNITY_NODE_RATE_LIMIT_PER_SECOND。"
  type        = number
  default     = 10
}

variable "rate_limit_burst" {
  description = "COMMUNITY_NODE_RATE_LIMIT_BURST。"
  type        = number
  default     = 30
}

variable "iroh_relay_client_rx_bytes_per_second" {
  description = "任意の COMMUNITY_NODE_IROH_RELAY_CLIENT_RX_BYTES_PER_SECOND。0 なら未設定。"
  type        = number
  default     = 0
}

variable "iroh_relay_client_rx_max_burst_bytes" {
  description = "任意の COMMUNITY_NODE_IROH_RELAY_CLIENT_RX_MAX_BURST_BYTES。0 なら未設定。"
  type        = number
  default     = 0
}

# --- blob cache (disabled by default) ---
variable "blob_cache_enabled" {
  description = "blob cache を有効化するか。低コスト既定は false。"
  type        = bool
  default     = false
}

variable "blob_cache_size_gb" {
  description = "blob cache 専用ディスクサイズ（GB）。blob_cache_enabled=true のときのみ作成。"
  type        = number
  default     = 0
}

variable "blob_cache_ttl_hours" {
  description = "blob cache の TTL（時間）。env として記録（cache eviction の上限指針）。"
  type        = number
  default     = 24
}

variable "blob_cache_path" {
  description = "blob cache のマウント先 path（backup 対象外）。"
  type        = string
  default     = "/var/lib/kukuri/blob-cache"
}

# --- backup ---
variable "backup_enabled" {
  description = "pg_dump -> GCS backup を有効化するか。"
  type        = bool
  default     = true
}

variable "backup_bucket" {
  description = "backup 先 GCS bucket 名（backup_enabled=true のとき必須）。"
  type        = string
  default     = ""
}

variable "backup_schedule_oncalendar" {
  description = "systemd OnCalendar 形式の backup 実行スケジュール。"
  type        = string
  default     = "*-*-* 03:30:00"
}

# --- monitoring helpers passthrough ---
variable "deployment_profile" {
  description = "deployment profile 名（メタ表示用）。"
  type        = string
  default     = "low-cost"
}

# --- operator manifest (#380) ---
variable "operator_config_file" {
  description = "operator-config.yaml の中身（YAML 文字列）。空でなければ VM に配置し、cn-user-api の COMMUNITY_NODE_OPERATOR_CONFIG に設定して public manifest endpoint / report_endpoint gating を有効化する。空なら manifest endpoint は 404 のまま。"
  type        = string
  default     = ""
}
