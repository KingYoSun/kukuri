# Issue #943: Metaverse 再表示後の Column 選択巻き戻りを調査・修正する

## 目的・対象外

Metaverse → Live → Metaverse と選択した後に、古い focus／route 処理によって Live へ戻らず、Metaverse の描画が復帰するようにする。正常な次の利用者操作、履歴移動、画面外の省資源化、session 継続を維持する。

- 対象 Issue: [#943](https://github.com/KingYoSun/kukuri/issues/943)。AC／INVAR の正本は Issue 本文。本書は調査根拠、実装手順、検証対応を補う。
- 種別: `fix`。リスク区分 **B**。共有選択状態を扱うため、Issue 指定どおり独立監査を行う。
- Scope revision: `2026-09-08-column-activation-ci-v1` を維持。本書の inventory／transition 詳細化は同じ条件の具体化であり、新要件の追加ではない。
- 調査日: 2026-09-09。調査基準: `main` / `d2610af4467f6a91fefa208952abc5728496dc29`。調査開始時の tracked／untracked 差分は 0。
- CI 失敗基準: `8d03a7ac8f5c22b98263e73f92bfc804dcd63965`。現在の HEAD と区別して before／after を記録する。
- 計画作成時の依頼範囲: 調査と計画ファイル作成。
- 実行承認: 2026-09-09 に本プランを承認済み。実装・commit・PR・必須 CI 成功後の merge を追加確認なしで進める。独立監査の条件は維持する。
- 対象外: #872 の候補追加、#873 の継続監査、#924 の Dome handoff／authority、既知 native fullscreen 停止問題の同時修正、依存更新、UI 再設計、永続化 schema／protocol／認証・network 契約の変更、無関係な一般化。

参照する正本: [PLANS.md](../PLANS.md)、[Issue lifecycle](../docs/runbooks/issue-lifecycle.md)、[検証マトリクス](../REFACTORING.md#path別検証マトリクス)、[dev runbook](../docs/runbooks/dev.md)、[ADR 0031](../docs/adr/0031-variable-span-column-workspace.md) §11–12、[DESIGN.md](../DESIGN.md) §3・6–10、[ADR 0014](../docs/adr/0014-uiux-dev-flow.md)、[UI 実装配置](../docs/architecture/desktop-ui-implementation.md)。

## 完了条件と対応

| ID | Issue の固定条件の要約 | 作業・証跡 |
| --- | --- | --- |
| AC-1 | 修正前の再選択を test／trace で再現し、発火順、browser 条件、最終選択を固定する | T1 の失敗 test とイベント列。CI の最終 snapshot だけでは未達 |
| AC-2 | 原因となる最小経路を直し、最後の選択を古い focus／route が上書きしない | T2。同じ再現条件の成功と、古い要求による activate／navigate がない証拠 |
| AC-3 | offscreen 停止・復帰、Live／Metaverse session、route／history／focus を維持する | T3–T5 の回帰・before／after・残存制約 |
| INVAR-1 | 正常な選択、route／history、a11y focus、省資源化を維持する | T2–T5。TR-1～4 と既存 contract の組合せ |
| INVAR-2 | sleep 追加、assertion 弱体化、skip、無制限 retry で隠さない | T1–T5 の試行条件・差分確認 |

修正完了は、全条件への証跡、未分類 inventory 0、必要な validation、固定 head の独立監査 PASS が揃った時点とする。Issue の Complete／Close は、別途依頼された PR／merge と merge 後の対象一致確認を経て判断する。計画作成の完了と、不具合の解消を混同しない。

## 調査結果

### 今回確認した事実

1. [失敗 job](https://github.com/KingYoSun/kukuri/actions/runs/34167073580/job/101880130851) の artifact `kukuri-desktop-visual-diff`（ID `10034593041`）を取得した。含まれるのは対象 test の `error-context.md` 1ファイルで、trace はない。`data-render-suspended=true` が expected 0／received 1、5000ms・13 poll 後も残った。最終 snapshot の207行は Live Active かつ DOM focus、228行は Metaverse Inactive。発火順はこの資料だけでは決定できない。
2. [column-immersive.spec.ts](../apps/desktop/tests/playwright/column-immersive.spec.ts) 155–193行は、900×900、fresh layout の主要5 Column＋Live＋Metaverse、developer mode 有効、Metaverse room 作成後に header `h2` へ `dispatchEvent('pointerdown')` する。189行で active、190行で runtime 復帰、191行で scene 復帰を順に観測するため、190行の成功は191行まで選択が維持された証明にならない。合成 pointerdown は実 pointer の default focus／scroll と同一ではない。
3. [ColumnCanvas.tsx](../apps/desktop/src/components/shell/ColumnCanvas.tsx) 152–185行は active 変更時に scroll し、DOM focus が body の場合に rAF で Column を focus する。cleanup に `cancelAnimationFrame` が既にあり、単なるキャンセル漏れと断定できない。callback は実行時の activeColumnId と対象 Column の一致を検証せず、body focus だけを再確認する。245–255行の `activateFromEvent` は `onFocusCapture`（411行）と desktop pointerdown（413行）から使われる。
4. 同 Canvas は `data-column-gesture-owner`／`data-column-preserve-activation` 内の activation を除外する。通常の button 等は active を変更しても route 同期は要求しない。mobile には scroll settle、programmatic target、wheel による引継ぎ、edge／indicator swipe の別経路がある。focus イベント全体を無視する修正では既存契約を満たさない。
5. [DesktopShellColumnWorkspace.tsx](../apps/desktop/src/shell/page/DesktopShellColumnWorkspace.tsx) 281–285行は `activateColumn` で store を更新してから、必要な場合だけ親の `onActivateColumn` を呼ぶ。[DesktopShellPage.tsx](../apps/desktop/src/shell/DesktopShellPage.tsx) 672–717行の `activateWorkspaceColumn` は **`loadTopics` の await 前に `syncRoute('push')`** を呼ぶ。一方、await 後には stream／game の selected ID 更新が残る。その遅着が再投影へ影響するかは未確認。
6. [useSyncRoute.ts](../apps/desktop/src/shell/routing/useSyncRoute.ts) 97–105行は render 時の location と次 URL を比較し、`pendingRouteUrlRef` を設定して navigate する。[useRouteSynchronization.ts](../apps/desktop/src/shell/routing/useRouteSynchronization.ts) 175–201行は実 hash と pending を照合し、881–903行は初期投影・route 変更・pending の追越し・game 解決中に `workspaceForRoute` を適用する。914行以降には正規化用の rAF 予約もある。
7. [useDesktopShellRouting.ts](../apps/desktop/src/shell/useDesktopShellRouting.ts) 172–194行の `scheduleAnimationFrame` は frame を追跡し unmount 時にキャンセルする。予約元 route の有効性は helper 自体では判定しない。この helper は Settings の focus 復元にも使われるため、共有 helper 全体のキャンセル方針を不用意に変えない。
8. [columnRuntime.ts](../apps/desktop/src/shell/columnRuntime.ts) の suspension は `immersive && !visible && !audioFocused`。active と独立である。同じ [ColumnRuntimeProvider](../apps/desktop/src/shell/ColumnRuntimeContext.tsx) の値を Surface と [MetaverseRoomView.tsx](../apps/desktop/src/components/extended/metaverse/MetaverseRoomView.tsx) が使い、後者は `runtime.suspended` を Scene に渡す。active だから強制描画する対処は ADR 0031 と異なる。
9. 現 HEAD と CI 失敗基準の差分を Canvas／workspace／routing ディレクトリ／対象 browser test／root page で比較した。この集合では root page だけ差分があり、locale と community-node UI 等の変更だった。`activateWorkspaceColumn` 本体は同じ。ただし周辺環境・全製品 tree が同一とは扱わない。
10. [Playwright 設定](../apps/desktop/playwright.config.ts) は trace が `on-first-retry`、retry の明示設定なし、local では `reuseExistingServer: true`、port 4176、mock build、locale `en-US`／timezone `UTC`。[Fast CI](../.github/workflows/kukuri-fast.yml) は Ubuntu、Node 22、pnpm 10.16.1、lockfile install、Chromium を使用し、browser 成功後に visual を実行する。今回の診断では初回失敗から trace を明示採取する必要がある。

### 仮説・未確認

- 第一候補は旧 active の遅延 focus → `onFocusCapture` → 再 activate。ただし既存 cleanup が効く順序では起こらないため、実際に残る順序を証明する必要がある。
- 第二候補は古い location の再投影／正規化 rAF。第三候補は `loadTopics` 後の選択 state 更新と route 再評価の連鎖。個別のコード形状だけで CI 原因とは断定しない。
- Issue に記載された Windows 5 PASS／11.1秒、過去の同製品 tree の CI 成功、#924 との履歴比較は過去の証跡。本調査で再実行した結果ではない。
- 今回は静的調査と保存済み artifact の確認まで。新しい browser trace、失敗する回帰 test、Vitest／Playwright／実機検証は未実施。AC-1 の未解決点を T1 に置く。

## UI brief と実装境界

- 対象利用者: desktop で Live／Metaverse Column を切り替える利用者。主要操作は header 選択と再表示。最後の明示選択を維持し、可視領域に応じて描画だけを再開する。
- UI 分類: ADR 0014 の不具合修正。既存の色・幅・文言・component 構造を変える設計ではなく、新しい token／画面／UI review record を前提にしない。
- 維持する状態: active、DOM focus、visible、audio focus の分離、Column ID／順序／scope、draft、session 内状態、history、通常の keyboard focus。loading／error／refresh が古い選択処理を再実行するかを対象範囲で確認する。
- 条件: 主再現は Chromium 900×900・en-US・UTC・既定 theme・通常 motion。共有 Canvas の回帰は mobile 390×844、wide desktop 1920×1080、reduced motion、Tab／実 pointer／indicator／touch を影響に合わせて選ぶ。Windows WebView2 の通常切替は実機補完対象。既知 fullscreen 不具合の解消は本 Issue の完了条件に追加しない。
- 第一の変更候補は `ColumnCanvas.tsx` とその test。route が原因なら `useRouteSynchronization.ts`／`useSyncRoute.ts` と対応 test、await 後の遅着が原因なら root page の該当 activation と統合 test を必要最小限変更する。観測のために全候補を一括修正しない。
- `ColumnSurface`、workspace slice、runtime/context、Metaverse Panel／View／Scene、Live 表示側はまず回帰確認先とする。独立した不整合が同じ AC に到達する証拠が得られた場合のみ T2 へ統合する。
- 状態を恒久的にロックせず、「旧要求からの継続処理」と「次の正当な利用者選択」を区別する。再現に応じ、予約時の要求所有者と実行時の選択を照合するか、古い予約を所有層で無効化する。commit 後だけの cleanup で十分か、同一 frame／render 前の入力にも対応が必要かを T1 の順序で判断する。

## 固定 surface inventory の詳細化

Issue の INV-1～3 を維持する。変更対象 helper からの逆引きで追加 member が判明したら、その行に名前・副作用・検証を統合する。現在の3行だけで全 caller の監査完了とはしない。

| ID | 入口・trigger／shared helper | 読み書き・副作用 | 維持する guard／invariant | transition／検証 |
| --- | --- | --- | --- | --- |
| INV-1 | Canvas header pointerdown／通常 focus → `activateFromEvent` → workspace `activate` → `activateWorkspaceColumn`。Control Center Focus、close 後の親選択、active Timeline scope 変更も同 callback の影響確認先 | active store、canonical URL／history、topic load、選択中 session ID、scroll／focus | 最後の明示選択。interactive target の route 非同期指定、gesture-owner／preserve-activation の除外、Column 不在の早期 return を維持 | TR-1～4。Canvas test、Column scope／hash-routing browser、必要な root page 統合 test |
| INV-2 | active effect の rAF、scroll settle／swipe／indicator／wheel、route observation／history／初期投影、正規化予約、activation の await 継続 | 旧 Column の focus、再 activate、route push／replace、workspace 再投影 | 旧要求が新要求を上書きしない。正当な history／Tab／次の選択は通す。unmount／Column 消失時の無効化。Settings focus 復元は同 helper の影響確認先 | TR-3・4。制御した rAF／Promise／router 順序の test、既存 pending push／history contract |
| INV-3 | IntersectionObserver → visible 集合 → `projectColumnRuntime` → provider → Surface／Metaverse View／Scene／Stream | runtime 属性、render／media 縮退、audio focus。Column 削除・同数置換時に visible 集合再構築 | visible 判定 `>0.01`、active との分離、明示 audio focus の例外、session を破棄しない | TR-1～3。runtime test、observer 再購読 test、既存 immersive browser と session 回帰 |

列挙方法: CodeGraph の `node`／`callers` で `ColumnCanvas`、`activateColumn`、`workspaceForRoute`、`useSyncRoute`、`scheduleAnimationFrame`、`projectColumnRuntime` を逆引きする。nested callback は `DesktopShellColumnWorkspace`／`DesktopShellPage` の `onActivateColumn` 登録と実呼出し、`useDesktopShellRouting` の予約元を照合する。動的 callback を件数だけで完了扱いにしない。期待する差分は旧要求の無効化と回帰証跡の追加で、公開入口・永続状態・network sink の追加は 0。

## 状態遷移の確認条件

| ID | 事前状態・event sequence | 期待状態 | 許可する I/O／副作用 | 禁止する副作用 | test／証跡 |
| --- | --- | --- | --- | --- | --- |
| TR-1 | fresh layout → Live／Metaverse 追加 → room 作成 → Metaverse 表示 | Metaverse active／visible／描画中。Live offscreen 縮退 | 通常の作成・取得、現要求の route／scroll／focus | 初期 focus／normalize による古い Column への巻き戻り | 既存 immersive test の初期節、T1 の基準 trace |
| TR-2 | Metaverse → Live。audio focus 未指定 | Live active／visible、Metaverse suspended。room／canvas／session 継続 | visibility 更新、render／media 縮退 | 切替だけによる leave／end、remount に伴う session 初期化 | 既存 immersive test＋T3 の session 継続確認 |
| TR-3 | Live → Metaverse の後に、旧 focus callback／旧 route 投影・正規化／取得完了を制御順で到着させる | 選択と URL が Metaverse を維持し、描画復帰、Live offscreen 縮退。新操作がない限り再 activate されない | 新要求の取得結果反映と可視性更新 | 古い要求による activate／focus／navigate、選択 ID の巻き戻り | T1 の失敗 test／イベント列 → T2 の同条件成功。削除済み Column／unmount も原因 owner に応じて確認 |
| TR-4 | 復帰後の次の選択、Tab／入力 control focus、Control Center Focus、history back／forward／deep link、mobile 手動 paging | 新しい明示要求の target に移動し、focus／history が通常動作する | 正当な route push／replace、focus、scroll、既存の復元 | 固定ロック、全 focus 無視、history 抑止、入力中 focus の強奪 | Canvas 既存 test、routing pending/history test、hash-routing／column-scope browser。保存済み layout の復元は既存 persistence contract |

再現に影響する loading→ready／error と遅着処理は TR-3 に含め、不要な全状態直積は作らない。認証失効・401・account 混在・DB rollback は本修正で変更する境界ではない。session／network 自体を変更する必要が判明した場合は B のまま範囲を拡張せず、影響と区分を再評価する。

## 作業

### Phase 1: 修正前の順序を固定する

| ID | AC／INVAR | 作業・対象 path | 受入条件 | 検証・証跡 | 依存 |
| --- | --- | --- | --- | --- | --- |
| T1 | AC-1、INVAR-2、INV-1～3、TR-1～3 | `column-immersive.spec.ts` と該当 component／routing test を使い、変更前 trace と最小の失敗回帰 test を作る。下記の有限診断を実行 | 原因となる入口→継続処理→再 activate を名前と時系列で特定。期待最終状態と browser 条件を固定。再現なしなら AC-1 未達を明記し見込み修正へ進まない | before commit、command、失敗 assertion、trace／イベント列、root cause 判断を progress に保存 | なし |

### Phase 2: 原因の最小修正と回帰

| ID | AC／INVAR | 作業・対象 path | 受入条件 | 検証・証跡 | 依存 |
| --- | --- | --- | --- | --- | --- |
| T2 | AC-2、INVAR-1・2、INV-1・2、TR-3・4 | T1 で特定した owner に旧要求の無効化を実装。Canvas／routing／root page の必要箇所のみ。変更 helper の全 caller と例外・早期 return を逆引き | 旧処理が再選択／navigate しない。同じ Column へ戻る連続操作も正当な次の選択も通る。cleanup と実行時 guard の役割を説明できる | T1 の同条件 PASS、正当な後続入力の positive test、変更前後の callback／route 記録 | T1 |
| T3 | AC-3、INVAR-1・2、INV-1～3、TR-1～4 | `ColumnCanvas.test.tsx`、routing tests、`columnRuntime.test.ts`、`column-immersive.spec.ts`、`hash-routing.spec.ts`、`column-scope.spec.ts` を中心に不足する回帰だけ追加 | 最終選択・URL・runtime・scene を同じ観測時点で確認。旧処理を解放した後も維持。room／Live session と draft／scope が不必要に失われない | 既存 contract と新規 test。元の合成 pointerdown 再現は保持し、必要な実 pointer／keyboard 操作を補完 | T2 |

### Phase 3: 検証・証跡・独立監査

| ID | AC／INVAR | 作業・対象 path | 受入条件 | 検証・証跡 | 依存 |
| --- | --- | --- | --- | --- | --- |
| T4 | AC-3、INVAR-1・2 | 下記 validation を実行。既存 UI の Windows WebView2 通常切替、共有操作の対象 viewport を確認 | 変更前と同条件の操作結果が成功。Linux CI の visual 比較まで完走。実機未確認と browser 成功を区別 | 実行 command／OS／commit／結果、必要な画像または trace。失敗・未実行理由と補完先 | T3 |
| T5 | AC-1～3、INVAR-1・2 | `docs/progress/2026-09-09-943-column-activation.md`（実装日が変われば日付変更）へ root cause、before／after、AC 証跡、inventory／transition、残存制約を集約 | 全条件と実装・test・実行結果が対応。今回の実装と過去 CI、既知 fullscreen 制約が混在しない。正本とのずれがあれば該当箇所を同期 | `git diff --check`、リンク・path・証跡確認。実装の PR 作成依頼時は同記録へリンク | T4 |
| T6 | AC-1～3、INVAR-1・2 | 固定 PR head と scope から別担当／別コンテキストが独立監査。実装者の結論を前提にせず入口と逆引きを再構築 | inventory の未分類・不適合・blocker が 0、全 AC 証跡と必須 CI が成功。対象変更後は delta 再監査 | 対象 SHA、scope、inventory 判定、validation、PASS／FAIL／INCONCLUSIVE。依頼された merge 後に対象一致を確認して Close 判定 | T5、PR head 固定 |

## T1 の有限診断手順

1. 実装開始時の HEAD／差分を再確認する。現 HEAD と失敗基準の製品・test・lockfile の差を記録し、必要なら失敗 commit の隔離 checkout を使用する。既存作業中の server を停止・再利用しない。port 4176 使用中なら所有元を確認し、専用 port の一時 Playwright 設定を使う。
2. 対象 test を変更前製品で初回から `--trace=on --retries=0` で採取する。locale、theme、motion、viewport、OS、Node／Chromium version、build commit、mock 有効、worker 数を記録する。まず単独1回、その後同条件5回／2 workers の有限試行とする。Windows 成功だけで Linux failure を解消扱いしない。
3. test fixture／一時診断で、単調時刻と連番、入力 origin、Column ID、activeColumnId、URL（実 hash と router location／pending）、activeElement、scrollLeft、visible 集合、runtime／scene 属性を記録する。rAF の予約・開始・cancel、focusin、workspace activate、navigate／route 投影、正規化 callback、loadTopics 完了を対応付ける。prod に恒久ログを足さず、`console.error` を使わない。
4. 未再現なら上記3候補について rAF／deferred Promise／router 更新を1つずつ制御する。旧 callback が正常にキャンセルされた後で無理に呼び出すような、実際の browser では到達しない順序を CI 原因の証拠にしない。trace と整合する最小の順序を component／hook または browser 回帰 test に落とす。
5. 単独・有限反復と3候補の制御診断で固定できなければ、調べた順序と未確認事項を記録して Linux の同条件採取へ進む。利用可能な実行環境がなければ再開条件を明記する。無制限の反復や suite の再実行で調査を延長しない。

## 検証コマンドと証跡の扱い

修正前の採取例（`apps/desktop`、専用 server／出力先を確認して実行）:

```powershell
# CI=1 は既存 server の再利用を無効化するため。この子プロセス内だけに限定する。
pwsh -NoProfile -Command '$env:CI="1"; npx pnpm@10.16.1 exec playwright test --project=chromium tests/playwright/column-immersive.spec.ts -g "offscreen Metaverse and Live" --workers=1 --retries=0 --trace=on --output=test-results/issue-943-before-single'
# 次は同じ指定で --repeat-each=5 --workers=2、別の --output を使う。
```

Linux では `CI=1 npx pnpm@10.16.1 exec playwright test ...` として同じ引数を使用する。trace 計測自体が timing を変える可能性も記録し、取得できた trace だけから未観測経路を断定しない。before の artifact を別場所へ退避してから after／全 suite を実行し、上書きを防ぐ。

対象 unit／contract の初期セット（`apps/desktop`）:

```text
npx pnpm@10.16.1 exec vitest run src/components/shell/ColumnCanvas.test.tsx src/shell/columnRuntime.test.ts src/shell/routing/useRouteSynchronization.test.tsx src/shell/routing/useSyncRoute.test.tsx src/shell/routing/routeWorkspaceProjection.test.ts
npx pnpm@10.16.1 exec playwright test --project=chromium tests/playwright/column-immersive.spec.ts tests/playwright/hash-routing.spec.ts tests/playwright/column-scope.spec.ts --retries=0
```

- T1 の再現 test を上記へ追加する。共有 routing helper を変更したら `useDesktopShellRouting.test.tsx`、root activation を変更したらその新規統合 test を追加実行する。runtime／View へ変更が及ぶ場合は既存 Metaverse View／Scene test も対象にする。
- 既存 offscreen test は Metaverse room／canvas を検査するが、実際の Live session 作成・継続までは証明しない。T3 では既存 fixture／test を確認し、Live の selected session／参加状態の保持を検査する不足分だけ補う。browser mock の成功を実 network session の成功とは扱わない。
- 最終必須入口は repo root の `cargo xtask desktop-ui-check`。Linux CI の `desktop-visual-test` が実比較まで成功することを確認する。Windows の非 CI visual は到達 smoke のため、画像比較の代替にならない。通常の見た目を変えない修正で baseline を更新して失敗を隠さない。
- 日常 integration の `cargo xtask check`／`cargo xtask test` は実装時の依存・所要時間を確認して実行する。ローカル不能なら狭い関連検証を先行し、未実行範囲と CI で補う項目を明記する。関係しない CN／network scenario を追加 gate にしない。
- `git diff --check` と必要な大型ファイル ratchet 確認を行う。root page／routing が上限へ達する場合も、大規模分割や baseline 緩和を自動で混ぜない。

## 未決事項・次の一手

- 原因 owner と正確な interleaving は T1 で決める。調査済みの候補が失敗しなければ、その不成立も証拠として残す。
- Linux 同条件の新規再現、Windows WebView2 の実操作、独立監査は未実施。現在の計画作成を妨げる追加仕様判断はない。
- 実装開始後は T1 の before 証跡を最初の成果物にする。製品変更に進む条件は再現／原因順序の固定であり、過去の PASS や候補コードの存在だけを根拠にしない。

## 実行状況（2026-09-09）

T1～T5 の実装とローカル確認を実施。古い router location との比較で最後の navigate が欠落する順序を unit／Chromium で再現し、`useSyncRoute` だけを修正した。現在判定と証跡は [作業記録](../docs/progress/2026-09-09-943-column-activation.md) に集約する。上記の「未確認」は計画作成時点の記録であり、実行結果は同記録を参照する。
