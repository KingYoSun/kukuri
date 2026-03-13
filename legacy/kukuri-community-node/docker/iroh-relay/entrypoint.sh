#!/bin/sh
set -eu

IROH_RELAY_BIN="${IROH_RELAY_BIN:-/usr/local/bin/iroh-relay}"

is_true() {
  case "${1:-}" in
    1|true|TRUE|yes|YES|on|ON)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

generate_config() {
  config_path="$1"
  http_bind_addr="${IROH_RELAY_HTTP_BIND_ADDR:-0.0.0.0:3340}"
  https_bind_addr="${IROH_RELAY_HTTPS_BIND_ADDR:-0.0.0.0:443}"
  quic_bind_addr="${IROH_RELAY_QUIC_BIND_ADDR:-0.0.0.0:7842}"
  tls_cert_mode="${IROH_RELAY_TLS_CERT_MODE:-Manual}"
  tls_cert_dir="${IROH_RELAY_TLS_CERT_DIR:-/certs}"
  tls_manual_cert_path="${IROH_RELAY_TLS_MANUAL_CERT_PATH:-${tls_cert_dir}/default.crt}"
  tls_manual_key_path="${IROH_RELAY_TLS_MANUAL_KEY_PATH:-${tls_cert_dir}/default.key}"
  tls_hostname="${IROH_RELAY_TLS_HOSTNAME:-}"
  tls_contact="${IROH_RELAY_TLS_CONTACT:-}"
  enable_quic="false"
  prod_tls="false"
  dangerous_http_only="false"

  if is_true "${IROH_RELAY_ENABLE_QUIC_ADDR_DISCOVERY:-0}"; then
    enable_quic="true"
  fi
  if is_true "${IROH_RELAY_TLS_PROD:-1}"; then
    prod_tls="true"
  fi
  if is_true "${IROH_RELAY_TLS_DANGEROUS_HTTP_ONLY:-1}"; then
    dangerous_http_only="true"
  fi

  case "${tls_cert_mode}" in
    Manual|Reloading)
      if [ ! -f "${tls_manual_cert_path}" ]; then
        echo "missing iroh relay certificate: ${tls_manual_cert_path}" >&2
        exit 1
      fi
      if [ ! -f "${tls_manual_key_path}" ]; then
        echo "missing iroh relay private key: ${tls_manual_key_path}" >&2
        exit 1
      fi
      ;;
    LetsEncrypt)
      if [ -z "${tls_hostname}" ]; then
        echo "IROH_RELAY_TLS_HOSTNAME is required when IROH_RELAY_TLS_CERT_MODE=LetsEncrypt" >&2
        exit 1
      fi
      if [ -z "${tls_contact}" ]; then
        echo "IROH_RELAY_TLS_CONTACT is required when IROH_RELAY_TLS_CERT_MODE=LetsEncrypt" >&2
        exit 1
      fi
      ;;
    *)
      echo "unsupported IROH_RELAY_TLS_CERT_MODE: ${tls_cert_mode}" >&2
      exit 1
      ;;
  esac

  cat > "${config_path}" <<EOF
enable_relay = true
http_bind_addr = "${http_bind_addr}"
enable_quic_addr_discovery = ${enable_quic}

[tls]
https_bind_addr = "${https_bind_addr}"
quic_bind_addr = "${quic_bind_addr}"
cert_mode = "${tls_cert_mode}"
cert_dir = "${tls_cert_dir}"
prod_tls = ${prod_tls}
dangerous_http_only = ${dangerous_http_only}
EOF

  case "${tls_cert_mode}" in
    Manual|Reloading)
      cat >> "${config_path}" <<EOF
manual_cert_path = "${tls_manual_cert_path}"
manual_key_path = "${tls_manual_key_path}"
EOF
      ;;
  esac

  if [ -n "${tls_hostname}" ]; then
    printf 'hostname = "%s"\n' "${tls_hostname}" >> "${config_path}"
  fi
  if [ -n "${tls_contact}" ]; then
    printf 'contact = "%s"\n' "${tls_contact}" >> "${config_path}"
  fi
}

if [ -n "${IROH_RELAY_CONFIG_PATH:-}" ]; then
  exec "${IROH_RELAY_BIN}" --config-path "${IROH_RELAY_CONFIG_PATH}"
fi

if is_true "${IROH_RELAY_ENABLE_QUIC_ADDR_DISCOVERY:-0}"; then
  generated_config_path="$(mktemp /tmp/iroh-relay.generated.XXXXXX.toml)"
  generate_config "${generated_config_path}"
  exec "${IROH_RELAY_BIN}" --config-path "${generated_config_path}"
fi

if is_true "${IROH_RELAY_DEV:-1}"; then
  exec "${IROH_RELAY_BIN}" --dev
fi

exec "${IROH_RELAY_BIN}" --dev
