terraform {
  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

# managed-db / ha profile 用の拡張点。rendezvous / presence / short-lived
# connection hints の TTL 付き ephemeral state 用 cache を managed に寄せるための module。
# low-cost profile からは enabled=false で何も作成しない（VM 上の Valkey container を使う）。
resource "google_redis_instance" "this" {
  count = var.enabled ? 1 : 0

  name               = "${var.name_prefix}-valkey"
  project            = var.project_id
  region             = var.region
  tier               = var.tier
  memory_size_gb     = var.memory_size_gb
  redis_version      = var.redis_version
  authorized_network = var.authorized_network_self_link == "" ? null : var.authorized_network_self_link
}
