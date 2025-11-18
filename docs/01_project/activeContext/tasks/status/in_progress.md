[title] 作業中タスク（in_progress）

最終更新日: 2025年11月17日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### MVP Exit タスク

- （現在進行中のタスクはありません。2025年11月17日更新）

### リファクタリングプラン完了タスク

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
    - メモ (2025年11月17日): `gh run view 19407530345 --job profile-avatar-sync` で `bash: -c: line 12: syntax error near unexpected token '|'`、`gh run view 19407530345 --job desktop-e2e` で `Unknown command: e2e` と判明したため、`scripts/test-docker.sh` の `profile-avatar-sync` コマンドを `pnpm vitest run … 2>&1 | tee …` 形式に修正し、`COMMAND` 判定へ `e2e` を追加。併せて `Dockerfile.test` に `RUN chmod +x scripts/docker/ts-test-entrypoint.sh` を追加し Windows ホストでも `gh act` で ts-test entrypoint が実行できるようにした。`gh act --workflows .github/workflows/nightly.yml --job profile-avatar-sync` では Vitest 本体が完走し（act 上は Windows パスの volume 制限によりログ upload ステップでのみ失敗）、`--job desktop-e2e` は artefact upload を除き成功することを確認したため、本番 GitHub Actions でも再実行で緑化できる見込み。


