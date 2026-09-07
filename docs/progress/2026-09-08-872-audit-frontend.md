# Issue #872 Phase 0–2: desktop frontend 有限監査

## 対象と開始 trigger

- 担当 inventory: `INV-3`（desktop frontend の route / store / loader / action / view / API adapter）。
- 基準: `main` の `ac9946d66aaf197204f669a10701fac8a13877e9`、2026-09-08。
- 比較元: [2026-05-28 responsibility refactoring plan](2026-05-28-responsibility-refactoring-plan.md)。変更圧力の期間は `git log --since=2026-05-29`。
- trigger: Column workspace 移行後の非 active Column 更新、通知既読状態、route 再投影に関する実際の修正履歴。
- 実装・修正・test 追加は行っていない。以下の「実施」は個別 Issue 化の選定であり、リファクタリング実装済みを意味しない。
- 正本: [REFACTORING.md](../../REFACTORING.md)、[Issue lifecycle](../runbooks/issue-lifecycle.md)、[UI 実装配置](../architecture/desktop-ui-implementation.md)、[DESIGN.md](../../DESIGN.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[ADR 0023](../adr/0023-local-notification-inbox-v1.md)、[ADR 0031](../adr/0031-variable-span-column-workspace.md)。通知内容の表示は [ADR 0046](../adr/0046-age-attestation-adult-content-gating.md)、OS activation は [ADR 0049](../adr/0049-linux-gui-cli-control-plane.md)も維持対象。

## 有限 scope と列挙方法

監査は route、store、loader、action、view、API adapter の現行責務境界と、以下の四候補を分類して終了する。frontend 全ファイルの動作証明や各 component の新規 UI 監査は含めない。

`.codegraph/` が存在し、`codegraph explore` / `codegraph node` が利用できた。最初に shell / route / store / loader / action / view-model / runtimeApi を探索し、その後に全 member と文字列依存を `rg` で補完した。CodeGraph の caller 数だけを完全性の根拠にしない。存在しない `commands/dispatch.ts` を一度照会したが、実在する `invoke/dispatch.ts` に解決して読んだ。index 新規作成・更新は行っていない。

```powershell
# 本 inventory の機械的な母集合（基準 commit で 188 files）
rg --files apps/desktop/src/shell apps/desktop/src/lib/api apps/desktop/src/mocks | Sort-Object
# frontend 全体の登録漏れ確認（487 files。型、test、story、CSS、locale も含む）
rg --files apps/desktop/src | Sort-Object
# exports、入口、sink を候補ごとに逆引きする
rg -n 'export (function|const|type)|useEffect|setInterval|addEventListener' apps/desktop/src/shell
rg -n 'getNotificationStatus|listNotifications|markNotificationRead|markAllNotificationsRead' apps/desktop/src
rg -n 'loadNotificationsSection|refreshNotificationStatus' apps/desktop/src
git log --since=2026-05-29 --format='%h %ad %s' --date=short -- apps/desktop/src/shell apps/desktop/src/lib/api apps/desktop/src/mocks
```

全体 487 files の path 分類は root 5 / components 212 / i18n 31 / lib 68 / mocks 13 / shell 132 / stories 10 / styles 15 / test 1。188 files の中には tests / stories 68、型・fixture 19 が含まれる。母集合を列挙したことを全 487 files の実行確認とは扱わない。

| 群 | 全 member の列挙方法 / anchor | 現在の入口 → helper → 読み書き・副作用 | 維持境界と既存確認面 | 分類 |
|---|---|---|---|---|
| INV-3-1 起動・単一 store | `App.tsx`、`shell/store.ts`、`shell/slices/*`、`shell/*Persistence.ts`、`shell/savedWorkspaceLayouts.ts`、`shell/columnRuntime.ts`、`shell/ColumnRuntimeContext.tsx` | App → `createDesktopShellStore` → workspace / draft / preference 読取り、単一 Zustand の `setField` / `patchState`。state の定義は 12 slices の合成 | slice 横断更新の原子性、local layout と canonical URL の分離。`store.test.ts`、`workspacePersistence.test.ts`、`columnDraftPersistence.test.ts`、`savedWorkspaceLayouts.test.ts` | 現行境界維持（F-3） |
| INV-3-2 routing | `shell/routes.ts`、`shell/useDesktopShellRouting.ts`、`shell/routing/*` | navigation / location / Escape → `useSyncRoute` / `useRouteSynchronization` / route projection → URL、workspace、選択対象、thread loader | pending route の追い越し、Column parent、private scope、focus。`routing/*.test.*`、`DesktopShellPage.routing.test.tsx`、`DesktopShellPage.columnParents.test.tsx`、`DesktopShellPage.notificationFocus.test.tsx` | 延期（F-2） |
| INV-3-3 data | `shell/useDesktopShellData.ts`、`shell/data/*`、`shell/data/loaders/*` | startup / interval / focus / visibility / runtime event / section 遷移 / explicit refresh → queued topic loader、section loaders → DesktopApi と store | source topic・channel scope、dirty editor、partial failure、非 active Column。`useDesktopShellData.test.tsx`、loader tests、各 shell integration | 通知部分だけ実施選定（F-1）、他は維持 |
| INV-3-4 actions | `shell/useDesktopShellActions.ts`、`shell/actions/*` | UI callback → `create*Actions` / optimistic post helpers → DesktopApi mutation / state / route | publish target、DM、private audience、notification navigation の要求識別子。`useDesktopShellActions.test.tsx`、`actions/metaverse.test.ts`、`DesktopShellPage.*.test.tsx` | 既存 feature 所有を維持（F-3） |
| INV-3-5 view / page | `shell/useDesktopShellViewModels.ts`、`shell/viewModels/*`、`shell/presentation.ts`、`shell/page/*`、`shell/DesktopShellPage.tsx`。描画側は `components/*` | store selector → projection → page / component props、local dialog / focus / preview lifecycle | 全 store 購読を増やさない。UI flow / CSS cascade / adult preview gate / Column state を維持。view-model tests、render isolation、component tests、browser / visual | 再分割を却下（F-3）。通知手動 refresh の wiring のみ F-1 |
| INV-3-6 API adapter | `lib/api/*`、`lib/api/commands/*`、`lib/api/invoke/*` | `DesktopApi` → `command`（呼出時の mock 判定）→ `invokeDesktop` → IPC / error normalization | IPC 名、generated DTO、`satisfies`、mock の `this` 束縛、error envelope。`runtimeApi.test.ts`、`invoke/error.test.ts`、view contract、mock 利用 integration | domain 全面分割は延期（F-4） |
| INV-3-7 mocks / tests / stories / style / locale | `mocks/*`、`**/*.test.*`、`**/*.stories.*`、`stories/*`、`styles/*`、`i18n/*`、`test/*` | mock factory → 8 domain factory + 共有 runtime、story/test seed → 本番 component / gate | assert・fixture・visual contract を弱めない。CSS scoped override は現役、mock 内 `this.otherMethod()` を維持 | 旧候補の再起票却下（F-3）、新規 UI/locale 修正は別種別 |

`other` として残る本 inventory の shell hook は `useAppUpdateStore.ts`、`useOsNotificationActivation.ts`、`useOsNotificationBridge.ts`、`testSupport/renderShellHook.tsx` である。更新 scheduler / OS activation / OS 設定転送 / test harness として INV-3-5 / 7 に分類し、通知一覧の取得 owner とは混ぜない。

## 前回候補の継続と陳腐化

| 2026-05-28 の記録 | 現行 evidence | 今回の判断 |
|---|---|---|
| `DesktopShellPage.test.tsx` の behavior 分割が残作業 | `a1bab0b9`（2026-07-03、#428）でテーマ別分割。現行は `DesktopShellPage.*.test.tsx` | 古い残作業は実施済み。巨大 test という理由で再起票しない |
| `desktopApiMock.ts` の domain 分割が残作業 | `0c9aa670`（2026-07-08、#505）。現行 factory は 34 行、8 domain factory と共有 `mockRuntime` を合成 | 実施済み。再分割は却下 |
| shell data/action/view model の feature 所有 | `b9ac151f` #502 store slices、`72f62349` #501 selectors、`8e988046` #531 loaders、`14fdeccc` loader 正本化、`271ba04b` view-model 抽出 | 現行境界は成立。F-1 はその後の通知経路の再増殖に対する限定的な継続 |
| Metaverse room の scene / view 分離 | 前回 closeout で scene/model 抽出済み。その後の Dome / Column 機能追加は別の製品変更 | 古い room panel のサイズだけを再利用しない。今回は scene 再設計を対象外 |

## 候補一覧

| 候補 | 観測した問題と根拠 | 目標成果 | 優先順位 / 分類 | 理由・後続 |
|---|---|---|---|---|
| F-1 通知取得・state 反映を単一 loader owner へ抽出 | `useDesktopShellDataEffects.refreshNotificationStatus` と `useDesktopShellSectionLoaders.loadNotificationsSection` が両方とも status/list API と結果の store 反映を所有。`586aaee4`（2026-08-30）の background inbox 修正では page / effects / section loaders / data facade の 4 製品ファイルを変更 | 意図的に異なる badge / inbox の取得モードを維持したまま取得 owner 2 → 1、effects から通知 API と取得結果の setter を除く | P1 / 実施 | contract 先行 F-1C → `refactor:extract` F-1R。既読化は永続 mutation のため両方 C |
| F-2 route / Column state の再抽出 | `fa21da6b`（#847）の pending push 追い越し、`192dc01a` の Column flicker、`9fb13c36`（#771）の親子関係修正。変更圧力はある | 将来再調査時は URL projection と navigation orchestration の所有を測る | P2 / 延期 | 現行は既に route projection / synchronization / sync facade に分離。具体的に減らす正本や依存を今回示せない。新しい抽象層を起票しない |
| F-3 前回の test/mock/store/view-model 再分割 | 前節の実施済み evidence、store 12 slices の合成、mock 8 domain factory | 現行境界の維持 | 却下 | 旧ファイル行数や古い follow-up だけを根拠とした再起票は不要。slice 名と action 名は分類軸が異なるため名前合わせも不要 |
| F-4 runtimeApi を domain adapter 群へ全面分割 | 期間内に 25 commits。`domeTransitionApi` / `socialBlockApi` は `Pick<DesktopApi, ...>` を合成する一方、既存 domain の request literal は `runtimeApi.ts` にある | 将来対象 domain の adapter owner と import 範囲を限定 | P2 / 延期 | mock 分岐と request 型は既に正本化済み。直近変更の取り違え・契約漏れがこの配置に起因する evidence は得られず、25 commits や行数だけでは着手不可 |

既知の UI 不具合 #913–#918 は親監査が列挙した別種別の既存 Issue として扱う。本監査では再現や新規 blocker 判定を行っておらず、refactor に混ぜず、重複 Issue を作らない。

## F-1 の観測した事実と現在の責務

`useDesktopShellDataEffects.ts:278` の `refreshNotificationStatus` は hidden 時に何もせず、status を取得して即時反映する。未読ありかつ active section が notifications 以外なら list も取得して反映する。失敗は best effort として保持値を残す。これを mount / 60 秒 interval と `useRuntimeEventBridge` の `notification_status_changed` が呼ぶ。

`useDesktopShellSectionLoaders.ts:314` の `loadNotificationsSection(options)` は status/list を `Promise.all` で取得する。`markAsRead` は省略時 true。true かつ未読項目ありなら mark-all を実行し、返った status と、`Date.now()` を未読項目の `read_at` にだけ入れた list を反映する。mark-all 失敗時は unread と auto-read error を残しつつ panel は ready。status/list の取得失敗は panel error。active inbox、非 active inbox、page の手動 refresh、`loadShellSections` がこの入口を使う。

両方が status/list を呼ぶが、同一の取得 policy ではない。「重複だから同じ関数へ置き換える」のではなく、通知 domain の同一 owner に既存の二つの操作を置く。同期の抑止、並行要求の新しい勝者判定、既読契約、通信量の最適化は本 Issue に追加しない。

`586aaee4` の 4 製品ファイルへの波及と、上記 2 owner は観測事実である。単一 owner が次回の通知変更の調査範囲を狭めることは構造に基づく期待効果であり、不具合件数の低減を測定済みとは扱わない。

## F-1 固定 surface inventory と sensitive sink 逆引き

以下は F-1C / F-1R 共通の `INV`。各 Issue は本表を固定参照し、実装開始時に基準 SHA の差分を確認する。

| ID | 入口・trigger / 全 caller | shared helper | 読み書き・副作用 | guard / invariant | TR / evidence |
|---|---|---|---|---|---|
| INV-1 | data effects mount、60 秒 interval、activePrimarySection変更によるcallback再生成とeffect再実行 | `refreshNotificationStatus` | get status、条件付き list、通知 status/list store 更新、即時refreshとinterval再設定 | hidden no-op、active inbox中はstatusのみ、badge は mark 禁止、失敗は best effort | TR-1/2/3/8、先行 contract |
| INV-2 | `kukuri://runtime-event` の `notification_status_changed` | `useRuntimeEventBridge` → 同 refresh | INV-1 と同じ。listener setup / cleanup | Tauri 判定、最新 callback ref、unsubscribe。sync event は既存別 callback | TR-1/2/8、`useRuntimeEventBridge.test.ts` |
| INV-3 | section effect: active notifications または background notifications Column | `loadNotificationsSection({markAsRead: active})` | status/list、条件付き mark-all、panel / auto-read error | background は mark 禁止、active でも全既読なら mark 禁止 | TR-4/5/6、notifications integration |
| INV-4 | `loadTopics` → `queuedLoadTopics` → `runLoadTopics` → `loadShellSections` の active notifications 分岐 | `loadNotificationsSection()` | 同上。`runLoadTopics` は先に `refreshVisibleShellData` を待つ | 省略時 true を維持。無関係 section から mark しない | TR-4/5、`useDesktopShellData.test.tsx` |
| INV-5 | `DesktopShellPage.refreshNotificationsColumn` | `loadNotificationsSection({markAsRead: ...})` | 呼出前に auto-read error clear / panel loading、その後は loader | background refresh が ready/error で完了し未読を維持 | TR-5/6、`DesktopShellPage.notifications.test.tsx` |
| INV-6 | loader の `api.markAllNotificationsRead` | runtimeApi → `command` → `invokeDesktop('mark_all_notifications_read')` → Tauri `commands/profile.rs` → runtime `notifications_messages_api.rs` → AppService `notifications.rs` | `NotificationStore` → SQLite `notifications.read_at = COALESCE(read_at, timestamp)`、runtime status-change event | local-only、既存 read_at 不変、IPC/DTO 無変更 | TR-4/6、既存 app-api `mark_notification_read_and_mark_all_read_update_unread_count` |
| INV-7 | 通知描画、OS activation と通知クリック | store → `DesktopShellNotificationsSurface` / `useOsNotificationActivation` / action `handleOpenNotification` | 既存表示・対象 navigation。OS dispatch は Rust 所有、`useOsNotificationBridge` は設定だけ転送 | adult preview gate、private scope、notification ID、Column parent/focus の契約 | `DesktopShellPage.notifications.test.tsx` / `notificationFocus` / `osNotificationActivation` / `adultContentGating` |

基準 SHA で製品 frontend に `api.getNotificationStatus` と `api.listNotifications` を直接呼ぶ箇所は上記の 2 owner、`api.markAllNotificationsRead` は section loader の 1 箇所。adapter はそれぞれ `runtimeApi.ts:339–352`。公開 `markNotificationRead` adapter は実在するが、この監査の検索では製品 frontend の呼出は見つからない。Rust、CLI、登録簿に契約が存在するので dead code として削除しない。

全 caller の確認は前掲 `rg` を tests/mock/型を含めて実行し、その後 `--glob '!**/*.test.*' --glob '!**/*.stories.*' --glob '!**/types*' --glob '!**/mocks/**'` で製品側を分離する。hook 戻り値の forwarding は `useDesktopShellData.ts` から effects と page への配線、section loader 内部の batch 呼出をすべて辿る。runtime event の動的登録は `useRuntimeEventBridge` の文字列と Tauri command 登録 `src-tauri/src/lib.rs` を照合した。

## F-1 状態遷移と contract 草案

新しい product requirement は追加せず、現行で実行可能な操作と禁止副作用を固定する。下記で「新規 contract」は未実装・未実行である。

| ID | 事前状態 / event | 期待 state と許可 I/O | 禁止する副作用 | 既存 / 先行確認 |
|---|---|---|---|---|
| TR-1 | hidden document、badge interval / runtime event | 変更なし、通知 API 0 回 | status/list/mark | 新規 contract: hook harness + visibility、API spies |
| TR-2 | visible、active inbox 内 / 外、未読 count が正 / 0、badge refresh | status取得・反映。active inbox外かつcount正の場合だけlist取得。active inbox内は未読ありでもlist 0回、既存list保持 | mark、panel loading/error の新規変更 | 新規 contractで全4分岐、既存 `loads unread notification rows outside the inbox` |
| TR-3 | badge status 成功、list 失敗 / status 失敗 | list 失敗でも成功済み status は保持、旧 list 保持。status 失敗は両保持 | mark、inbox error semantics への置換 | 新規 contract: 2 failure branches |
| TR-4 | active inbox、未読あり / 全既読、section/batch load | status/list。未読ありだけ mark、成功後未読項目だけ read_at 投影。既読値維持 | 全既読時 mark、read_at 再書込み、通知生成 | 既存 data hook / notifications integration + 新規混在 fixture |
| TR-5 | 非 active inbox Column、initial load / manual refresh | status/list、ready または error、未読維持 | mark、route/active Column 変更 | 既存 `a visible background notifications column...` / `refreshing a background notifications column...` |
| TR-6 | inbox status/list の片方失敗 / mark-all 失敗 | 取得失敗は panel error で mark なし。mark 失敗は unread 保持、auto-read error、panel ready | 失敗を既読成功扱い、新しい自動 retry | 既存 load / auto-read error tests、新規 status/list の各 failure |
| TR-7 | 複数通知が混在、再 refresh / 個別通知 activation | 全既読 refresh で余分な mark なし。actor/topic/channel/object/labels 等の payload と順序を保持 | 別通知への置換、private scope 拡張、adult preview の解除 | 既存 notification navigation / focus / adult gating integration、新規 mixed fixture |
| TR-8 | effects mount → activePrimarySection変更 → event → unmount → remount | section変更時はcallback再生成に伴い旧intervalを解除、即時refreshと60秒interval再設定。listener数とcallback更新は現行同等。unmount後の新しいtimer/event発火なし | duplicate listener、新規 late-response cancellation policy、section変更triggerの消失 | 既存 event bridge test + 新規 fake timer / event contract |

進行中要求の完了順を変える必要が見つかったら fix に分離する。TR-8 は取消し処理の新仕様を要求せず、現在の cleanup の維持を対象にする。

## 個別 Issue 草案 F-1C

- タイトル案: `contract: 通知のbadge更新とinbox読込みのモード別副作用を固定する`
- Current status: Planned。Scope revision: `2026-09-08-F-1C`。基準 commit: `ac9946d66aaf197204f669a10701fac8a13877e9`。Blocker: 0（後続実装の前提を作る Issue）。
- リスク区分: C。製品コード変更はないが、後続が local 永続既読 mutation / shared loader を動かすため、その禁止 I/O の contract を先に固定する。
- Goal: 利用者が background 通知を表示・更新しても未読を失わず、active inbox の既読処理と失敗状態が現在と同じになることを、構造変更前のコードで確認できる。
- 親: #872 の INV-3 / F-1。責務所有: この Issue は test evidence のみ。loader 抽出は F-1R が所有。
- In scope: `apps/desktop/src/shell/data/loaders/useDesktopShellSectionLoaders.test.tsx`、`apps/desktop/src/shell/useDesktopShellData.test.tsx`、`apps/desktop/src/shell/data/useRuntimeEventBridge.test.ts`。必要なら同じ test 領域に通知 domain の test file を追加する。既存 `testSupport/renderShellHook.tsx` を利用し、test 用抽象層の全面変更はしない。
- Non-goals: 製品コード修正、cancel/retry 新仕様、API/DTO 変更、OS notification dispatch、通知クリック遷移の変更、UI redesign、テスト分割自体を成果にすること。
- AC-1: 本表 INV-1–7 / TR-1–8 に既存または新規 test 名を対応させ、未対応 0 とする。
- AC-2: TR-1/2/3/5/6 で許可 API と禁止 mark-all を spy の呼出回数、fixture の unread/read_at、既存値保持で検査する。成功表示だけで済ませない。
- AC-3: 混在 fixture の content_labels / private channel / read_at / 順序を維持し、現在の badge と inbox の error semantics の違いを test で区別する。
- AC-4: 新規 test を製品コード変更前に実行し、既存 notifications/data/event suites とともに baseline を SHA・command・結果付きで記録する。予想と現行が矛盾する場合は fix を混ぜず分類を親へ戻す。
- INVAR-1: 製品コードと public IPC/DTO は無変更。
- INVAR-2: 既存 test の削除、skip、assertion 弱体化なし。snapshot は既存 wire/visual 契約以外に新設しない。
- Inventory / transitions: F-1 の INV-1–7 / TR-1–8 を固定参照。
- Validation: `cd apps/desktop; npx pnpm@10.16.1 test -- <対象test path群>`（package script は `vitest run`）。最終は `cargo xtask desktop-ui-check` と `git diff --check`。CI の Linux visual 比較を含み、Windows local smoke を visual 比較成功と報告しない。
- Rollback: test だけの PR を revert。F-1R 開始前なら製品挙動への影響なし。新しく見つけた契約矛盾は別 Issue に evidence を残す。
- Close: C の独立監査、必須検証、merge/監査対象 SHA の一致確認後。現時点では未着手。

## 個別 Issue 草案 F-1R

- タイトル案: `refactor:extract 通知取得とstate反映を単一loaderへ集約する`
- Current status: Planned。Scope revision: `2026-09-08-F-1R`。基準 commit: `ac9946d66aaf197204f669a10701fac8a13877e9`。開始依存: F-1C の merge と baseline PASS。完了 blocker とは区別して dependency に記録する。
- リスク区分: C（mark-all は local DB mutation、複数通知の global apply、shared loader）。
- Goal: 利用者の通知表示・未読・refresh・失敗時の挙動を維持し、通知取得 policy と取得結果の state 反映を一つの責務として変更・review できるようにする。
- 種別・一意図: `refactor:extract`。通知の取得・結果反映の owner を統一する。badge と inbox の別操作は維持する。
- In scope: `apps/desktop/src/shell/data/useDesktopShellDataEffects.ts`、`apps/desktop/src/shell/data/loaders/useDesktopShellSectionLoaders.ts`、`apps/desktop/src/shell/useDesktopShellData.ts`、通知 owner 用の新規 `apps/desktop/src/shell/data/loaders/useNotificationLoaders.ts`（提案 path）、対応 test。`DesktopShellPage.tsx` は必要な型/配線変更だけ許容し、manual refresh の開始状態と markAsRead 判定は維持する。
- Target structure: 新規通知 loader hook が `refreshNotificationStatus` と `loadNotificationsSection` の実装と通知結果 setter を所有。data facade が一回生成し、effects に badge/event callback、section facade に inbox callback を渡す。effects は event/interval/section trigger と cleanup を所有し続ける。page の手動 refresh 入口と public facade の戻り値は維持する。
- Non-goals: section 全体分割、routing/store slices 再設計、OS 通知 dispatch・設定・permission・activation 変更、read policy 統一、重複要求の coalescing、取消し/順序 policy 変更、network 最適化、bug fix、Rust / DB / IPC / UI/locale/CSS の変更。
- AC-1: 通知 status/list の API 呼出と結果 state 反映の製品 owner を 2 modules → 1 module にする。移動前後の path/symbol 対応を記録する。
- AC-2: effects の通知取得 API 呼出と取得結果 setter は 0 箇所、section loader の通知取得 inline implementation は 0 箇所。どちらも同じ owner で生成された callback を使用する。既存 mark-all の製品呼出箇所数 1 を維持する。
- AC-3: F-1C の INV-1–7 / TR-1–8 と既存 notifications/data/event/navigation tests が同じ外部結果で成功する。timer cadence、visibility no-op、markAsRead default、error semantics、read_at 投影を変えない。
- AC-4: `DesktopApi` / command 名 / generated DTO / runtime event 名 / locale key / UI flow に差分なし。全 caller 逆引きで旧 owner の残存経路と新規経路の重複なしを確認する。
- AC-5: before/after owner map、validation、残存リスク、差し戻し手順を記録し、独立監査 PASS 後に Close する。
- INVAR-1: 通知は local-only。private preview / content_labels をそのまま保持し、adult display gate を迂回しない。
- INVAR-2: badge と background inbox は mark しない。active inbox は未読ありの場合のみ既存 mark-all を実行し、全既読なら追加 mutation をしない。
- INVAR-3: status/list と mark-all の失敗時 state、panel error と auto-read error の分離を維持する。
- INVAR-4: 単一 Zustand store、mock 呼出時 dispatch と `this`、現行 `Date.now()`、callback dependency、event setup/cleanup、非 active Column の loading 終端を維持する。
- INVAR-5: 既存 DTO / serialized shape / API・event 名 / DB schema / identity / audience / route / UI 採用記録は変更しない。
- Baseline / 成功指標: owner 2 → 1、effects direct status/list 呼出 2 → 0、section inline owner 1 → 0、mark-all callsite 1 → 1。行数減少やファイル分割自体は完了条件にしない。静的構造の確認は review evidence とし、構造だけを検査する test は追加しない。
- PR 分割: F-1C を先行 PR として merge、F-1R は一つの抽出 PR。全通知呼出の移動と配線は同じ差分で閉じ、関係ない feature loader を移さない。
- Validation: F-1C の targeted command を抽出前後に実施。必須は `cargo xtask desktop-ui-check`、`git diff --check`。既存 notificationFocus / osNotificationActivation / adultContentGating integrations も full suite 内で維持する。Rust は path 対象外。Rust/IPC 変更が必要になれば本 refactor を停止して scope を再分類する。
- Rollback: F-1R の単一 PR を revert。新規 owner の実装を 2 旧 owner に戻す機械対応が追える差分にする。F-1C tests は保持できる。DB migration や利用者データ変換を持ち込まない。
- 残存リスク: 現行の並行完了順、callback closure、mark-all event と定期更新の競合は元の挙動を固定して維持する。別の取消しモデルを導入しない。実施開始までに対象 path が変わったら delta を同じ inventory へ再対応する。

## 監査終了判定

- 固定した責務群と四候補を分類済み。実施候補は F-1 の一意図だけで、contract と抽出を別 Issue にする。F-2/F-4 は延期、F-3 は却下。
- 今回の担当では製品コード、test、GitHub、commit を変更していない。成果物はこの報告のみ。
- CodeGraph / 静的検索 / 履歴 / 既存 test の確認を実施した。test の成功は本調査だけでは主張しない。親監査で実行する baseline と、この報告の構造 evidence を分けて管理する。
- 本報告は区分 A の文書として `git diff --check` と参照確認を行う。F-1C/F-1R の必須製品 validation と独立監査は後続の未実施項目である。
