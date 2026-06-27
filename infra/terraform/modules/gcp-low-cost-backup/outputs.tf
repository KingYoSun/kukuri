output "bucket_name" {
  description = "backup bucket 名。"
  value       = google_storage_bucket.backup.name
}

output "bucket_url" {
  description = "backup bucket の gs:// URL。"
  value       = "gs://${google_storage_bucket.backup.name}"
}
