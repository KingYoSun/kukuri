# #994 ブックマーク・ソーシャル一覧の空状態と始め方の案内

- 判定: In progress（実装・ローカル検証完了。CI／merge の最終結果は Issue の Current status と PR を参照する）
- Scope revision: `994-plan-v3 / 2026-09-13`（ユーザー承認済み。v1 の対象外だったフォロー中／フォロワー／ブロック中、chip による実ボタンの明示、ブックマークの loading／error、一覧の最低0.5秒 loading を同じ Issue で扱う）
- 基準commit: `b6e249f`
- リスク区分: B。表示、既存導線の呼出し、表示用 status の追加のみ。network / IPC / 認証 / 同意 / 永続化は変更しない。
- UI分類: 既存画面の改善。利用者は初回起動直後に一覧を開いた利用者。単一目的は、0件の一覧で「どの画面のどのボタンを押すか」と次に移る操作を同じ場所に示し、取得中を0件と混同させないこと。
- 対象外: ブックマーク／フォロー／ミュート／ブロック機能自体の変更、新しい network / storage sink、Tauri 実機確認。
- 正本: [Issue運用手順](../runbooks/issue-lifecycle.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[ADR 0013](../adr/0013-social-graph-foundation-draft.md)、[ADR 0021](../adr/0021-local-post-bookmark-data-classification.md)、[DESIGN](../../DESIGN.md) 4.2、[UI実装配置](../architecture/desktop-ui-implementation.md)。

## 調査で固定した事実

- ブックマーク一覧は `TimelineFeed` の `emptyCopy` 経路で1行だけを描画し、`bookmarkedPosts`（初期値 `[]`）に status が無いため取得前・失敗時も0件表示だった（DESIGN 4.2 の false empty に該当する既存 gap）。取得は section 遷移ごとに best effort で走り、失敗は握りつぶされていた。
- フォロー中／フォロワー／ミュート中／ブロック中は `ProfileConnectionsPanel` が共通の `socialConnectionsPanelState` で loading / error を出し、`ready` かつ0件で1行だけを描画していた。プロフィール概要と設定「安全」の両方から同じ panel に到達する。
- 実ボタン: ブックマークは投稿カードの icon-only ボタン（lucide `Bookmark`、操作名「ブックマーク」）。フォロー／ミュート／ブロックは投稿者プロフィールの文字ボタン。ミュートは投稿の通報 icon（`Flag`）ダイアログからも選べる。
- ブックマークは端末ローカル（ADR 0021）。ミュートは端末ローカルで相手に伝わらず、ブロックは署名され他端末へ同期される（設定「安全」の既存文言 `settings:safety.social.*` をそのまま補足に使う）。フォロワーは届いた follow 情報から端末内で導出する（ADR 0013）。
- 最低表示時間の先例は #956 の `useAcknowledgedPending`（操作ボタン、結果反映を遅らせない）。一覧の初回 loading は初回表示自体を遅らせるため、DESIGN 4.2 に一覧用の規則を追記して区別した。

## 修正前の再現

- 2026-09-13、基準 commit に次の test だけを置いて Vitest を実行し、4 file が失敗した（[記録](assets/issue-994/t1-before.log)）。
  - `useMinimumLoading.test.tsx` / `ProfileConnectionsPanel.emptyState.test.tsx`: module 不在で import 失敗。
  - `TimelineFeed.emptyState.test.tsx`: `emptyState` prop が無く、従来の1行文言が出る（2件失敗）。
  - `DesktopShellPage.bookmarksEmptyState.test.tsx`: ブックマーク view に `role="status"` の loading も案内も無く、失敗時も再試行が無い（2件失敗）。
- Issue 記載の 0.2.2 実機観測（Debian、日本語）は「まだブックマークした投稿がありません。」「ミュート中のユーザーはいません。」の1行のみ。本セッションは remote container のため実機の変更前観測は取得していない。

## 固定した受入条件・不変条件

| ID | 条件 | 作業 / 証跡 |
| --- | --- | --- |
| AC-1 | ブックマーク0件で、`Bookmark` icon と操作名の chip、投稿カード上にあること、端末内保存、「タイムラインを見る」 | T2, T3 / TR-1 |
| AC-2 | フォロー中0件で、投稿者名／avatar からプロフィールを開く手順、「フォロー」chip、「タイムラインを見る」「見つけるで探す」 | T4 / TR-1 |
| AC-3 | フォロワー0件で、相手のフォローがこの端末に届くと表示されること、「ユーザーIDをコピー」「タイムラインを見る」 | T4 / TR-1 |
| AC-4 | ミュート中0件で、「ミュート」chip と通報 icon 経由、端末内設定で相手に伝わらないこと、「タイムラインを見る」 | T4 / TR-1 |
| AC-5 | ブロック中0件で、「ブロック」chip、署名・他端末同期の補足、「タイムラインを見る」 | T4 / TR-1 |
| AC-6 | chip は操作不能・focus 不能、icon は `aria-hidden`、操作名は可視文字、実ボタンと同じ i18n key | T2 / TR-1 |
| AC-7 | CTA が実動。ブックマークは同じ Column を feed へ戻し（#765 維持）、他は timeline / explore section へ移る。ID コピーは clipboard へ書くだけ | T3, T4 / TR-2, TR-3 |
| AC-8 | ja / en / zh-CN × dark / light × 1280 / 390px で収まり、横 scroll と clipping がない | T6 / TR-6 |
| AC-9 | ブックマーク view 初回に loading を `role="status"` で示し、取得成功・0件のときだけ案内へ移る | T3 / TR-5 |
| AC-10 | 初回 loading は表示開始から最低0.5秒続き、取得が長い場合は完了まで続く | T2b / TR-5, TR-6b |
| AC-11 | データが既にある再取得では一覧を消さず、取得完了時に差し替える（下記「AC-11 の確定」） | T3 / TR-7 |
| AC-12 | ブックマーク取得失敗時は失敗文言と「再試行」を表示し、再試行は同じ取得を1回だけ行う | T3 / TR-8 |
| INVAR-1 | 表示・CTA は外部送信・認証・同意・永続化を起こさない（follow / mute / block / bookmark API 呼出し 0） | T5, T6 / TR-2 |
| INVAR-2 | タイムライン・自分の投稿・他ユーザー投稿の空文言、1件以上あるときの一覧描画は変わらない | T3 / 既存 test |
| INVAR-3 | 空状態は `status === 'ready'` かつ0件のときだけ表示する | T3, T4 / TR-3, TR-5 |
| INVAR-4 | 最低表示は timer による表示制御のみで、取得の開始・回数・順序を変えない。unmount で timer を破棄する | T2b / TR-9 |
| INVAR-5 | `reduced motion` で追加の動きを入れない（文字の Notice のみ） | T2 / Storybook |

### AC-11 の確定

計画 v3 では「再取得で loading 通知を最低0.5秒重ねる」としていたが、ブックマークの取得は無関係な section 遷移のたびに走るため、通知を毎回重ねると一覧の上に通知が出入りして layout が動き、抑止したい点滅そのものになる。ソーシャル一覧（既存）も初回だけ loading を示す。よって「loading をフォロー中／フォロワー／ブロック中と合わせて表示する」という要求どおり、ブックマークも初回（一覧をまだ出せない間）と初回失敗後の再試行だけ loading を示し、値がある再取得は一覧を保持して完了時に差し替える。Issue の AC-11 をこの内容へ更新した。

## Surface inventory

| ID | 入口・trigger | helper / owner | 読み書き・副作用 | 条件 / transition | 証跡 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | ブックマーク view、取得成功0件 | `BookmarksListFrame` → `TimelineFeed.emptyState` → `BookmarksEmptyState` | 表示のみ | AC-1, AC-6, AC-9, INVAR-3 / TR-1, TR-5 | shell test、browser |
| INV-2 | フォロー中／フォロワー／ミュート中／ブロック中の `ready` 0件（プロフィール概要・設定「安全」の両入口を同一 group。`openProfileConnections` / `openSocialConnections` の全 caller） | `ProfileConnectionsPanel` → `ProfileConnectionsEmptyState`、`profileConnectionsEmptyGuidance` | 表示のみ | AC-2〜6, INVAR-3 / TR-1 | component test、table test、browser |
| INV-3 | 「タイムラインを見る」（ブックマーク） | 既存 `selectColumnTimelineView(column,'feed')` | Column の view と、focus 中 Column なら route | AC-7 / TR-2 | shell test、browser |
| INV-4 | 「タイムラインを見る」「見つけるで探す」（ソーシャル一覧） | 既存 `focusPrimarySection('timeline' \| 'explore')` | column / route | AC-7 / TR-2 | component test、browser |
| INV-5 | 「ユーザーIDをコピー」 | 既存 `copyTextToClipboard(localAuthorPubkey)` | clipboard 書込のみ | AC-7 / TR-2 | component test |
| INV-6 | ブックマーク初回取得（section 遷移 effect。#765 の非 active Column 経由を含む） | `loadBookmarksSection` が `bookmarksPanelState` を loading → ready / error | 既存 read 1回 | AC-9, AC-10, AC-12 / TR-5, TR-8 | shell test、browser（遅延1.5秒） |
| INV-7 | 設定「リアクション」経由の `listBookmarkedPosts` | 同じ setter で `ready` | 既存 read | AC-11 / TR-7 | loaders 差分（既存 settings test） |
| INV-8 | 投稿カードのブックマーク toggle による local 更新 | `messageReactionSocial`（status を変えない） | 既存 IPC | INVAR-2 / TR-7 | 既存 `socialGraph` test |
| INV-9 | 「再試行」 | 既存 `loadBookmarksSection` | 既存 read 1回 | AC-12 / TR-8 | shell test |

期待 inventory 差分は INV-1〜9 の追加のみ。新規 network / storage sink はない。

## 状態遷移

| ID | 事前状態 / sequence | 期待状態 | 禁止する副作用 | 検証 |
| --- | --- | --- | --- | --- |
| TR-1 | `ready` → 0件 | 見出し（既存文言）、chip 付き手順、補足、CTA を `role="status"` で表示 | 0件を故障と断定、chip の button 化 | component / browser |
| TR-2 | 案内 → CTA | 既存導線へ移る。bookmark / follow / mute / block の送信 0 | 送信・認証・同意・保存 | component（spy 0）、browser（call 記録 0） |
| TR-3 | 案内 → view / section 切替 → 戻る | view ごとの案内に変わる。非 focus Column の切替で route 不変 | route の巻き添え | shell（既存 #765 test）、browser |
| TR-4 | `loading` / `error` で0件 | 案内を出さず、loading / error のみ | false empty | component |
| TR-5 | 初回 `loading` → 0.05秒で成功0件 | 0.5秒まで loading → 案内 | 取得の重複 | hook test、component（fake timers）、shell |
| TR-6b | 初回 `loading` → 1.5秒で成功 | 完了まで loading → 案内 | 0.5秒で false empty | browser（遅延 fixture） |
| TR-7 | 値あり → 再取得 | 一覧を保持し完了時に差し替え。通知を重ねない | 一覧の消失 | loaders 差分、既存 test |
| TR-8 | 初回失敗 → 再試行 → 成功 | error（文言＋再試行）→ loading → 案内。再試行で read 1回 | 空文言の表示 | shell test |
| TR-9 | loading 中に unmount / 切替 | timer 破棄、副作用なし | 遅延 setState | hook test |
| TR-6 | 3 locale × 2 theme × 1280 / 390 | 文言・操作を保持し Column 内に収まる | 横 scroll、clipping | browser、Storybook |

## 作業・検証の結果

- T1: 失敗 test を基準 commit で記録（上記）。
- T2: `components/ui/action-ref.tsx`（非操作 chip。pill・secondary 面・太字で実ボタンに合わせ、icon は `aria-hidden`、操作名は可視文字）、`components/core/EmptyStateGuidance.tsx`（`Notice role="status"`、見出し・手順・補足・CTA）。手順文は `Trans` の自己閉じ component で chip を埋め込む。
- T2b: `lib/useMinimumLoading.ts`（`MIN_LIST_LOADING_MS = 500`）。`loading` で mount した場合だけ最低時間を保持し、`ready` / `error` で mount した panel は即座に実 status を返す。単体 test 6件（fake timers）。
- T3: `TimelineFeed.emptyState`（`null` で空文言を出さない）、`reactionsBookmarks.bookmarksPanelState`、`loadBookmarksSection` の loading / ready / error、`BookmarksEmptyState` / `BookmarksListFrame`、`DesktopShellPrimaryWorkspace` の配線（CTA は `selectColumnTimelineView`、再試行は `loadBookmarksSection`）。
- T4: `profileConnectionsEmptyGuidance.ts`（view → 手順・補足・CTA の table、test 5件）、`ProfileConnectionsEmptyState.tsx`、`ProfileConnectionsPanel` の差替えと `useMinimumLoading` 適用、`openExploreSection` の配線。
- T5: ja / en / zh-CN の `shell.json`（`workspace.bookmarksLoading`、`workspace.bookmarksEmpty.*`）、`profile.json`（`connections.emptyGuidance.*`、`connections.emptyActions.*`）、`common.json`（`errors.failedToLoadBookmarks`）。chip の操作名は既存 `common:actions.*` / `shell:report.actionLabel` を再利用。`parity.test.ts` の用語規則（author 禁止）に合わせて「ユーザー」表記にした。`DESIGN.md` 4.2 に一覧の初回 loading と chip の規則を追記。
- T6: 既存 test 4件（ブックマーク空文言の同期取得）を非同期取得へ変更。Story: `BookmarksEmptyState`（4）、`ProfileConnectionsPanel` に空状態 4 view を追加。axe（story root scope、48条件 = 8 story × 3 locale × 2 theme）は[違反0・要確認0](assets/issue-994/story-a11y.json)。browser spec `empty-state-guidance.spec.ts` 15件（6条件 × ブックマーク／ソーシャル4 view、見つける CTA、遅延1.5秒の loading、設定「安全」からのミュート一覧）。

### AC / INVAR と証跡

| 条件 | 実装・証跡 |
| --- | --- |
| AC-1 / AC-9 / AC-12 | `BookmarksEmptyState.tsx`、`DesktopShellPage.bookmarksEmptyState.test.tsx`（loading → 案内 → feed 復帰、失敗 → 再試行）。[ja dark 1280](assets/issue-994/bookmarks-ja-dark-1280.png)、[en light 390](assets/issue-994/bookmarks-en-light-390.png) |
| AC-2〜AC-5 | `ProfileConnectionsEmptyState.tsx`、`ProfileConnectionsPanel.emptyState.test.tsx`、`profileConnectionsEmptyGuidance.test.ts`。[フォロー中 ja dark](assets/issue-994/following-ja-dark-1280.png)、[フォロワー zh-CN light 390](assets/issue-994/followers-zh-CN-light-390.png)、[ミュート ja light 390](assets/issue-994/muted-ja-light-390.png)、[ブロック en dark](assets/issue-994/blocking-en-dark-1280.png) |
| AC-6 | `action-ref.tsx`。component test で `button` role 不在・`tabindex` 不在、browser で svg `aria-hidden` と button 0件 |
| AC-7 | shell test（feed tab `aria-selected`、hash に `timelineView` なし）、component test（callback 呼出し・clipboard）、browser（URL 遷移） |
| AC-8 | browser 12条件（`expectContained`、document 横 scroll なし）、Storybook 48条件 |
| AC-10 / INVAR-4 | `useMinimumLoading.test.tsx`（0.2秒成功→0.5秒まで保持、long fetch→即時、unmount で timer 0）、panel test（fake timers）、browser（遅延1.5秒で完了まで loading） |
| AC-11 | `useDesktopShellSectionLoaders.ts` の `showsProgress`。値ありの再取得は status を変えず、失敗は best effort |
| INVAR-1 | shell / component test で bookmark / follow / mute / block の spy 0、browser で記録した mutation call 0 |
| INVAR-2 / INVAR-3 | 既存 `TimelineFeed` caller 3件は `emptyState` 未指定、`TimelineFeed.emptyState.test.tsx`、panel test（loading / error で案内なし）、全体 Vitest |

### 実行結果（Linux container、Node 22.22、pnpm 10.16.1）

| 検証 | 結果 |
| --- | --- |
| `eslint` 変更 file / `tsc --noEmit` | 成功 |
| targeted Vitest（hook / feed / panel / guidance / shell / parity） | 22件 + 既存 41件 + parity 成功 |
| 全体 Vitest | 1回目は `parity.test.ts` 2件が新規文言の「author」で失敗（192 files 中 1 file）。文言を「ユーザー」へ直し、parity と panel test を再実行して成功（63件） |
| Playwright chromium `empty-state-guidance.spec.ts` | 15件成功 |
| Playwright chromium 全体 | 1回目 291件中 `orange-emphasis.spec.ts` の dark 1件が失敗（下記）。spec を直して orange / 本 spec / localization-layout / profile-density の 38件成功。修正後に全体を再実行し 291件成功。visual project（`ignoreSnapshots`、smoke）36件成功 |
| Storybook build + axe | build 成功、48条件 違反0・要確認0 |
| `cargo xtask oversized-files` / `git diff --check` | 1回目は `DesktopShellPage.tsx` が 999 → 1007 行で ratchet に抵触。section 遷移 handler 4件を 1 行ずつへ畳んで 995 行にし成功。`git diff --check` 成功 |

repo 固定の chromium build（1234）が container に無いため、Playwright は `/opt/pw-browsers/chromium` を `executablePath` に指定する未 commit の local config で実行した（#960 と同じ扱い）。

### 追加発見の分類

- Regression（merge 前に修正）: `orange-emphasis.spec.ts` の「primary filled buttons alone use orange (dark)」が本差分の build でのみ失敗した。基準 commit の build では成功する。原因は spec の race で、開いた composer の送信ボタンが click 位置の直下に現れ、pointer が乗ったまま静止色を検証していた（基準 build でも取得時点の背景は既に hover 色へ遷移中で、最初の poll が遷移前に当たるかどうかだけの差）。本差分は button の色・layout を変えないが、bundle の変化で timing が動いた。静止色の検証前に `page.mouse.move(0, 0)` で pointer を外す最小修正を同じ PR に含め、hover 色は既存の `hover()` で確認する。
- Existing-gap（同じ Issue で対応）: ブックマーク一覧の false empty（取得前・失敗時の0件表示）。loading / error 要件として AC-9 / AC-12 に固定した。
- New-requirement（別 Issue 候補、本 Issue の blocker にしない）: 自分の投稿／他ユーザーの投稿／タイムライン本体の空文言にも同型の案内を置くこと。

### 未確認の境界

- Tauri 実機（Ubuntu / Windows WebView2）での表示・focus・chip の描画は未確認。remote container のため実 App を起動できない。browser（mock API）と Storybook の成功を実機成功とは記録しない。
- Linux visual baseline は変更なし。既存 `visual.spec.ts` のブロック一覧はブロック済み1名を seed し空状態を含まないため、新規 baseline は追加していない。
- 実機での取得時間分布は未計測。最低0.5秒は DESIGN 4.2 に固定した値であり、mock の即時応答と1.5秒遅延で挙動を確認した。
