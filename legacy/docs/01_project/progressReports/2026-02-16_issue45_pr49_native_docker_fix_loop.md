# Issue #45 / PR #49 Native+Docker fix loop

最終更新日: 2026年02月16日

## 概要

PR #49 の CI Run `22070890976` で失敗した `Native Test (Linux)` / `Docker Test Suite` を、Issue #45 PR-1（i18n キー整合）の範囲内で最小修正して解消した。

- 対象Run: `https://github.com/KingYoSun/kukuri/actions/runs/22070890976`
- 対象Job:
  - `Native Test (Linux)`（Job ID `63774951758`）
  - `Docker Test Suite`（Job ID `63774971196`）

## 失敗原因（確定）

1. `OfflineIndicator.test.tsx` が `date-fns/locale` を部分mockしており、`i18n.ts` が参照する `enUS` export が欠落。
2. これにより `i18n.ts` import 時点で例外が発生し、TypeScript テストの多数スイートが連鎖失敗。
3. 併せて、いくつかのテストが固定日本語文言に依存しており、i18n キー／ロケール変更に追従できていなかった。

## 実施方針（最小修正）

- プロダクト機能の追加・設計変更は行わない。
- テスト基盤と失敗テスト期待値の整合に限定。
- PR-1 の本来目的（i18n整備）から逸脱しない差分のみ採用。

## 実施内容

1. `src/tests/setup.ts` でテスト時ロケールを `ja` に固定（`localStorage` + `i18n.changeLanguage`）。
2. `src/tests/unit/components/OfflineIndicator.test.tsx` から `date-fns/locale` mock を除去し、実ロケールオブジェクトを許容するアサーションへ変更。
3. 失敗していた UI/Store テストを i18n キー基準へ更新（`i18n.t(...)` 参照）し、固定文字列依存を解消。
4. `src/stores/authStore.ts` の未使用 catch 変数を除去し lint 警告を解消。
5. `./scripts/test-docker.sh all` を再実行して Docker スイート通過を確認。

## 検証結果

- `pnpm vitest run`（失敗対象12ファイル）: pass
- `pnpm test`: pass（95 files / 842 tests）
- `pnpm type-check`: pass
- `pnpm lint`: pass
- `cargo test --locked --workspace --all-features`（`kukuri-tauri/src-tauri`）: pass
- `cargo clippy --locked --workspace --all-features -- -D warnings -A dead_code -A unused_variables`（ローカル）: fail
  - 理由: ローカル Rust `1.93.1` では `collapsible_if` が大量に `-D warnings` へ昇格。
  - 備考: CI workflow は Rust `1.86` 固定のため、ローカルとの差分として扱う。
- `HOME=/tmp/kukuri-home DOCKER_CONFIG=/tmp/docker-config ./scripts/test-docker.sh all`: pass
  - Rust tests / Rust clippy / TypeScript tests / type-check / ESLint まで全通過。
- `gh act --workflows .github/workflows/test.yml --job format-check`: pass
  - ログ: `tmp/logs/gh-act-format-check-issue45-pr49-native-docker-fix-loop.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: pass
  - ログ: `tmp/logs/gh-act-native-test-linux-issue45-pr49-native-docker-fix-loop.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: pass
  - ログ: `tmp/logs/gh-act-community-node-tests-issue45-pr49-native-docker-fix-loop.log`

## 影響範囲

- 主体はテストコードとテストセットアップの整合修正。
- 例外として `authStore.ts` の catch 変数削除（挙動変更なし）。
- i18n PR-1 のスコープを維持し、機能拡張は行っていない。
