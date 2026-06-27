variable "enabled" {
  description = "true のとき Memorystore (Redis/Valkey 互換) を作成する。low-cost では false。"
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
  description = "Memorystore instance の region。"
  type        = string
}

variable "tier" {
  description = "Memorystore tier。BASIC または STANDARD_HA。"
  type        = string
  default     = "BASIC"

  validation {
    condition     = contains(["BASIC", "STANDARD_HA"], var.tier)
    error_message = "tier は BASIC または STANDARD_HA を指定する。"
  }
}

variable "memory_size_gb" {
  description = "Memorystore のメモリサイズ（GB）。"
  type        = number
  default     = 1
}

variable "authorized_network_self_link" {
  description = "接続を許可する VPC network self_link（空なら default network）。"
  type        = string
  default     = ""
}

variable "redis_version" {
  description = "Memorystore の Redis バージョン。"
  type        = string
  default     = "REDIS_7_2"
}
