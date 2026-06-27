variable "manage_cloud_dns" {
  description = "true なら Cloud DNS の既存 managed zone に A レコードを作成する。false なら何も作らない（手動 DNS）。"
  type        = bool
  default     = false
}

variable "dns_zone_name" {
  description = "A レコードを作成する Cloud DNS managed zone 名（manage_cloud_dns=true のとき必須）。"
  type        = string
  default     = ""
}

variable "records" {
  description = "作成する DNS A レコード（FQDN -> IP）。manage_cloud_dns=true のときのみ使用する。"
  type = map(object({
    name = string
  }))
  default = {}
}

variable "ip_address" {
  description = "A レコードが指す IP アドレス。"
  type        = string
}

variable "ttl" {
  description = "A レコードの TTL（秒）。"
  type        = number
  default     = 300
}
