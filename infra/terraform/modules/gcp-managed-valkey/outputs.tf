output "enabled" {
  description = "この module が Memorystore を作成したか。"
  value       = var.enabled
}

output "host" {
  description = "Memorystore host（未作成なら空）。"
  value       = var.enabled ? google_redis_instance.this[0].host : ""
}

output "port" {
  description = "Memorystore port（未作成なら 0）。"
  value       = var.enabled ? google_redis_instance.this[0].port : 0
}

output "redis_url" {
  description = "rendezvous 用 redis URL（未作成なら空）。"
  value       = var.enabled ? "redis://${google_redis_instance.this[0].host}:${google_redis_instance.this[0].port}/" : ""
}
