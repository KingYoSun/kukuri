# PR #112 TrustProviderList validation 強化レポート

作成日: 2026年02月21日

## 概要

- `kukuri-community-node/crates/cn-kip-types/src/lib.rs` の `KIND_TRUST_PROVIDER_LIST (10040)` バリデーションを強化。
- `:rank` で終わるタグに対して以下を必須化。
  - kind prefix が `30382` / `30383` / `30384` / `30385` のいずれかであること
  - provider pubkey が 64桁hex公開鍵であること
- 不正タグを含む場合はイベントを reject するよう変更。

## テスト

- 追加:
  - `validate_trust_provider_list_rejects_invalid_tag_kind_prefix`
  - `validate_trust_provider_list_rejects_invalid_pubkey`
- 維持:
  - `validate_trust_provider_list_accepts_valid_event`

## 検証コマンド

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-kip-types"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
