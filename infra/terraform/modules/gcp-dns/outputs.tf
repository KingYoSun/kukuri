output "managed_record_fqdns" {
  description = "作成した A レコードの FQDN 群（manage_cloud_dns=false なら空）。"
  value       = [for r in google_dns_record_set.a : r.name]
}
