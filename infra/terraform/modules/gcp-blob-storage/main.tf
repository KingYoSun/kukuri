terraform {
  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

locals {
  bucket_name = var.bucket_name != "" ? var.bucket_name : "${var.name_prefix}-blob-cache"
}

# ha / 大規模 profile 用の拡張点。blob/media の本体は Postgres に置かず、
# rebuildable cache として object storage に置く。canonical DB には混ぜない。
# low-cost / managed-db では enabled=false（local cache / iroh blobs を使う）。
resource "google_storage_bucket" "blob_cache" {
  count = var.enabled ? 1 : 0

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
      age = var.cache_ttl_days
    }
  }
}

resource "google_storage_bucket_iam_member" "rw" {
  count = var.enabled && var.service_account_email != "" ? 1 : 0

  bucket = google_storage_bucket.blob_cache[0].name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${var.service_account_email}"
}
