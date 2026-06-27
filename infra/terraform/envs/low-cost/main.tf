provider "google" {
  project = var.project_id
  region  = var.region
  zone    = var.zone
}

locals {
  # 同一 static IP を api/relay 2 hostname が指す。
  dns_records = {
    api = {
      name = "${var.api_domain}."
    }
    relay = {
      name = "${var.relay_domain}."
    }
  }
}

module "network" {
  source = "../../modules/gcp-network"

  name_prefix                 = var.name_prefix
  region                      = var.region
  extra_ingress_source_ranges = var.extra_ingress_source_ranges
}

module "vm" {
  source = "../../modules/gcp-vm-compose"

  name_prefix        = var.name_prefix
  project_id         = var.project_id
  zone               = var.zone
  machine_type       = var.machine_type
  disk_size_gb       = var.disk_size_gb
  network_self_link  = module.network.network_self_link
  subnet_self_link   = module.network.subnet_self_link
  static_ip          = module.network.static_ip
  network_tags       = module.network.network_tags
  deployment_profile = var.deployment_profile

  api_domain   = var.api_domain
  relay_domain = var.relay_domain
  acme_email   = var.acme_email

  cn_user_api_image   = var.cn_user_api_image
  cn_iroh_relay_image = var.cn_iroh_relay_image
  cn_cli_image        = var.cn_cli_image

  # low-cost: local Postgres + Valkey containers
  deploy_local_postgres = true
  deploy_local_valkey   = true
  postgres_user         = var.postgres_user
  postgres_db           = var.postgres_db
  postgres_data_disk_gb = var.postgres_data_disk_gb

  jwt_secret_id               = var.jwt_secret_id
  postgres_password_secret_id = var.postgres_password_secret_id
  accessor_secret_ids         = [var.jwt_secret_id, var.postgres_password_secret_id]

  rendezvous_key_prefix                 = var.rendezvous_key_prefix
  rate_limit_enabled                    = var.rate_limit_enabled
  rate_limit_per_second                 = var.rate_limit_per_second
  rate_limit_burst                      = var.rate_limit_burst
  iroh_relay_client_rx_bytes_per_second = var.iroh_relay_client_rx_bytes_per_second
  iroh_relay_client_rx_max_burst_bytes  = var.iroh_relay_client_rx_max_burst_bytes

  blob_cache_enabled   = var.blob_cache_enabled
  blob_cache_size_gb   = var.blob_cache_size_gb
  blob_cache_ttl_hours = var.blob_cache_ttl_hours
  blob_cache_path      = var.blob_cache_path

  backup_enabled = var.backup_enabled
  backup_bucket  = var.backup_enabled ? module.backup[0].bucket_name : ""
}

module "backup" {
  source = "../../modules/gcp-low-cost-backup"
  count  = var.backup_enabled ? 1 : 0

  name_prefix           = var.name_prefix
  project_id            = var.project_id
  location              = var.region
  bucket_name           = var.backup_bucket_name
  retention_days        = var.backup_retention_days
  service_account_email = module.vm.service_account_email
  force_destroy         = var.backup_force_destroy
}

module "dns" {
  source = "../../modules/gcp-dns"

  manage_cloud_dns = var.manage_cloud_dns
  dns_zone_name    = var.dns_zone_name
  ip_address       = module.network.static_ip
  records          = local.dns_records
}

# 任意: VM boot disk の snapshot schedule。enable_disk_snapshots=true のとき
# resource policy を作成し、VM boot disk に attach する。
resource "google_compute_resource_policy" "disk_snapshot" {
  count   = var.enable_disk_snapshots ? 1 : 0
  name    = "${var.name_prefix}-disk-snapshot"
  project = var.project_id
  region  = var.region

  snapshot_schedule_policy {
    schedule {
      daily_schedule {
        days_in_cycle = 1
        start_time    = "18:00"
      }
    }
    retention_policy {
      max_retention_days    = var.snapshot_schedule_days
      on_source_disk_delete = "KEEP_AUTO_SNAPSHOTS"
    }
  }
}

resource "google_compute_disk_resource_policy_attachment" "boot" {
  count   = var.enable_disk_snapshots ? 1 : 0
  name    = google_compute_resource_policy.disk_snapshot[0].name
  disk    = module.vm.boot_disk_name
  zone    = var.zone
  project = var.project_id
}

resource "google_compute_disk_resource_policy_attachment" "postgres_data" {
  count   = var.enable_disk_snapshots && var.postgres_data_disk_gb > 0 ? 1 : 0
  name    = google_compute_resource_policy.disk_snapshot[0].name
  disk    = module.vm.postgres_data_disk_name
  zone    = var.zone
  project = var.project_id
}
