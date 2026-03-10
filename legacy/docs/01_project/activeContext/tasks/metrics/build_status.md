# ビルド状況

**最終更新**: 2025年10月21日（担当: Codex）

## 更新概要
- 計測日時: 2025年10月20日〜2025年10月21日（Windows + PowerShell、Docker 必要時）
- 更新頻度: 火曜・金曜の午前に定期実施。リリース直前は追加で取得。
- レビュー体制: 週次スタンドアップで本ファイルと `update_flow.md` を確認し、乖離があれば `tasks/context/blockers.md` に記録。
- 詳細手順: `docs/01_project/activeContext/tasks/metrics/update_flow.md` を参照。

## 実行結果一覧
| 区分 | コマンド | ステータス | 詳細 / 備考 |
| --- | --- | --- | --- |
| TypeScript | `pnpm test` | 成功 | 694件中 688件成功 / 6件スキップ。JSONレポートを `docs/01_project/activeContext/artefacts/metrics/2025-10-20-vitest-results.json` に保存。 |
| TypeScript | `pnpm exec vitest run --reporter=json` | 成功 | `pnpm test` の結果と同一。`test_results.md` の統計に反映済み。 |
| TypeScript | `pnpm lint` | 成功 | 2025年10月21日再実行で成功。`src/stores/draftStore.test.ts` の未使用変数を解消済み。 |
| Rust | `cargo test` | 失敗 | Windows DLL 依存で `STATUS_ENTRYPOINT_NOT_FOUND`。Docker 実行（`scripts/run-rust-tests.ps1`）へ移行中。 |
| Rust | `cargo clippy -- -D warnings` | 成功 | 警告なし。 |
| Rust | `scripts/run-rust-tests.ps1` | 未実施 | Phase 5 CI 対応に合わせて既定手順へ編入予定。 |
| Tauri | `pnpm tauri build` | 未実施 | Phase 7 作業中は週次リリース前のスポット実行に限定。 |

## 次回アクション
- Rust テストは Docker 経由を標準化し、次回計測から `scripts/run-rust-tests.ps1` の結果を記録する。
- `scripts/metrics/collect-metrics.{ps1,sh}` をメトリクス取得フローへ組み込み、Lint/Test 結果と合わせて運用する。
