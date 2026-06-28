variable "deployment_profile" {
  description = "deployment profile。この root は low-cost 固定。"
  type        = string
  default     = "low-cost"

  validation {
    condition     = var.deployment_profile == "low-cost"
    error_message = "この root は low-cost profile 専用。managed-db / ha は envs/managed-db, envs/ha を使う。"
  }
}

variable "project_id" {
  description = "GCP project ID。"
  type        = string
}

variable "region" {
  description = "GCP region。"
  type        = string
  default     = "asia-northeast1"
}

variable "zone" {
  description = "GCP zone。"
  type        = string
  default     = "asia-northeast1-a"
}

variable "name_prefix" {
  description = "リソース名の接頭辞。"
  type        = string
  default     = "kukuri-cn"
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
  description = "ACME(Let's Encrypt) 登録 email。"
  type        = string
}

variable "manage_cloud_dns" {
  description = "true なら Cloud DNS の既存 zone に A レコードを作成する。false なら static IP を output して手動 DNS。"
  type        = bool
  default     = false
}

variable "dns_zone_name" {
  description = "Cloud DNS managed zone 名（manage_cloud_dns=true のとき必須）。"
  type        = string
  default     = ""
}

# --- VM sizing ---
variable "machine_type" {
  description = "Compute Engine machine type。"
  type        = string
  default     = "e2-small"
}

variable "disk_size_gb" {
  description = "boot/persistent disk サイズ（GB）。"
  type        = number
  default     = 30
}

variable "postgres_data_disk_gb" {
  description = "Postgres data 用の専用 persistent disk サイズ（GB）。0 なら boot disk 上の docker volume を使う（VM 置換でデータ消失リスクあり）。本番では > 0 を推奨。"
  type        = number
  default     = 0
}

# --- container images (public GHCR) ---
variable "cn_user_api_image" {
  description = "cn-user-api の公開 GHCR image。"
  type        = string
}

variable "cn_iroh_relay_image" {
  description = "cn-iroh-relay の公開 GHCR image。"
  type        = string
}

variable "cn_cli_image" {
  description = "cn-cli (migrate) の公開 GHCR image。"
  type        = string
}

# --- secrets (Secret Manager IDs) ---
variable "jwt_secret_id" {
  description = "COMMUNITY_NODE_JWT_SECRET を保持する Secret Manager secret ID。"
  type        = string
}

variable "postgres_password_secret_id" {
  description = "Postgres password を保持する Secret Manager secret ID。"
  type        = string
}

# --- postgres / rendezvous ---
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

variable "rendezvous_key_prefix" {
  description = "COMMUNITY_NODE_RENDEZVOUS_KEY_PREFIX。"
  type        = string
  default     = "cn:rendezvous:v1"
}

# --- rate limit ---
variable "rate_limit_enabled" {
  description = "cn-user-api の rate limit を有効化するか。"
  type        = bool
  default     = true
}

variable "rate_limit_per_second" {
  description = "rate limit per second。"
  type        = number
  default     = 10
}

variable "rate_limit_burst" {
  description = "rate limit burst。"
  type        = number
  default     = 30
}

# --- relay rx limit (optional) ---
variable "iroh_relay_client_rx_bytes_per_second" {
  description = "任意の relay client rx bytes/sec。0 なら未設定。"
  type        = number
  default     = 0
}

variable "iroh_relay_client_rx_max_burst_bytes" {
  description = "任意の relay client rx burst bytes。0 なら未設定。"
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
  description = "blob cache 専用ディスクサイズ（GB）。"
  type        = number
  default     = 0
}

variable "blob_cache_ttl_hours" {
  description = "blob cache TTL（時間）。"
  type        = number
  default     = 24
}

variable "blob_cache_path" {
  description = "blob cache マウント path（backup 対象外）。"
  type        = string
  default     = "/var/lib/kukuri/blob-cache"
}

# --- backup ---
variable "backup_enabled" {
  description = "pg_dump -> GCS backup を有効化するか。"
  type        = bool
  default     = true
}

variable "backup_bucket_name" {
  description = "backup bucket 名。空なら name_prefix から導出。"
  type        = string
  default     = ""
}

variable "backup_retention_days" {
  description = "backup 保持日数。"
  type        = number
  default     = 30
}

variable "backup_force_destroy" {
  description = "terraform destroy 時に backup bucket を中身ごと削除するか。"
  type        = bool
  default     = false
}

variable "enable_disk_snapshots" {
  description = "VM persistent disk の snapshot schedule を有効化するか（任意）。"
  type        = bool
  default     = false
}

variable "snapshot_schedule_days" {
  description = "disk snapshot の保持日数。"
  type        = number
  default     = 14
}

# --- ingress hardening (optional) ---
variable "extra_ingress_source_ranges" {
  description = "API/relay public ingress を絞る場合の許可レンジ。既定は全公開。"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

# --- operator manifest (#380) ---
variable "operator_config_path" {
  description = "operator-config.yaml のパス（この env ディレクトリからの相対パス、例: operator-config.yaml）。空でなければ main.tf が file() で読み込み、VM に配置して cn-user-api の COMMUNITY_NODE_OPERATOR_CONFIG に設定し public manifest endpoint / report_endpoint gating を有効化する。空なら manifest endpoint は 404 のまま。"
  type        = string
  default     = ""
}
