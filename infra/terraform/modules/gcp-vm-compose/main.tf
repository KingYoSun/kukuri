terraform {
  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

locals {
  install_dir = "/var/lib/kukuri/community-node"
  certs_dir   = "/var/lib/kukuri/certs" # host path（certbot が書き込む）
  certs_mount = "/certs"                # container 内のマウント先
  # api/relay を 1 つの SAN 証明書 lineage にまとめる。初回発行と更新で同じ lineage を
  # 使うことで、relay 用 PEM が更新されず期限切れになる事故を防ぐ。
  cert_name = var.api_domain

  # ポートは single source of truth として local に集約し、全 template へ渡す。
  # api/relay-http は内部固定ポート、relay-quic の内部 bind も固定 7842。
  api_port                 = 8080
  relay_http_port          = 3340
  relay_quic_internal_port = 7842

  # Postgres data は boot disk 上の docker named volume ではなく、別 PD に置けるようにする
  # （VM 置換で boot disk が消えても data を残すため）。
  postgres_data_path  = "/var/lib/kukuri/postgres"
  use_postgres_disk   = var.deploy_local_postgres && var.postgres_data_disk_gb > 0
  use_blob_cache_disk = var.blob_cache_enabled && var.blob_cache_size_gb > 0

  # operator-config.yaml を VM に配置するか（#380）。空文字なら配置せず manifest endpoint は 404。
  operator_config_enabled = trimspace(var.operator_config_file) != ""
  operator_config_path    = "/etc/kukuri/operator-config.yaml"
  operator_config_b64     = local.operator_config_enabled ? base64encode(var.operator_config_file) : ""

  # 各テンプレートを base64 で metadata に渡し、startup script が展開する。
  compose_b64 = base64encode(replace(templatefile("${path.module}/templates/docker-compose.yml.tftpl", {
    deploy_local_postgres    = var.deploy_local_postgres
    deploy_local_valkey      = var.deploy_local_valkey
    postgres_image           = var.postgres_image
    valkey_image             = var.valkey_image
    cn_user_api_image        = var.cn_user_api_image
    cn_iroh_relay_image      = var.cn_iroh_relay_image
    cn_cli_image             = var.cn_cli_image
    caddy_image              = var.caddy_image
    postgres_user            = var.postgres_user
    postgres_db              = var.postgres_db
    relay_quic_port          = var.relay_quic_port
    relay_quic_internal_port = local.relay_quic_internal_port
    api_port                 = local.api_port
    relay_http_port          = local.relay_http_port
    certs_mount              = local.certs_mount
    certs_dir                = local.certs_dir
    use_postgres_disk        = local.use_postgres_disk
    postgres_data_path       = local.postgres_data_path
    blob_cache_enabled       = var.blob_cache_enabled
    blob_cache_path          = var.blob_cache_path
    operator_config_enabled  = local.operator_config_enabled
    operator_config_path     = local.operator_config_path
  }), "\r\n", "\n"))

  caddyfile_b64 = base64encode(replace(templatefile("${path.module}/templates/Caddyfile.tftpl", {
    api_domain      = var.api_domain
    relay_domain    = var.relay_domain
    certs_mount     = local.certs_mount
    cert_name       = local.cert_name
    api_port        = local.api_port
    relay_http_port = local.relay_http_port
  }), "\r\n", "\n"))

  env_runtime_b64 = base64encode(replace(templatefile("${path.module}/templates/community-node.env.tftpl", {
    rendezvous_key_prefix                 = var.rendezvous_key_prefix
    api_domain                            = var.api_domain
    relay_domain                          = var.relay_domain
    jwt_issuer                            = var.jwt_issuer
    jwt_ttl_seconds                       = var.jwt_ttl_seconds
    rate_limit_enabled                    = var.rate_limit_enabled
    rate_limit_per_second                 = var.rate_limit_per_second
    rate_limit_burst                      = var.rate_limit_burst
    api_port                              = local.api_port
    relay_http_port                       = local.relay_http_port
    relay_quic_internal_port              = local.relay_quic_internal_port
    iroh_relay_client_rx_bytes_per_second = var.iroh_relay_client_rx_bytes_per_second
    iroh_relay_client_rx_max_burst_bytes  = var.iroh_relay_client_rx_max_burst_bytes
    blob_cache_enabled                    = var.blob_cache_enabled
    blob_cache_ttl_hours                  = var.blob_cache_ttl_hours
    blob_cache_path                       = var.blob_cache_path
    certs_mount                           = local.certs_mount
    cert_name                             = local.cert_name
  }), "\r\n", "\n"))

  backup_script_b64 = base64encode(replace(templatefile("${path.module}/templates/backup.sh.tftpl", {
    install_dir   = local.install_dir
    backup_bucket = var.backup_bucket
    postgres_user = var.postgres_user
    postgres_db   = var.postgres_db
  }), "\r\n", "\n"))

  renew_script_b64 = base64encode(replace(templatefile("${path.module}/templates/renew-certs.sh.tftpl", {
    install_dir  = local.install_dir
    certs_dir    = local.certs_dir
    acme_image   = var.acme_image
    api_domain   = var.api_domain
    relay_domain = var.relay_domain
    acme_email   = var.acme_email
    cert_name    = local.cert_name
  }), "\r\n", "\n"))

  startup_script = replace(templatefile("${path.module}/templates/startup.sh.tftpl", {
    install_dir           = local.install_dir
    certs_dir             = local.certs_dir
    acme_image            = var.acme_image
    api_domain            = var.api_domain
    relay_domain          = var.relay_domain
    acme_email            = var.acme_email
    deploy_local_postgres = var.deploy_local_postgres
    deploy_local_valkey   = var.deploy_local_valkey
    postgres_user         = var.postgres_user
    postgres_db           = var.postgres_db
    cert_name             = local.cert_name
    use_postgres_disk     = local.use_postgres_disk
    postgres_data_path    = local.postgres_data_path
    use_blob_cache_disk   = local.use_blob_cache_disk
    # 外部 DB（managed-db/ha）は password を metadata に焼かず、boot 時に Secret Manager から取得する。
    external_db_host                    = var.external_db_host
    external_db_port                    = var.external_db_port
    external_db_user                    = var.external_db_user
    external_db_name                    = var.external_db_name
    external_db_password_secret_id      = var.external_db_password_secret_id
    external_db_password_secret_version = var.external_db_password_secret_version
    external_redis_url                  = var.external_redis_url
    postgres_password_secret_id         = var.postgres_password_secret_id
    postgres_password_secret_version    = var.postgres_password_secret_version
    jwt_secret_id                       = var.jwt_secret_id
    jwt_secret_version                  = var.jwt_secret_version
    blob_cache_enabled                  = var.blob_cache_enabled
    blob_cache_path                     = var.blob_cache_path
    backup_enabled                      = var.backup_enabled
    backup_schedule_oncalendar          = var.backup_schedule_oncalendar
    operator_config_enabled             = local.operator_config_enabled
    operator_config_path                = local.operator_config_path
    operator_config_b64                 = local.operator_config_b64
    compose_b64                         = local.compose_b64
    caddyfile_b64                       = local.caddyfile_b64
    env_runtime_b64                     = local.env_runtime_b64
    backup_script_b64                   = local.backup_script_b64
    renew_script_b64                    = local.renew_script_b64
  }), "\r\n", "\n")
}

resource "google_service_account" "vm" {
  account_id   = "${var.name_prefix}-vm"
  display_name = "kukuri community node VM (${var.deployment_profile})"
  project      = var.project_id
}

# Secret Manager 取得 / GCS backup を VM が行うため最小権限を付与する。
resource "google_project_iam_member" "logging" {
  project = var.project_id
  role    = "roles/logging.logWriter"
  member  = "serviceAccount:${google_service_account.vm.email}"
}

# secret accessor binding を VM 起動前に確実に作るため module 内に置き、
# instance が depends_on する。
resource "google_secret_manager_secret_iam_member" "accessor" {
  for_each = toset(var.accessor_secret_ids)

  project   = var.project_id
  secret_id = each.value
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.vm.email}"
}

