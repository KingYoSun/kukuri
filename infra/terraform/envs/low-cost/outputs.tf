output "static_ip" {
  description = "VM の static external IP。manage_cloud_dns=false なら手動 DNS でこの IP に api/relay を向ける。"
  value       = module.network.static_ip
}

output "api_base_url" {
  description = "cn-user-api の公開 URL。"
  value       = "https://${var.api_domain}"
}

output "relay_connectivity_url" {
  description = "client へ配る connectivity URL（cn-iroh-relay）。"
  value       = "https://${var.relay_domain}"
}

output "ssh_iap_command" {
  description = "IAP 経由 SSH コマンド。"
  value       = module.vm.ssh_iap_command
}

output "backup_bucket" {
  description = "backup bucket 名（backup 無効なら空）。"
  value       = var.backup_enabled ? module.backup[0].bucket_name : ""
}

output "deployment_profile_summary" {
  description = "この deployment の profile とデータ階層の要約。"
  value = {
    deployment_profile     = var.deployment_profile
    compute                = "Compute Engine VM (${var.machine_type})"
    postgres               = "node-local container (control-plane data only)"
    valkey                 = "node-local container (TTL ephemeral state, not backed up)"
    blob_cache             = var.blob_cache_enabled ? "dedicated path ${var.blob_cache_path} (not in Postgres, not backed up)" : "disabled"
    backup                 = var.backup_enabled ? "pg_dump -> GCS (retention ${var.backup_retention_days}d)" : "disabled"
    managed_db             = "not used (extension point: envs/managed-db)"
    managed_cache          = "not used (extension point: envs/managed-db)"
    index_moderation_trust = "not provisioned (Phase B; kept out of low-cost DB)"
  }
}

output "operator_profile_notes" {
  description = "deployment profile と cn-operator capability profile の違いに関する注記。"
  value = join(" ", [
    "deployment_profile (low-cost/managed-db/ha) はインフラのコスト/データ階層の軸。",
    "cn-operator の capability profile (minimal/relay-enabled/full-service) は",
    "開示・manifest 用の機能の軸であり、別物。operator-config.yaml は別途用意する。",
  ])
}
