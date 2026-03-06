#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_ENV_FILE="${SCRIPT_DIR}/home-relay-edge.env"

usage() {
  cat <<'EOF'
Usage:
  sudo ./scripts/vps/setup-home-relay-edge.sh [env-file]

Behavior:
  - installs wireguard-tools, nftables, and caddy
  - configures wg0 on the VPS
  - configures Caddy for relay.kukuri.app and iroh-relay.kukuri.app
  - forwards UDP 11223 from the VPS to the home relay over WireGuard

Prepare first:
  cp scripts/vps/home-relay-edge.env.example scripts/vps/home-relay-edge.env
  edit scripts/vps/home-relay-edge.env
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

ENV_FILE="${1:-${DEFAULT_ENV_FILE}}"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "env file not found: ${ENV_FILE}" >&2
  exit 1
fi

if [[ "${EUID}" -ne 0 ]]; then
  echo "run as root: sudo $0 ${ENV_FILE}" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +a

if [[ -z "${PUBLIC_IFACE:-}" ]]; then
  PUBLIC_IFACE="$(ip route show default 0.0.0.0/0 | awk '/default/ { print $5; exit }')"
fi

required_vars=(
  PUBLIC_IFACE
  SSH_PORT
  WG_IFACE
  WG_PORT
  WG_ENDPOINT_HOST
  WG_VPS_ADDRESS
  WG_HOME_CLIENT_ADDRESS
  WG_HOME_ALLOWED_IPS
  HOME_WG_IP
  WG_SERVER_PRIVATE_KEY
  WG_HOME_PUBLIC_KEY
  RELAY_DOMAIN
  IROH_RELAY_DOMAIN
  HOME_RELAY_HTTP_PORT
  HOME_IROH_RELAY_HTTP_PORT
  HOME_RELAY_UDP_PORT
)

for var_name in "${required_vars[@]}"; do
  if [[ -z "${!var_name:-}" ]]; then
    echo "missing required variable: ${var_name}" >&2
    exit 1
  fi
done

backup_if_exists() {
  local path="$1"
  if [[ -f "${path}" ]]; then
    cp "${path}" "${path}.bak.$(date +%Y%m%d%H%M%S)"
  fi
}

install_packages() {
  export DEBIAN_FRONTEND=noninteractive
  apt-get update
  apt-get install -y --no-install-recommends \
    ca-certificates \
    caddy \
    nftables \
    wireguard-tools
}

write_sysctl() {
  cat > /etc/sysctl.d/99-kukuri-home-relay-edge.conf <<'EOF'
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1
EOF
  sysctl --system >/dev/null
}

write_wireguard() {
  local wg_conf="/etc/wireguard/${WG_IFACE}.conf"
  backup_if_exists "${wg_conf}"

  cat > "${wg_conf}" <<EOF
[Interface]
Address = ${WG_VPS_ADDRESS}
ListenPort = ${WG_PORT}
PrivateKey = ${WG_SERVER_PRIVATE_KEY}

[Peer]
PublicKey = ${WG_HOME_PUBLIC_KEY}
AllowedIPs = ${WG_HOME_ALLOWED_IPS}
EOF

  if [[ -n "${WG_HOME_PRESHARED_KEY:-}" ]]; then
    cat >> "${wg_conf}" <<EOF
PresharedKey = ${WG_HOME_PRESHARED_KEY}
EOF
  fi
}

ensure_caddy_import() {
  mkdir -p /etc/caddy/sites-enabled

  if [[ ! -f /etc/caddy/Caddyfile ]]; then
    if [[ -n "${CADDY_EMAIL:-}" ]]; then
      cat > /etc/caddy/Caddyfile <<EOF
{
	email ${CADDY_EMAIL}
}

import /etc/caddy/sites-enabled/*
EOF
    else
      cat > /etc/caddy/Caddyfile <<'EOF'
import /etc/caddy/sites-enabled/*
EOF
    fi
    return
  fi

  if ! grep -Fq 'import /etc/caddy/sites-enabled/*' /etc/caddy/Caddyfile; then
    printf '\nimport /etc/caddy/sites-enabled/*\n' >> /etc/caddy/Caddyfile
  fi
}

write_caddy_site() {
  local caddy_site="/etc/caddy/sites-enabled/kukuri-home-relay-edge.caddy"
  backup_if_exists "${caddy_site}"

  cat > "${caddy_site}" <<EOF
${RELAY_DOMAIN} {
	encode zstd gzip
	reverse_proxy http://${HOME_WG_IP}:${HOME_RELAY_HTTP_PORT} {
		flush_interval -1
		transport http {
			versions h1
			dial_timeout 10s
			response_header_timeout 0
			read_timeout 0
			write_timeout 0
		}
	}
}

${IROH_RELAY_DOMAIN} {
	encode zstd gzip
	reverse_proxy http://${HOME_WG_IP}:${HOME_IROH_RELAY_HTTP_PORT} {
		flush_interval -1
		transport http {
			versions h1
			dial_timeout 10s
			response_header_timeout 0
			read_timeout 0
			write_timeout 0
		}
	}
}
EOF

  caddy validate --config /etc/caddy/Caddyfile
}

write_nftables() {
  local nft_conf="/etc/nftables.conf"
  backup_if_exists "${nft_conf}"

  cat > "${nft_conf}" <<EOF
#!/usr/sbin/nft -f
flush ruleset

table inet filter {
  chain input {
    type filter hook input priority 0; policy drop;
    iifname "lo" accept
    ct state established,related accept
    tcp dport ${SSH_PORT} accept
    udp dport ${WG_PORT} accept
    tcp dport { 80, 443 } accept
    udp dport ${HOME_RELAY_UDP_PORT} accept
    iifname "${WG_IFACE}" ip saddr ${HOME_WG_IP} accept
    counter reject with icmpx type admin-prohibited
  }

  chain forward {
    type filter hook forward priority 0; policy drop;
    ct state established,related accept
    iifname "${PUBLIC_IFACE}" oifname "${WG_IFACE}" udp dport ${HOME_RELAY_UDP_PORT} accept
    iifname "${WG_IFACE}" oifname "${PUBLIC_IFACE}" udp sport ${HOME_RELAY_UDP_PORT} accept
  }

  chain output {
    type filter hook output priority 0; policy accept;
  }
}

table ip nat {
  chain prerouting {
    type nat hook prerouting priority dstnat; policy accept;
    iifname "${PUBLIC_IFACE}" udp dport ${HOME_RELAY_UDP_PORT} dnat to ${HOME_WG_IP}:${HOME_RELAY_UDP_PORT}
  }

  chain postrouting {
    type nat hook postrouting priority srcnat; policy accept;
    oifname "${WG_IFACE}" ip daddr ${HOME_WG_IP} udp dport ${HOME_RELAY_UDP_PORT} masquerade
  }
}
EOF
}

write_home_client_template() {
  local server_public_key
  local psk_line=""
  local vps_tunnel_ip

  server_public_key="$(printf '%s' "${WG_SERVER_PRIVATE_KEY}" | wg pubkey)"
  vps_tunnel_ip="${WG_VPS_ADDRESS%%/*}"
  if [[ -n "${WG_HOME_PRESHARED_KEY:-}" ]]; then
    psk_line="PresharedKey = ${WG_HOME_PRESHARED_KEY}"
  fi

  cat > "/root/${WG_IFACE}-home-client.conf" <<EOF
[Interface]
Address = ${WG_HOME_CLIENT_ADDRESS}
PrivateKey = <home-private-key>

[Peer]
PublicKey = ${server_public_key}
${psk_line}
Endpoint = ${WG_ENDPOINT_HOST}:${WG_PORT}
AllowedIPs = ${vps_tunnel_ip}/32
PersistentKeepalive = 25
EOF
}

restart_services() {
  systemctl daemon-reload
  systemctl enable --now "wg-quick@${WG_IFACE}"
  systemctl restart "wg-quick@${WG_IFACE}"
  systemctl enable --now nftables
  systemctl restart nftables
  systemctl enable --now caddy
  systemctl restart caddy
}

install_packages
write_sysctl
write_wireguard
ensure_caddy_import
write_caddy_site
write_nftables
write_home_client_template
restart_services

cat <<EOF
VPS edge setup completed.

Generated files:
  /etc/wireguard/${WG_IFACE}.conf
  /etc/caddy/sites-enabled/kukuri-home-relay-edge.caddy
  /etc/nftables.conf
  /root/${WG_IFACE}-home-client.conf

Next steps:
  1. Copy /root/${WG_IFACE}-home-client.conf to the home server and complete the private key.
  2. Set kukuri-community-node/.env from .env.home-vps-edge.example on the home server.
  3. Start the home server with: docker compose --profile bootstrap up -d --build
  4. Confirm:
     - curl https://${RELAY_DOMAIN}/v1/p2p/info
     - systemctl status wg-quick@${WG_IFACE}
     - nft list ruleset
EOF
