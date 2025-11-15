[title] 作業中タスク（in_progress）

最終更新日: 2025年11月15日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### MVP Exit タスク

14. **グローバルコンポーザー & 投稿削除: キャッシュ整合とテスト更新**  
    - 背景: `docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md:19` と `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:123,128` で、`TopicSelector` の create-from-composer モードと `useDeletePost` の React Query 無効化テスト、`post-delete-cache` Docker シナリオの整備が未完と整理されている。  
    - やること: (1) `corepack enable pnpm` 前提で `pnpm vitest run src/tests/unit/components/topics/TopicSelector.test.tsx src/tests/unit/components/posts/PostCard.test.tsx src/tests/unit/routes/{trending,following}.test.tsx` を再実行し、グローバルコンポーザー導線と Summary Panel を検証。(2) `useDeletePost` / `postStore` に `invalidatePostCaches` を実装し、トレンド/フォロー/トピック/プロフィール各キャッシュと `offlineStore` の整合を確認、Docker `./scripts/test-docker.{sh,ps1} ts --scenario post-delete-cache` の artefact を更新。(3) 結果を `phase5_user_flow_inventory.md` 5.7/5.9/5.10 と Runbook に反映。  
    - 完了条件: グローバルコンポーザー経由のトピック作成と投稿削除がキャッシュ不整合なく動作し、Vitest/Docker/Nightly のログが揃っている。

15. **鍵管理ダイアログ: 鍵バックアップ/復旧フローの提供**  
    - 背景: `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:125` で、設定 > 鍵管理ボタンが未配線でバックアップ手段が無いことが MVP ブロッカーとして挙げられている。  
    - やること: (1) `KeyManagementDialog` を実装し、`export_private_key` / `SecureStorageApi.addAccount` / `add_relay` 連動と注意喚起 UI を整備。(2) エクスポート/インポート操作を `errorHandler` に記録、`withPersist` へ操作履歴を残す。(3) `pnpm vitest`（UI）と `./scripts/test-docker.ps1 rust -Test key_management`（仮）でバックアップ/復旧の契約テストを追加し、Runbook・`phase5_user_flow_inventory.md` 5.1/5.6 に掲載。  
    - 完了条件: ユーザーが UI から鍵を安全にバックアップ/復旧でき、テストとドキュメントで手順が保証される。

