output "static_ip" {
  description = "VM の static external IP。"
  value       = module.network.static_ip
}

output "api_base_url" {
  description = "cn-user-api の公開 URL。"
  value       = "https://${var.api_domain}"
}

output "relay_connectivity_url" {
  description = "client へ配る connectivity URL。"
  value       = "https://${var.relay_domain}"
}

output "ssh_iap_command" {
  description = "IAP 経由 SSH コマンド。"
  value       = module.vm.ssh_iap_command
}

output "blob_bucket" {
  description = "blob/media object storage bucket（無効なら空）。"
  value       = module.blob_storage.bucket_name
}

output "deployment_profile_summary" {
  description = "この deployment の profile とデータ階層の要約。"
  value = {
    deployment_profile     = var.deployment_profile
    compute                = "Compute Engine VM (${var.machine_type})"
    postgres               = "Cloud SQL REGIONAL (Multi-AZ HA, PITR)"
    valkey                 = "Memorystore STANDARD_HA"
    blob_cache             = var.blob_storage_enabled ? "GCS object storage (rebuildable cache; not in Postgres)" : "disabled"
    managed_db             = "in use (HA)"
    index_moderation_trust = "not provisioned (Phase B; rebuildable, kept out of canonical DB)"
    note                   = "high-cost profile; not the third-party operator default"
  }
}
