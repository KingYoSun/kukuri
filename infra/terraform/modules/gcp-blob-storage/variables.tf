variable "enabled" {
  description = "true のとき blob/media 用 object storage (GCS) を作成する。low-cost では false。"
  type        = bool
  default     = false
}

variable "name_prefix" {
  description = "リソース名の接頭辞。"
  type        = string
}

variable "project_id" {
  description = "GCP project ID。"
  type        = string
}

variable "location" {
  description = "blob bucket の location。"
  type        = string
}

variable "bucket_name" {
  description = "blob bucket 名。空なら name_prefix から導出する。"
  type        = string
  default     = ""
}

variable "cache_ttl_days" {
  description = "blob cache オブジェクトの TTL（lifecycle delete）。canonical store ではなく rebuildable cache 前提。"
  type        = number
  default     = 7
}

variable "service_account_email" {
  description = "blob cache を読み書きする service account の email。"
  type        = string
  default     = ""
}

variable "force_destroy" {
  description = "terraform destroy 時に中身ごと bucket を削除するか。"
  type        = bool
  default     = true
}
