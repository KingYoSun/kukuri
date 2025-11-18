[title] 作業中タスク（in_progress）

最終更新日: 2025年11月17日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### MVP Exit タスク

24. **GitHub Actions ワークフロー失敗の恒久修正**
    - 背景: `Test` ワークフローの `Format Check`/`Native Test (Linux)`/`Docker Test Suite` が、Rust フォーマットの崩れ・clippy 警告・Vitest/ESLint の失敗で毎回こけており、`gh run view` で 55660781722/55660781731/55660781733 が全滅している。
    - やること: (1) `kukuri-tauri/src-tauri` の `cargo fmt` と TS 側の Prettier を通して差分を解消。(2) clippy (`needless_borrows_for_generic_args` など) と未使用 import を除去。(3) Reply/Quote/Profile 系テストが正しくモックを掴むように import 順序を直し、`SummaryDirectMessageCard` の data-testid 更新に追従。(4) `scripts/test-docker.ps1 ts` および `gh act --workflows .github/workflows/test.yml --job {format-check,native-test-linux}` で CI 手順をホスト再現し、緑のログを添付。
    - 進捗メモ (2025年11月18日): rustfmt 差分と Prettier 欠けを解消し、TS テストをまとめて再配線。`gh act` で Format Check / Native Test を通過させ、Windows 指定の `./scripts/test-docker.ps1 ts` も成功。Native ジョブでは Docker → Rust → CLI → Vitest → ESLint まで完走することを確認済み。
    - 完了条件: GitHub Actions (Test) ワークフローが再実行で成功するか、`gh act`/`test-docker.ps1` の再現ログで全ジョブが成功していることを CI channel に報告できる状態。

### リファクタリングプラン完了タスク

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