16. **Ops/CI: Nightly & GitHub Actions で MVP 導線を安定再現**  
    - 背景: `docs/01_project/roadmap.md:20` と `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` の「追加予定のテスト/artefact」節で、GitHub Actions の `trending-feed` Docker 失敗・Nightly artefact 権限・`scripts/test-docker.ps1 all` の安定化・`docs/01_project/progressReports/` への Runbook リンク不足が指摘されている。  
    - やること: (1) GitHub Actions `trending-feed` ジョブで発生している Docker 権限問題と artefact 不足を切り分け、`nightly.yml` の `*-logs` / `*-reports` 命名を固定。(2) `cmd.exe /c "corepack enable pnpm"` → `pnpm install --frozen-lockfile` を `docs/01_project/setup_guide.md` / Runbook に追記し、`scripts/test-docker.ps1 all` で同前提を明文化。(3) `docs/01_project/progressReports/` へ Nightly テスト ID（`nightly.profile-avatar-sync`, `nightly.trending-feed`, `nightly.user-search-pagination`, ほか）と対応するログ/artefact リンクを整理。  
    - 完了条件: GitHub Actions / Nightly がすべての MVP 導線を再現し、failure 時に参照すべき artefact ・ Runbook リンクが一元化されている。
    - メモ (2025年11月14日): `Format Check` 失敗は `kukuri-tauri/src-tauri/src/infrastructure/p2p/event_distributor/state.rs` と `tests/common/performance/{mod.rs,offline_seed.rs}` の整形漏れが原因だったため `cargo fmt` で修正済み。`gh act --workflows .github/workflows/test.yml --job format-check --container-options "--user root"` によりローカル再現・緑化を確認。`docker-test` / `native-test-linux` の artefact への影響は無く、GitHub Actions 本番では `Test` ワークフローの再実行で回復予定。
    - メモ (2025年11月15日): `gh run view 19377708787 --job format-check --log` で再発を確認。`kukuri-tauri/src-tauri/src/state.rs:516` の `sync_service.schedule_sync(DEFAULT_SYNC_INTERVAL_SECS).await` が rustfmt 規約（引数ごとの改行）に反しており CI が再度失敗しているため、該当ブロックの整形をやり直し `gh act --workflows .github/workflows/test.yml --job format-check` でローカル再検証予定。
    - メモ (2025年11月15日): `gh run view 19384995086` で `format-check`/`native-test-linux` が `src/routes/search.tsx` の未使用型インポートと Prettier 警告で落ちていたため修正。`scripts/test-docker.ps1 ts` で TypeScript テスト/型チェック/ESLint を Docker 経由で再実行し、`gh act -j format-check --container-options "--user 0"` でフォーマット専用ジョブがローカルでも完走することを確認。
    - メモ (2025年11月15日): `gh run view 19387135994` の `format-check` では `get_offline_retry_metrics` の改行崩れ、`build-test-windows` では `RecordOfflineRetryOutcomeRequest::validate` 未 import・`OfflineRetryMetrics::new` での `LastRetryMetadata::default()` 呼び出し・`MutexGuard::cloned()` 使用が原因で Rust ビルドが落ちていたため、`Validate` import 追加と `LazyLock` ベースの初期化、`LastRetryMetadata` へ `Clone` を付与して `cargo fmt` → `cargo check --workspace --all-features`（`kukuri-tauri/src-tauri` / `kukuri-cli`）までローカルで確認。`gh act --workflows .github/workflows/test.yml --job format-check --reuse` では Rust/CLI フォーマットは通過したが Prettier が Windows 側の CRLF 差分で `src/components/SyncStatusIndicator.tsx` 等 3 ファイルに警告を出すため失敗扱いになる点と、`pnpm format:check` / `pnpm type-check` が同理由（および既存の `useSyncManager` の TODO) でローカルのみ失敗する旨を記録。
    - メモ (2025年11月15日-2): `gh run view 19389007187 --job native-test-linux --log` で `kukuri-tauri/src-tauri/src/infrastructure/offline/metrics.rs` のユニットテスト内で `snapshot()` 関数と同名変数が衝突し `error[E0618]` となっていたため、`super::snapshot()` 呼び出しに変更。`gh act --workflows .github/workflows/test.yml --job native-test-linux --container-options "--user root"` による再現では `Run Rust tests` / `Run Rust clippy` まで完走し、残る failure は既知の `useSyncManager` Vitest locale issue であることを確認。
    - メモ (2025年11月15日-3): `gh run view 19389473094 --job native-test-linux --log` の Vitest 失敗（`useSyncManager` のトースト文言が文字化け＆ `SyncStatusIndicator.test.tsx` がポップオーバー前提へ未更新）を修正。`useSyncManager.ts` の `toast.success/error` および `setSyncError` 文字列を正規化し、`useSyncManager.test.tsx` へ `removePendingAction` / `updateLastSyncedAt` のモックを追加、`SyncStatusIndicator.test.tsx` では `fireEvent.click` でポップオーバーを開いてから再送メトリクスを検証。`gh act --workflows .github/workflows/test.yml --job native-test-linux --container-options "--user root"` を `tmp/gh-act-native.log` に記録しつつ再実行したところ、TypeScript tests / type-check / ESLint まで緑化した。

### リファクタリングプラン完了タスク

17. **機能使用状況マップ: アクティブ導線の呼び出し元トレース確定**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:214` で「機能名: 呼び出し元 → 実装箇所」を網羅したマップが未チェックのまま残っており、`phase5_feature_usage_map.md` の内容と Tauri コマンド一覧の整合を証跡付きで残す必要がある。  
    - やること: (1) `scripts/check-tauri-commands.mjs` の結果と `phase5_feature_usage_map.md` 3章を突き合わせ、各アクティブ導線について「UIイベント → Hook/Store → Tauri Command」の紐付け表を埋める。(2) `phase5_user_flow_inventory.md` の導線 ID と相互参照できるよう、マップにテスト ID・Nightly artefact へのリンクを追加。(3) マップ更新後に refactoring plan のチェックボックスへ反映し、エビデンスを `phase5_ci_path_audit.md` に追記。  
    - 完了条件: 主要導線ごとに呼び出し元トレースが文書で確認でき、該当チェックボックスを完了に更新できる状態。

18. **機能使用状況マップ: 未使用機能/ dead_code の棚卸し完了**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:217` にある「機能名: 実装箇所（dead_code）」が未完了で、`add_relay` など Phase5 backlog に残る未導線 API の扱いが確定していない。  
    - やること: (1) `phase5_feature_usage_map.md` と `phase5_dependency_inventory_template.md` を用い、未導線 API / dead_code の一覧に削除 or 代替導線を明記。(2) 削除対象は Rust/TypeScript 双方から撤去し、Nightly ログと CI 監査に反映。(3) 維持する場合は UI 導線作成の期日・責務を追記し、refactoring plan のチェックボックスを完了させる。  
    - 完了条件: 未使用機能一覧がゼロまたは明確な移行計画付きでドキュメント化され、対応結果を refactoring plan に反映できている。

