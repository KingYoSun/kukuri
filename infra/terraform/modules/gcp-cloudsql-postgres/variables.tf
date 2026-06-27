variable "enabled" {
  description = "true のとき Cloud SQL for PostgreSQL を作成する。low-cost では false。"
  type        = bool
  default     = true
}

variable "name_prefix" {
  description = "リソース名の接頭辞。"
  type        = string
}

variable "project_id" {
  description = "GCP project ID。"
  type        = string
}

variable "region" {
  description = "Cloud SQL instance の region。"
  type        = string
}

variable "database_version" {
  description = "Cloud SQL の PostgreSQL バージョン。"
  type        = string
  default     = "POSTGRES_17"
}

variable "tier" {
  description = "Cloud SQL machine tier。"
  type        = string
  default     = "db-custom-1-3840"
}

variable "availability_type" {
  description = "ZONAL または REGIONAL（REGIONAL は HA）。"
  type        = string
  default     = "ZONAL"

  validation {
    condition     = contains(["ZONAL", "REGIONAL"], var.availability_type)
    error_message = "availability_type は ZONAL または REGIONAL を指定する。"
  }
}

variable "disk_size_gb" {
  description = "Cloud SQL データディスクサイズ。"
  type        = number
  default     = 20
}

variable "database_name" {
  description = "作成する database 名。"
  type        = string
  default     = "cn"
}

variable "database_user" {
  description = "作成する database user 名。"
  type        = string
  default     = "cn"
}

variable "database_password" {
  description = "database user のパスワード。Secret Manager から渡す想定で、tfvars には書かない。"
  type        = string
  default     = ""
  sensitive   = true
}

variable "backup_enabled" {
  description = "自動バックアップを有効化するか。"
  type        = bool
  default     = true
}

variable "point_in_time_recovery_enabled" {
  description = "PITR（WAL archiving）を有効化するか。"
  type        = bool
  default     = true
}

variable "private_network_self_link" {
  description = "private IP 用に接続する VPC network self_link（空なら private IP を設定しない）。"
  type        = string
  default     = ""
}

variable "deletion_protection" {
  description = "instance の削除保護。"
  type        = bool
  default     = true
}
