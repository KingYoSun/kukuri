provider "google" {
  project = var.project_id
  region  = var.region
  zone    = var.zone
}

locals {
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

  name_prefix                    = var.name_prefix
  region                         = var.region
  enable_private_services_access = true
}

# managed Postgres（control-plane data の canonical store）。private IP で VM から接続する。
module "cloudsql" {
  source = "../../modules/gcp-cloudsql-postgres"

  enabled                   = true
  name_prefix               = var.name_prefix
  project_id                = var.project_id
  region                    = var.region
  tier                      = var.cloudsql_tier
  availability_type         = var.cloudsql_availability_type
  disk_size_gb              = var.cloudsql_disk_size_gb
  database_name             = var.database_name
  database_user             = var.database_user
  database_password         = var.database_password
  deletion_protection       = var.cloudsql_deletion_protection
  private_network_self_link = module.network.network_self_link

  # private services access peering が出来てから instance を作る。
  depends_on = [module.network]
}

# managed cache（rendezvous / presence の TTL ephemeral state）。
module "valkey" {
  source = "../../modules/gcp-managed-valkey"

  enabled                      = true
  name_prefix                  = var.name_prefix
  project_id                   = var.project_id
  region                       = var.region
  tier                         = var.memorystore_tier
  memory_size_gb               = var.memorystore_memory_size_gb
  authorized_network_self_link = module.network.network_self_link
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

  # managed-db: external managed DB + cache を使い、local container は立てない。
  # password は VM metadata に焼かず、boot 時に Secret Manager から取得して URL-encode する。
  deploy_local_postgres               = false
  deploy_local_valkey                 = false
  external_db_host                    = module.cloudsql.private_ip_address
  external_db_port                    = 5432
  external_db_user                    = var.database_user
  external_db_name                    = var.database_name
  external_db_password_secret_id      = var.database_password_secret_id
  external_db_password_secret_version = var.database_password_secret_version
  external_redis_url                  = module.valkey.redis_url

  jwt_secret_id       = var.jwt_secret_id
  accessor_secret_ids = [var.jwt_secret_id, var.database_password_secret_id]

  # managed-db では local Postgres backup timer は不要。
  backup_enabled = false
}

module "dns" {
  source = "../../modules/gcp-dns"

  manage_cloud_dns = var.manage_cloud_dns
  dns_zone_name    = var.dns_zone_name
  ip_address       = module.network.static_ip
  records          = local.dns_records
}