19. **機能使用状況マップ: 部分利用機能の使用/未使用整理**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:220` で「使用箇所 / 未使用箇所」の棚卸しが未完了扱いとなっており、Summary/Inventory とメトリクスの突合が必要。  
    - やること: (1) `/profile`, `/search`, Offline Sync など部分的に利用されている UI を対象に、導線と未配線部位を `phase5_user_flow_summary.md`・`phase5_user_flow_inventory.md` と同期。(2) `docs/01_project/progressReports/` にテスト ID / artefact 参照先を記載して、部分利用箇所のトリアージ手順を Runbook 化。(3) Refactoring plan 側のチェックボックスと更新履歴を最新化。  
    - 完了条件: 部分利用機能の残課題が Inventory/Runbook 上で一元管理され、refactoring plan の該当チェックを完了できる。

20. **コード重複率30%削減の実測と実装**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:431` で KPI として掲げられているが、Zustand ストア persist 定義や Vitest モックの重複解消が未完。  
    - やること: (1) `pnpm dlx jscpd --reporters json` 等で TypeScript 側の重複レポートを採取し、`cargo llvm-cov --json` / `cargo udeps` を併用して Rust 側の重複候補を洗い出す。(2) `phase5_dependency_inventory_template.md` 2章に記載された重複パターン（Zustand persist、MockEventManager など）を共通モジュール化し、利用箇所を差し替える。(3) レポートを `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` に添付し、30% 減を達成した時点で refactoring plan を更新。  
    - 完了条件: 重複検出ツールの最新版で 30% 減を確認でき、Runbook/監査ドキュメントにも値が記録されている。

21. **未使用 API エンドポイント 0 件の達成**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:442` にて未使用 API をゼロにする KPI が未完。`phase5_user_flow_summary.md` では `add_relay` / `join_topic_by_name` などが backlog に残っている。  
    - やること: (1) `scripts/check-tauri-commands.mjs` と Inventory 3.2/3.3 の一覧を同期し、未導線コマンドの削除または UI 配線計画を確定。(2) 削除対象は `.sqlx` / Runbook の参照先を更新し、CI の `check:tauri-commands` でゼロ件を検証。(3) UI へ残す場合は導線 ID + テスト ID を作成し、Nightly artefact と紐づける。  
    - 完了条件: 未導線 API のリストが空になり、refactoring plan・Inventory・CI 監査でゼロ件が確認できる。

22. **孤立コンポーネント 0 件の証跡化**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:448` にて孤立コンポーネント撲滅が未達成。鍵管理ダイアログや一部 Summary Widget が未配線として Inventory 5章に残っている。  
    - やること: (1) `phase5_user_flow_inventory.md` 5章・`phase5_user_flow_summary.md` 2章を元に、Sidebar／Header／モーダル経由で呼び出されないコンポーネントを列挙。(2) 必要であれば導線を追加し、不要なものは削除 or Storybook サンプルへ移設。(3) 結果を Runbook/Inventory に記録し、孤立コンポーネントゼロを refactoring plan に反映。  
    - 完了条件: すべての UI コンポーネントが導線とテレメトリに接続されるか、廃止されている状態であることをドキュメントとテストで確認できる。

23. **dead_code の 80% 以上削除または利用開始**  
    - 背景: `docs/01_project/refactoring_plan_2025-08-08_v3.md:451` で掲げた KPI が未完で、`hybrid_distributor` / `event_sync` / `offline_api` などの dead_code が Phase5 backlog に残っている。  
    - やること: (1) `rg "#\\[allow(dead_code)` を基準に `.allow` リストを抽出し、`phase5_ci_path_audit.md` の dead_code セクションにリスト化。(2) 使用予定があるものは該当導線とテスト ID を追記し、不要と判断したコードは Rust/TypeScript 双方から削除して `.sqlx` や API JSON を更新。(3) `cargo clippy --all-features -- -D warnings` と `pnpm run lint` を回して警告ゼロを維持しつつ、削除率が 80% を超えたことを refactoring plan に記録。  
    - 完了条件: `#[allow(dead_code)]` 数が 20% 以下に減り、削除/使用開始のエビデンスが Runbook・CI 監査・refactoring plan に揃っている。
