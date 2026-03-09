# kukuri-community-node

Minimal community node services for kukuri.

## Quick start

1. Copy `.env.example` to `.env` and edit values.
2. Start core services:
   `docker compose up -d`
3. Run migrations and seed:
   `docker compose run --rm cn-cli migrate`
   `docker compose run --rm cn-cli config seed`
   `docker compose run --rm cn-cli admin bootstrap --username admin --password change-me`
4. If you want to use the bundled custom iroh relay, set
   `RELAY_IROH_RELAY_MODE=custom` in `.env` and start bootstrap profile:
   `docker compose --profile bootstrap up -d`
5. If you expose relay services through a VPS + WireGuard edge, see
   `docs/03_implementation/community_nodes/home_vps_wireguard_edge.md` and
   copy `kukuri-community-node/.env.home-vps-edge.example` to `.env` as a base.
   The Home-side `cn-iroh-relay` also needs `7842/udp` and a public certificate/key
   under `kukuri-community-node/docker/iroh-relay/certs/`.

## Service endpoints (default)

- user-api: `http://localhost:8080/healthz`
- admin-api: `http://localhost:8081/healthz` (profile `admin`)
- relay: `http://localhost:8082/healthz`
- cn-iroh-relay (custom relay): `http://localhost:3340` (profile `bootstrap`)
- bootstrap: `http://localhost:8083/healthz` (profile `bootstrap`)
- user-api OpenAPI: `http://localhost:8080/v1/openapi.json`
- admin-api OpenAPI: `http://localhost:8081/v1/openapi.json`

## Deployment variants

- Local/default: copy `.env.example` to `.env`
- Home relay behind VPS edge: copy `.env.home-vps-edge.example` to `.env`, then
  deploy the VPS edge with `scripts/vps/setup-home-relay-edge.sh`
