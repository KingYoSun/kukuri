# `kukuri-cli` 削除移行タスク完了レポート

作成日: 2026年02月10日

## 概要

`kukuri-cli` の機能を `kukuri-community-node` の `cn-cli` へ統合する移行タスクを完了した。  
`kukuri-cli/` ディレクトリは削除済みで、非 `docs` 配下の参照は 0 件を確認した。

## 実施内容

1. 参照・導線の移行状態を確認
   - `cn-cli` ベースの P2P/bootstrap 導線へ統合済みであることを確認
   - `rg -n "kukuri-cli" --glob '!docs/**' -S` でヒットなしを確認
2. `community-node-tests` 非グリーン要因を解消
   - 原因: `Run community node clippy` の `-D warnings` で複数クレートが連鎖失敗
   - 対応クレート:
     - `kukuri-community-node/crates/cn-moderation`
     - `kukuri-community-node/crates/cn-admin-api`
     - `kukuri-community-node/crates/cn-index`
     - `kukuri-community-node/crates/cn-user-api`
     - `kukuri-community-node/crates/cn-trust`
     - `kukuri-community-node/crates/cn-bootstrap`
     - `kukuri-community-node/crates/cn-relay`
     - `kukuri-community-node/crates/cn-cli`
   - 修正方針:
     - 挙動変更を避け、clippy 警告解消を最小差分で実施
     - 文字列補間、`if let` 折りたたみ、`Range::contains` 化、不要 `mut`/借用除去
     - 必要最小限の lint suppress（`too_many_arguments` など）を明示的に付与
3. `gh act` 必須3ジョブを最新差分で再実行
   - `format-check` / `native-test-linux` / `community-node-tests` を完走

## 検証結果

- `gh act --workflows .github/workflows/test.yml --job community-node-tests`  
  成功。ログ: `tmp/logs/gh-act-community-node-tests-kukuri-cli-removal-20260210-193617.log`
- `gh act --workflows .github/workflows/test.yml --job format-check`  
  成功。ログ: `tmp/logs/gh-act-format-check-kukuri-cli-removal-20260210-194030.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`  
  成功。ログ: `tmp/logs/gh-act-native-test-linux-kukuri-cli-removal-20260210-194145.log`

## 補足

- `gh act` 実行時に `git clone: some refs were not updated` や Docker host 情報が stderr に出力されるが、各ジョブの最終結果は `Job succeeded` を確認。
