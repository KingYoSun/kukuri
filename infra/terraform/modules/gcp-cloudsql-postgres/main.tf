terraform {
  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

# managed-db / ha profile 用の拡張点。low-cost profile からは enabled=false で
# 何も作成しない。control-plane data（auth/consent/admission/report metadata/
# operator config）のみを保持する canonical Postgres を managed に寄せるための module。
resource "google_sql_database_instance" "this" {
  count = var.enabled ? 1 : 0

  name                = "${var.name_prefix}-pg"
  project             = var.project_id
  region              = var.region
  database_version    = var.database_version
  deletion_protection = var.deletion_protection

  settings {
    tier              = var.tier
    availability_type = var.availability_type
    disk_size         = var.disk_size_gb
    disk_autoresize   = true

    backup_configuration {
      enabled                        = var.backup_enabled
      point_in_time_recovery_enabled = var.point_in_time_recovery_enabled
    }

    ip_configuration {
      ipv4_enabled    = var.private_network_self_link == "" ? true : false
      private_network = var.private_network_self_link == "" ? null : var.private_network_self_link
    }
  }
}

resource "google_sql_database" "this" {
  count    = var.enabled ? 1 : 0
  name     = var.database_name
  project  = var.project_id
  instance = google_sql_database_instance.this[0].name
}

resource "google_sql_user" "this" {
  count    = var.enabled ? 1 : 0
  name     = var.database_user
  project  = var.project_id
  instance = google_sql_database_instance.this[0].name
  password = var.database_password
}
