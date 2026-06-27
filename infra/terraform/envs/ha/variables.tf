variable "deployment_profile" {
  description = "deployment profile。この root は ha 固定。"
  type        = string
  default     = "ha"

  validation {
    condition     = var.deployment_profile == "ha"
    error_message = "この root は ha profile 専用。"
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
  default     = "kukuri-cn-ha"
}

variable "api_domain" {
  description = "cn-user-api の公開 hostname。"
  type        = string
}

variable "relay_domain" {
  description = "cn-iroh-relay の公開 hostname。"
  type        = string
}

variable "acme_email" {
  description = "ACME 登録 email。"
  type        = string
}

variable "manage_cloud_dns" {
  description = "Cloud DNS に A レコードを作成するか。"
  type        = bool
  default     = false
}

variable "dns_zone_name" {
  description = "Cloud DNS managed zone 名。"
  type        = string
  default     = ""
}

variable "machine_type" {
  description = "Compute Engine machine type。"
  type        = string
  default     = "e2-standard-2"
}

variable "disk_size_gb" {
  description = "boot disk サイズ（GB）。"
  type        = number
  default     = 50
}

variable "cn_user_api_image" {
  description = "cn-user-api の公開 GHCR image。"
  type        = string
}

variable "cn_iroh_relay_image" {
  description = "cn-iroh-relay の公開 GHCR image。"
  type        = string
}

variable "cn_cli_image" {
  description = "cn-cli の公開 GHCR image。"
  type        = string
}

variable "jwt_secret_id" {
  description = "JWT secret の Secret Manager secret ID。"
  type        = string
}

# --- Cloud SQL (HA) ---
variable "cloudsql_tier" {
  description = "Cloud SQL machine tier。"
  type        = string
  default     = "db-custom-2-7680"
}

variable "cloudsql_disk_size_gb" {
  description = "Cloud SQL データディスクサイズ。"
  type        = number
  default     = 50
}

variable "database_name" {
  description = "database 名。"
  type        = string
  default     = "cn"
}

variable "database_user" {
  description = "database user 名。"
  type        = string
  default     = "cn"
}

variable "database_password" {
  description = "Cloud SQL user 作成用の password。TF_VAR 経由で渡す（tfvars に書かない）。VM metadata には焼かない。"
  type        = string
  default     = ""
  sensitive   = true
}

variable "database_password_secret_id" {
  description = "VM が boot 時に DB password を取得する Secret Manager secret ID。database_password と同じ値を事前に登録する。"
  type        = string
}

variable "database_password_secret_version" {
  description = "DB password secret の version。"
  type        = string
  default     = "latest"
}

# --- Memorystore (HA) ---
variable "memorystore_memory_size_gb" {
  description = "Memorystore メモリサイズ（GB）。"
  type        = number
  default     = 2
}

# --- blob storage / CDN ---
variable "blob_storage_enabled" {
  description = "blob/media 用 object storage を作成するか。"
  type        = bool
  default     = true
}

variable "blob_cache_ttl_days" {
  description = "blob cache TTL（日）。rebuildable cache 前提。"
  type        = number
  default     = 7
}
