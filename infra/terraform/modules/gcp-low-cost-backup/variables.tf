variable "name_prefix" {
  description = "リソース名の接頭辞。"
  type        = string
}

variable "project_id" {
  description = "GCP project ID。"
  type        = string
}

variable "location" {
  description = "backup bucket の location（通常は region）。"
  type        = string
}

variable "bucket_name" {
  description = "backup bucket 名。空なら name_prefix から導出する。"
  type        = string
  default     = ""
}

variable "retention_days" {
  description = "backup オブジェクトの保持日数（lifecycle delete）。"
  type        = number
  default     = 30
}

variable "service_account_email" {
  description = "backup を書き込む VM service account の email。"
  type        = string
}

variable "force_destroy" {
  description = "terraform destroy 時に中身ごと bucket を削除するか。"
  type        = bool
  default     = false
}
