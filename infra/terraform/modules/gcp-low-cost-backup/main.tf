terraform {
  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

locals {
  bucket_name = var.bucket_name != "" ? var.bucket_name : "${var.name_prefix}-pg-backup"
}

# low-cost profile の backup 先。Postgres の pg_dump のみを置く。
# Valkey / blob cache は backup 対象外（plan の方針どおり）。
resource "google_storage_bucket" "backup" {
  name                        = local.bucket_name
  project                     = var.project_id
  location                    = var.location
  force_destroy               = var.force_destroy
  uniform_bucket_level_access = true

  lifecycle_rule {
    action {
      type = "Delete"
    }
    condition {
      age = var.retention_days
    }
  }

  # backup の上書き/誤削除に備えて versioning を有効化する。
  versioning {
    enabled = true
  }
}

# VM は backup object を作成するだけでよい。objectAdmin（delete 可）にすると、
# VM 侵害時に既存 backup を全消去できてしまうため、append/write のみの objectCreator にする。
resource "google_storage_bucket_iam_member" "writer" {
  bucket = google_storage_bucket.backup.name
  role   = "roles/storage.objectCreator"
  member = "serviceAccount:${var.service_account_email}"
}
