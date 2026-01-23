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

## Service endpoints (default)

- user-api: `http://localhost:8080/healthz`
- admin-api: `http://localhost:8081/healthz` (profile `admin`)
- relay: `http://localhost:8082/healthz`
- bootstrap: `http://localhost:8083/healthz` (profile `bootstrap`)
- user-api OpenAPI: `http://localhost:8080/v1/openapi.json`
- admin-api OpenAPI: `http://localhost:8081/v1/openapi.json`
