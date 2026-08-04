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
  description = "Compute Engine machine type。Postgres + Valkey + ArcadeDB(JVM) + cn-indexer の同居を想定した既定。API / relay のみの最小構成なら e2-small へ下げてもよい。"
  type        = string
  default     = "e2-medium"
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

# --- index / moderation stack (#615) ---
# secret 系はすべて Secret Manager の ID（値ではない）。terraform.tfvars は
# `cn-operator generate-tfvars` が operator-config.yaml から生成する。
variable "deploy_indexer_stack" {
  description = "true なら cn-indexer + ArcadeDB + relation 定期解析を配備する。false へ戻すと従来の API / relay のみ構成（rollback 経路）。"
  type        = bool
  default     = false
}

variable "cn_indexer_image" {
  description = "cn-indexer の公開 GHCR image（tag/digest 込み。production は digest 固定を推奨）。"
  type        = string
  default     = "ghcr.io/kingyosun/kukuri-cn-indexer:latest"
}

variable "arcadedb_image" {
  description = "ArcadeDB container image（stable tag を使う。latest は SNAPSHOT）。"
  type        = string
  default     = "arcadedata/arcadedb:26.8.1"
}

variable "indexer_data_disk_gb" {
  description = "cn-indexer data 用の専用 persistent disk サイズ（GB）。0 なら boot disk 上の docker volume。本番では > 0 を推奨。"
  type        = number
  default     = 0
}

variable "relation_analyze_interval_minutes" {
  description = "cn-cli relation analyze の実行間隔（分）。"
  type        = number
  default     = 60
}

variable "indexer_own_relay" {
  description = "COMMUNITY_NODE_INDEXER_OWN_RELAY。low-cost は cn-iroh-relay を同梱するため既定 true。"
  type        = bool
  default     = true
}

variable "indexer_external_relay_urls" {
  description = "cn-indexer が使う外部 relay URL 群。"
  type        = list(string)
  default     = []
}

variable "index_query_enabled" {
  description = "COMMUNITY_NODE_INDEX_QUERY_ENABLED。full-stack E2E 完了までは false を維持する。"
  type        = bool
  default     = false
}

variable "trust_read_enabled" {
  description = "COMMUNITY_NODE_TRUST_READ_ENABLED。full-stack E2E 完了までは false を維持する。"
  type        = bool
  default     = false
}

variable "channel_secret_key_secret_id" {
  description = "COMMUNITY_NODE_CHANNEL_SECRET_KEY を保持する Secret Manager secret ID（deploy_indexer_stack=true のとき必須）。"
  type        = string
  default     = ""
}

variable "arcadedb_password_secret_id" {
  description = "ArcadeDB root password を保持する Secret Manager secret ID（deploy_indexer_stack=true のとき必須）。"
  type        = string
  default     = ""
}

variable "safety_signing_key_secret_id" {
  description = "moderation event signing key を保持する Secret Manager secret ID。空なら注入しない。"
  type        = string
  default     = ""
}

variable "arachnid_username_secret_id" {
  description = "PROJECT_ARACHNID_API_USERNAME を保持する Secret Manager secret ID。空なら注入しない。"
  type        = string
  default     = ""
}

variable "arachnid_password_secret_id" {
  description = "PROJECT_ARACHNID_API_PASSWORD を保持する Secret Manager secret ID。空なら注入しない。"
  type        = string
  default     = ""
}

variable "vlm_api_key_secret_id" {
  description = "任意の COMMUNITY_NODE_VLM_API_KEY を保持する Secret Manager secret ID。空なら注入しない。"
  type        = string
  default     = ""
}

variable "safety_provider_known_csam" {
  description = "known_csam slot の provider 名（例: project-arachnid-shield）。空なら未構成。"
  type        = string
  default     = ""
}

variable "safety_provider_known_csam_required" {
  description = "known_csam slot の required 宣言。"
  type        = bool
  default     = false
}

variable "safety_provider_general" {
  description = "general slot の provider 名（例: openai-compatible-vlm）。空なら未構成。"
  type        = string
  default     = ""
}

variable "safety_provider_general_required" {
  description = "general slot の required 宣言。"
  type        = bool
  default     = false
}

variable "safety_provider_unknown_csam" {
  description = "unknown_csam slot の provider 名（例: openai-compatible-vlm）。空なら未構成。"
  type        = string
  default     = ""
}

variable "safety_provider_unknown_csam_required" {
  description = "unknown_csam slot の required 宣言。"
  type        = bool
  default     = false
}

variable "safety_emit_signed_events" {
  description = "COMMUNITY_NODE_SAFETY_EMIT_SIGNED_EVENTS。"
  type        = bool
  default     = true
}

variable "safety_suspected_threshold" {
  description = "COMMUNITY_NODE_SAFETY_SUSPECTED_THRESHOLD（1-100）。0 なら未設定。"
  type        = number
  default     = 0
}

variable "safety_suspected_signal_visibility" {
  description = "COMMUNITY_NODE_SAFETY_SUSPECTED_SIGNAL_VISIBILITY。空なら未設定。"
  type        = string
  default     = ""
}

variable "media_fetch_max_bytes" {
  description = "COMMUNITY_NODE_MEDIA_FETCH_MAX_BYTES。0 なら未設定。"
  type        = number
  default     = 0
}

variable "media_fetch_timeout_secs" {
  description = "COMMUNITY_NODE_MEDIA_FETCH_TIMEOUT_SECS。0 なら未設定。"
  type        = number
  default     = 0
}

variable "vlm_api_base_url" {
  description = "COMMUNITY_NODE_VLM_API_BASE_URL。self-host endpoint は private 経路（WireGuard 等）で到達させる。空なら未設定。"
  type        = string
  default     = ""
}

variable "vlm_model" {
  description = "COMMUNITY_NODE_VLM_MODEL。空なら未設定。"
  type        = string
  default     = ""
}

variable "vlm_response_format" {
  description = "COMMUNITY_NODE_VLM_RESPONSE_FORMAT（json / guard）。空なら未設定。"
  type        = string
  default     = ""
}

variable "vlm_api_timeout_secs" {
  description = "COMMUNITY_NODE_VLM_API_TIMEOUT_SECS。0 なら未設定。"
  type        = number
  default     = 0
}

# --- operator manifest (#380) ---
variable "operator_config_path" {
  description = "operator-config.yaml のパス（この env ディレクトリからの相対パス、例: operator-config.yaml）。空でなければ main.tf が file() で読み込み、VM に配置して cn-user-api の COMMUNITY_NODE_OPERATOR_CONFIG に設定し public manifest endpoint / report_endpoint gating を有効化する。空なら manifest endpoint は 404 のまま。"
  type        = string
  default     = ""
}
