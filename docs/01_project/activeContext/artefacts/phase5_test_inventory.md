# Phase 5 テスト分類インベントリ
最終更新日: 2025年10月23日

## Rust（kukuri-tauri/src-tauri）

| グループ | 現行パス | 現行種別 | 移動先候補 | 不足/課題 |
| --- | --- | --- | --- | --- |
| EventService tests | `tests/unit/application/event_service` | ユニット | （現行維持） | EventManager 連携ケースは `tests/common/mocks` のスタブへ集約済み。Publish/Distribution モックも共有化したため追加作業なし。 |
| Shared test utilities | `src/application/shared/tests` | 共通モック | `tests/common`（既存）へ統合 | 既に汎用化済。Phase 5 後は Rust 側共通モックの唯一の配置として維持する。 |
| EventManager tests | `src/modules/event/manager/tests` | 結合 | `tests/integration/event/manager` | Tauri `AppHandle` 依存がないシナリオを追加予定（不足）。 |
| P2P module tests | `src-tauri/tests/p2p_gossip_smoke.rs`, `src-tauri/tests/p2p_mainline_smoke.rs` | 結合 | 同上 | Mainline DHT シナリオは接続統計で検証。Docker / CI 実行時は `ENABLE_P2P_INTEGRATION=1` を設定する。 |
| Contract tests | `tests/contract/nip10.rs` | 契約 | （現行維持） | Phase 5 でカテゴリディレクトリを明確化済み。 |
| Performance tests | `tests/performance_tests.rs` | パフォーマンス | `tests/performance/*.rs` | 実行条件と計測方法のドキュメント不足、要整備。 |
| Integration harness | `tests/integration/*` | 結合 | `tests/integration`（維持） | `test_p2p_mainline.rs` で Mainline DHT 設定の統合シナリオを追加。Offline 系統合ケースは引き続き未作成。 |
| Unit harness | `tests/unit` | （空） | `tests/unit` | Phase 5 で `application/shared` や `state` のユニットテストを移植。 |

### Rustカバレッジ（Workstream B）

- コマンド: `./scripts/test-docker.sh coverage`（PowerShell 版も同名コマンド）。内部で `docker compose run rust-coverage` を呼び出し、`cargo tarpaulin --locked --all-features --skip-clean --out Json --out Lcov --output-dir /app/test-results/tarpaulin --timeout 1800` を実行する。
- 成果物: `test-results/tarpaulin/tarpaulin-report.json` / `test-results/tarpaulin/lcov.info` を収集し、スクリプト完了時に `docs/01_project/activeContext/artefacts/metrics/YYYY-mm-dd-HHMMSS-tarpaulin.{json,lcov}` へコピーする。
- 初回計測（2025年10月26日）: 総カバレッジ 25.23%（1630/6460 行）。当面の目標は Phase 5 完了時に 50%、Phase 6 で 70% を達成する。テスト不足領域は JSON レポートの `files` 配列を参照する。
- 実行ポリシー: `cargo test` が通過した commit でのみ tarpaulin を実行。`ENABLE_P2P_INTEGRATION=0` のままユニット/結合テストを網羅し、P2P 系統合テストは別ジョブで担保する。

## TypeScript（kukuri-tauri/src）

| グループ | 現行パス | 現行種別 | 移動先候補 | 状態/課題 |
| --- | --- | --- | --- | --- |
| Component tests | `src/tests/unit/components/**/*.test.tsx` | コンポーネントユニット | （現行維持） | 旧 `__tests__` と直下ファイルを統合済み。Sidebar/PostCard など重複ケースを整理。 |
| Hook tests | `src/tests/unit/hooks/**/*.test.tsx` | カスタムフックユニット | （現行維持） | 主要フック（`useAuth` / `useP2P` / `useTopics` 等）を集約。欠損シナリオ無し（2025年10月23日確認）。 |
| Store tests | `src/tests/unit/stores/**/*.test.ts` | Zustand ストアユニット | （現行維持） | `topicStore`／`authStore` テストを統合し、永続化ヘルパー対応を検証。マルチアカウント系は `authStore.accounts.test.ts` に分離。 |
| Library tests | `src/tests/unit/lib/**/*.test.ts` | ユーティリティ／API | （現行維持） | `syncEngine` の競合解決シナリオは追加要検討（未着手）。 |
| Integration UI tests | `src/tests/integration/ui/*.integration.test.tsx` | UI＋ストア統合 | （現行維持） | 既存シナリオ（認証／リレー／トピック／マルチアカウント）を新ディレクトリへ移行済み。 |
| Integration DI tests | `src/tests/integration/di/*.integration.test.ts` | サービス依存統合 | （新設） | `store_di.integration.test.ts` で `useP2PStore` と `offlineSyncService` の依存注入を検証。 |
| Legacy root tests | （廃止） | - | - | `src/__tests__` ディレクトリを撤去。新構成へ完全移行済み。 |

## 移動／追加タスク（ドラフト）

- [x] `tests/unit` へ EventService の純粋ユニットテストを移動し、EventManager 依存をモック化する。
- [x] `tests/integration/p2p` に Mainline DHT ルートのシナリオを追加し、Docker 実行手順を更新する。
- [x] TypeScript の `__tests__` 直下の UI テストを `src/tests/unit` 配下へ移動し、ダブりを解消する。
- [x] Integration テストで `App` の複数アカウントシナリオ（`src/tests/integration/ui/multipleAccounts.test.tsx`）を新ディレクトリへ移管し、依存モックを再利用可能にする。
- [x] DI 依存の統合テストを `src/tests/integration/di/store_di.integration.test.ts` に追加し、P2P／Offline 系の初期化パスを検証する。
