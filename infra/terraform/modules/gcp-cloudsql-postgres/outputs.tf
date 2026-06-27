output "enabled" {
  description = "この module が Cloud SQL を作成したか。"
  value       = var.enabled
}

output "instance_name" {
  description = "Cloud SQL instance 名（未作成なら空）。"
  value       = var.enabled ? google_sql_database_instance.this[0].name : ""
}

output "connection_name" {
  description = "Cloud SQL connection name（Cloud SQL Auth Proxy 用、未作成なら空）。"
  value       = var.enabled ? google_sql_database_instance.this[0].connection_name : ""
}

output "private_ip_address" {
  description = "private IP（設定時のみ、未作成なら空）。"
  value       = var.enabled ? google_sql_database_instance.this[0].private_ip_address : ""
}

output "public_ip_address" {
  description = "public IP（ipv4_enabled 時のみ、未作成なら空）。"
  value       = var.enabled ? google_sql_database_instance.this[0].public_ip_address : ""
}