resource "google_compute_disk" "blob_cache" {
  count = var.blob_cache_enabled && var.blob_cache_size_gb > 0 ? 1 : 0

  name    = "${var.name_prefix}-blob-cache"
  type    = "pd-standard"
  zone    = var.zone
  size    = var.blob_cache_size_gb
  project = var.project_id
}

# Postgres data 専用 PD。VM が置換されても data を残すため auto_delete=false で attach し、
# 誤削除を防ぐため prevent_destroy する。
resource "google_compute_disk" "postgres_data" {
  count = local.use_postgres_disk ? 1 : 0

  name    = "${var.name_prefix}-postgres-data"
  type    = "pd-ssd"
  zone    = var.zone
  size    = var.postgres_data_disk_gb
  project = var.project_id

  lifecycle {
    prevent_destroy = true
  }
}

resource "google_compute_instance" "vm" {
  name         = "${var.name_prefix}-vm"
  project      = var.project_id
  zone         = var.zone
  machine_type = var.machine_type
  tags         = var.network_tags

  boot_disk {
    initialize_params {
      image = var.boot_image
      size  = var.disk_size_gb
    }
  }

  dynamic "attached_disk" {
    for_each = local.use_blob_cache_disk ? [1] : []
    content {
      source      = google_compute_disk.blob_cache[0].self_link
      device_name = "blob-cache"
    }
  }

  dynamic "attached_disk" {
    for_each = local.use_postgres_disk ? [1] : []
    content {
      source      = google_compute_disk.postgres_data[0].self_link
      device_name = "postgres-data"
    }
  }

  network_interface {
    network    = var.network_self_link
    subnetwork = var.subnet_self_link

    access_config {
      nat_ip = var.static_ip
    }
  }

  service_account {
    email  = google_service_account.vm.email
    scopes = ["cloud-platform"]
  }

  metadata = {
    enable-oslogin = "TRUE"
  }

  metadata_startup_script = local.startup_script

  allow_stopping_for_update = true

  # secret accessor binding と logging 権限が VM 起動前に存在することを保証する。
  depends_on = [
    google_secret_manager_secret_iam_member.accessor,
    google_project_iam_member.logging,
  ]
}
