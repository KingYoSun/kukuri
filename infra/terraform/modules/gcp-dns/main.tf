terraform {
  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

resource "google_dns_record_set" "a" {
  for_each = var.manage_cloud_dns ? var.records : {}

  managed_zone = var.dns_zone_name
  name         = each.value.name
  type         = "A"
  ttl          = var.ttl
  rrdatas      = [var.ip_address]
}
