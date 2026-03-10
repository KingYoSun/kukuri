# 2026-02-21 Issue #111 PR-2 trust provider key アルゴリズム分離

作成日: 2026年02月21日

## 概要

- 目的: trust provider key を trust algorithm（`report-based` / `communication-density`）ごとに分離し、発行・検証・設定の各経路で key 混線を防止する。
- 対象: `kukuri-tauri` の trust provider 設定保存、trust API DTO/command/handler、UI、テスト。

## 実装内容

- DTO / command / API client:
  - `CommunityNodeTrustAlgorithm` と selector を追加。
  - `getTrustProvider` / `clearTrustProvider` を algorithm 指定可能に拡張（未指定は後方互換モード）。
- trust provider 永続化:
  - 新キー `community_node_trust_providers_v2` で algorithm 別（`report_based` / `communication_density`）保存へ変更。
  - `set_trust_provider` は algorithm 指定時は片側更新、未指定時は両側更新（旧クライアント互換）。
  - `get_trust_provider` は algorithm 指定時に該当側を返却、未指定時は report-based 優先で返却。
  - `clear_trust_provider` は algorithm 指定時は片側削除、未指定時は全削除。
- trust 検証/発行:
  - assertion 生成で `claim` tag を付与。
  - verification で expected claim と expected provider pubkey を algorithm ごとに強制。
  - report-based で communication-density provider key が混入した assertion を reject。
  - trust query の node 選択で algorithm 別 provider `relay_url` を優先利用。
- UI:
  - `CommunityNodePanel` の trust anchor 設定を algorithm 別 2 セレクトに分離。
  - `PostCard` で report/density を別 provider 設定で問い合わせるよう更新。

## 互換/移行ノート

- legacy key 互換:
  - `community_node_trust_providers_v2` が未設定の場合、`community_node_trust_provider_v1`（旧単一 provider）を読み取り、両 algorithm へ複製して v2 に自動移行。
  - さらに旧 trust anchor 形式（単一値）の fallback 読み取りも維持。
  - 移行完了後は v2 保存に統一し、旧キーは削除。
- API 互換:
  - `algorithm` は optional のため既存呼び出しを破壊しない。
  - 未指定 `set` は両 algorithm 同期更新、未指定 `get` は report-based 優先返却、未指定 `clear` は全削除で旧挙動を維持。

## 追加・更新テスト

- Rust (`community_node_handler`):
  - `trust_provider_supports_algorithm_separation`
  - `trust_provider_migrates_legacy_single_provider_to_both_algorithms`
  - `trust_report_based_rejects_assertion_signed_by_other_algorithm_provider_key`
  - `trust_queries_use_algorithm_specific_provider_configuration`
- TypeScript:
  - `PostCard.test.tsx` を algorithm 別 provider モック前提へ更新し、report/density がそれぞれ対応 base_url へ問い合わせることを検証。

## 検証結果

- `cargo fmt`: pass
- `bash ./scripts/test-docker.sh rust`: pass
- `bash ./scripts/test-docker.sh ts`: pass
- `docker compose --project-name kukuri_tests -f docker-compose.test.yml run --rm --build lint-check`: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`: pass
