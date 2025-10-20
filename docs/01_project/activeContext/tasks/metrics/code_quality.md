# コード品質指標

**最終更新**: 2025年10月21日（担当: Codex）

## 更新概要
- 計測日時: 2025年10月20日〜2025年10月21日。
- 計測頻度: 火曜・金曜の午前に定期取得。大規模リファクタ前後で随時更新。
- 計測手順: `docs/01_project/activeContext/tasks/metrics/update_flow.md` に定義したコマンド群（`rg` ベース）を使用。
- レビュー体制: 週次スタンドアップで本ファイルを確認し、閾値超過は `tasks/context/blockers.md` へ記録。

## 指標一覧

### TypeScript
| 指標 | 値 | 取得コマンド | 補足 |
| --- | --- | --- | --- |
| TODOコメント | 1件 | `rg "TODO" -g "*.ts"` | `appshell/src/services/topicService.ts` の保留箇所。 |
| anyキーワード | 94箇所 | `rg "any" -g "*.ts"` | Phase 5 での型付け対応を優先度中で管理。 |
| ESLintエラー | 0件 | `pnpm lint` | 2025年10月21日に再実行し成功。未使用変数の修正を反映済み。 |
| 未使用APIエンドポイント | 計測準備中 | - | 2025年08月16日時点では 11件。`collect-metrics` スクリプト整備後に再計測。 |

### Rust
| 指標 | 値 | 取得コマンド | 補足 |
| --- | --- | --- | --- |
| TODOコメント | 21件 | `rg "TODO" -g "*.rs"` | `src-tauri/modules/p2p` 周辺に集中。 |
| `#[allow(dead_code)]` | 20箇所 | `rg "#\\[allow(dead_code)\\]" -g "*.rs"` | Phase 5 のテスト移行で削減予定。 |
| Clippy警告 | 0件 | `cargo clippy -- -D warnings` | 現状警告なし。 |
| 未使用インポート | 計測準備中 | - | `rustfmt --check` 等で検出予定。 |

## 改善タスクとフォローアップ
- `scripts/metrics/collect-metrics.{ps1,sh}` を追加済み。未計測指標（未使用API、未使用インポート）の算出ロジックを拡張し、定期実行に組み込む。
- TypeScript の未使用変数を解消し、Lint 成功状態で本ファイルを更新する。
- `#[allow(dead_code)]` と TODO コメントの削減状況を Phase 5 テスト移行タスクで追跡する。
