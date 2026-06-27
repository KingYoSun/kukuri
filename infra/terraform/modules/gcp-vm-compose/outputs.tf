output "instance_name" {
  description = "Compute Engine instance 名。"
  value       = google_compute_instance.vm.name
}

output "instance_self_link" {
  description = "Compute Engine instance の self_link。"
  value       = google_compute_instance.vm.self_link
}

output "service_account_email" {
  description = "VM service account の email。"
  value       = google_service_account.vm.email
}

output "zone" {
  description = "VM の zone。"
  value       = google_compute_instance.vm.zone
}

output "boot_disk_name" {
  description = "boot disk 名（GCE 既定で instance 名と同じ）。snapshot policy の attach に使う。"
  value       = google_compute_instance.vm.name
}

output "postgres_data_disk_name" {
  description = "Postgres data 専用 disk 名（未作成なら空）。snapshot policy の attach に使う。"
  value       = local.use_postgres_disk ? google_compute_disk.postgres_data[0].name : ""
}

output "ssh_iap_command" {
  description = "IAP 経由の SSH コマンド例。"
  value       = "gcloud compute ssh ${google_compute_instance.vm.name} --zone ${google_compute_instance.vm.zone} --tunnel-through-iap --project ${var.project_id}"
}
