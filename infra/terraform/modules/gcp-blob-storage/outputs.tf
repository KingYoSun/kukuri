output "enabled" {
  description = "この module が blob bucket を作成したか。"
  value       = var.enabled
}

output "bucket_name" {
  description = "blob cache bucket 名（未作成なら空）。"
  value       = var.enabled ? google_storage_bucket.blob_cache[0].name : ""
}

output "bucket_url" {
  description = "blob cache bucket の gs:// URL（未作成なら空）。"
  value       = var.enabled ? "gs://${google_storage_bucket.blob_cache[0].name}" : ""
}
