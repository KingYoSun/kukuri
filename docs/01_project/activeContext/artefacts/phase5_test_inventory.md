# Phase 5 テスト分類インベントリ
最終更新日: 2025年10月20日

## Rust（kukuri-tauri/src-tauri）

| グループ | 現行パス | 現行種別 | 移動先候補 | 不足/課題 |
| --- | --- | --- | --- | --- |
| EventService tests | `src/application/services/event_service/tests` | ユニット＼結合混在 | `tests/unit/application/event_service` と `tests/integration/application/event_service` | EventManager 連携ケースは本体内依存のまま。Publish/Distribution のモックを `tests/common` へ切り出す。 |
| Shared test utilities | `src/application/shared/tests` | 共通モック | `tests/common`（既存）へ統合 | 既に汎用化済。Phase 5 後は Rust 側共通モックの唯一の配置として維持する。 |
| EventManager tests | `src/modules/event/manager/tests` | 結合 | `tests/integration/event/manager` | Tauri `AppHandle` 依存がないシナリオを追加予定（不足）。 |
| P2P module tests | `src/modules/p2p/tests` | 結合 | `tests/integration/p2p` | Mainline DHT シナリオを `tests/integration/p2p/mainline_*.rs` として再配置。 |
| Contract tests | `tests/nip10_contract_tests.rs` | 契約 | `tests/contract/nip10.rs` | Phase 5 でカテゴリディレクトリを明確化。 |
| Performance tests | `tests/performance_tests.rs` | パフォーマンス | `tests/performance/*.rs` | 実行条件と計測方法のドキュメント不足、要整備。 |
| Integration harness | `tests/integration/*` | 結合 | `tests/integration`（維持） | `test_auth.rs` はモックベースで E2E 未到達。Offline/P2P の統合ケースが未作成（不足）。 |
| Unit harness | `tests/unit` | （空） | `tests/unit` | Phase 5 で `application/shared` や `state` のユニットテストを移植。 |

## TypeScript（kukuri-tauri/src）

| グループ | 現行パス | 現行種別 | 移動先候補 | 不足/課題 |
| --- | --- | --- | --- | --- |
| Component tests | `src/components/**/*.test.tsx` | コンポーネントユニット | `src/tests/unit/components`（別名維持可） | レイアウト系が二重に存在（直下と `__tests__`）。Phase 5 で `__tests__` を整理。 |
| Hook tests | `src/hooks/**/*test.tsx` | カスタムフックユニット | `src/tests/unit/hooks` | `useOfflineQueue` など未カバーのフックを追加（不足）。 |
| Store tests | `src/stores/**/*test.ts` | Zustand ストアユニット | `src/tests/unit/stores` | `p2pStore` 系の永続化検証を `persistHelpers` に合わせ更新。 |
| Library tests | `src/lib/**/*test.ts` | ユーティリティ／API | `src/tests/unit/lib` | `syncEngine` の競合解決シナリオ未カバー。 |
| Integration tests | `src/test/integration/*.integration.test.tsx` | 統合（React + Service） | `src/tests/integration/ui` | 新しい DI コンテナ／P2P デバッグパネルの統合ケースが未実装。 |
| Legacy root tests | `src/__tests__/**` | 混在（古い構成） | `src/tests/legacy` または段階的削除 | Phase 5 で最新構成へ移行し、古いエントリポイントを削除する。 |

## 移動／追加タスク（ドラフト）

- [ ] `tests/unit` へ EventService の純粋ユニットテストを移動し、EventManager 依存をモック化する。
- [ ] `tests/integration/p2p` に Mainline DHT ルートのシナリオを追加し、Docker 実行手順を更新する。
- [ ] TypeScript の `__tests__` 直下の UI テストを `src/tests/unit` 配下へ移動し、ダブりを解消する。
- [ ] Integration テストで `App` の複数アカウントシナリオ（`src/__tests__/integration/multipleAccounts.test.tsx`）を新ディレクトリへ移管し、依存モックを再利用可能にする。
