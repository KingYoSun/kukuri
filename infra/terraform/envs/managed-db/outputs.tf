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

output "cloudsql_connection_name" {
  description = "Cloud SQL connection name。"
  value       = module.cloudsql.connection_name
}

output "deployment_profile_summary" {
  description = "この deployment の profile とデータ階層の要約。"
  value = {
    deployment_profile     = var.deployment_profile
    compute                = "Compute Engine VM (${var.machine_type})"
    postgres               = "Cloud SQL (control-plane data; automated backup/PITR)"
    valkey                 = "Memorystore (TTL ephemeral state)"
    blob_cache             = "local cache / iroh blobs (not in Postgres; object storage is ha)"
    managed_db             = "in use"
    index_moderation_trust = "not provisioned (Phase B; rebuildable, kept out of canonical DB)"
  }
}
