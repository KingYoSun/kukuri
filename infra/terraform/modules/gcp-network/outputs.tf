output "static_ip" {
  description = "確保した static external IP アドレス。"
  value       = google_compute_address.this.address
}

output "static_ip_self_link" {
  description = "static external IP の self_link。"
  value       = google_compute_address.this.self_link
}

output "network_self_link" {
  description = "使用する VPC network の self_link。"
  value       = local.network_self_link
}

output "network_name" {
  description = "使用する VPC network 名。"
  value       = local.network_name
}

output "subnet_self_link" {
  description = "使用する subnet の self_link。"
  value       = var.create_network ? google_compute_subnetwork.this[0].self_link : data.google_compute_subnetwork.existing[0].self_link
}

output "network_tags" {
  description = "firewall を適用する network tag 群。"
  value       = var.network_tags
}

output "private_services_connection" {
  description = "private services access connection（有効時のみ）。Cloud SQL の依存順序付けに使う。"
  value       = var.enable_private_services_access ? google_service_networking_connection.private_services[0].id : ""
}
