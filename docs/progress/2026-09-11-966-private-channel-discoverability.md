# Issue #966: 通常 GUI からのプライベートチャンネル発見性（作成・招待参加・招待共有の入口）

- Issue: [#966](https://github.com/KingYoSun/kukuri/issues/966) / PR: [#982](https://github.com/KingYoSun/kukuri/pull/982)
- Scope revision: `2026-09-10-first-look-private-channel-discoverability-v1`（Issue 本文で固定。実装計画は 2026-09-11 に承認。未決 4 点は推奨案で確定: 入口は Timeline Column header、AC-1 の実機固定は browser 切り分け表＋実機欄未実施、dev.md の旧 UI 名称は別 Issue、「選択中のチャンネルを共有」は disabled のまま理由を表示）
- 基準 commit: `f2cdb5cacf02218b34291a35754b55a15c2f564e`（v0.2.1-preview.1、観測環境）。実装は main 先端 `c21ee62` 以降に対して行う。基準 commit と main 先端の間にチャンネル導線の差分はない（Control Center の接続表示・キーボード案内の追加のみ）
- リスク区分: B（既存 GUI の開始導線の改善。audience・権限・秘密値の取扱いは変更しない）
- UI 変更分類: 既存画面の改善（Control Center、ダイアログ文言）＋小さな新導線（Timeline Column header の入口）
- 状態: 実装・browser 検証完了。Linux .deb 実機観測（AC-1 の実機欄）は未実施

工程: [Issue lifecycle](../runbooks/issue-lifecycle.md)、[PLANS.md](../../PLANS.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[ADR 0018](../adr/0018-channel-first-sidebar-and-unified-epoch-lifecycle.md)、[DESIGN.md](../../DESIGN.md)、[UI 実装配置](../architecture/desktop-ui-implementation.md)。

## 変更前の切り分け（AC-1、browser mock）

2026-09-11、main 先端 `c21ee62` の browser mock（Chromium、ja、dark、開発者モード OFF、1280×800 / 390×844）で固定した。実装有無・UI 未表示・入口の見落とし・条件不足を区別する。

| 開始画面 | 状態 | 変更前の観測 | 分類 |
| --- | --- | --- | --- |
| Timeline Column（公開 scope） | 未参加 | header は「公開 · general」とトピック切替 Select のみ。投稿作成 footer は「投稿」ボタンのみ。画面全体に「プライベート」の語がない（`body.innerText` に不在） | 入口未表示（実装はあるが通常画面に入口がない） |
| Timeline Column（channel scope） | 参加済み | header は「友人 · general」のみ。設定・共有への操作なし | 入口未表示 |
| Control Center →「場所」 | 未参加 | 「チャンネルを作成または参加」（機能名に「プライベート」がなく説明なし）、「選択中のチャンネルを共有」は disabled で理由なし。各 topic の scope group は「公開 / 公開フィード」だけで、チャンネル group 自体が描画されない（`TopicNavList.tsx` の `hasChannels` 判定） | 入口の見落とし＋条件不足の未説明 |
| 作成・参加ダイアログ | 未参加 | 「作成」（チャンネル名、公開範囲 Select）と「参加」（トークン貼り付け）のみ。機能説明、公開範囲の説明、招待の入手方法の案内がない | 説明不足 |
| 参加済みチャンネル → 共有 | 参加済み | Control Center → チャンネル行の歯車 → 設定ダイアログ「共有リンク作成」で到達可能。通常画面からは 2 段先 | 到達可（Control Center 経由のみ） |
| 開発者モード | OFF | チャンネル導線は開発者モードで gate されていない（stream / metaverse column と設定のみ） | 条件不足ではない |

変更前画像: [Timeline 1280](assets/issue-966/before-timeline-ja-1280.png)、[Timeline 390](assets/issue-966/before-timeline-ja-390.png)、[Control Center 場所 1280](assets/issue-966/before-control-center-places-ja-1280.png)、[作成・参加ダイアログ 1280](assets/issue-966/before-dialog-ja-1280.png)、[作成後の Column header](assets/issue-966/before-channel-scope-ja-1280.png)、[設定ダイアログ](assets/issue-966/before-settings-ja-1280.png)。

Linux .deb（Debian 13、日本語、WebKitGTK）での再観測は本記録の「実機観測」欄に記入する。browser（Chromium）の結果で実機観測を置き換えない。

## 変更

- Timeline Column header の scope 行に入口を追加（`ColumnSurface.tsx` に `scopeActions` slot、`DesktopShellColumnWorkspace.tsx` の `renderScopeActions`）。公開 scope では「プライベートチャンネル」→ 既存の作成・参加ダイアログ、channel scope では「設定と共有」→ 既存の設定ダイアログ。非 active Column から押した場合は先に Column を activate する。
- 作成・参加ダイアログ（`PrivateChannelPanel.tsx`）に機能説明、作成の結果（オーナーになり招待を作成できる）、選択中の公開範囲の説明（`aria-describedby`）、招待の入手方法、同 topic の参加済み一覧（「開く」「設定と共有」）を追加。既存 handler と form は変更しない。
- Control Center「場所」（`DesktopShellControlCenter.tsx`、`TopicNavList.tsx`）: ボタン文言を「プライベートチャンネルを作成または参加」に変更。参加 0 件の topic に「チャンネル」group の空状態と「作成または参加」導線を追加（押した topic を選択してからダイアログを開く）。「選択中のチャンネルを共有」の disabled 時に理由を `aria-describedby` で添える。
- 設定ダイアログ（`PrivateChannelSettingsPanel`）: 相互フォロー限定で非オーナーの場合は共有ボタンを disabled にして理由（オーナーだけが作成できる）を表示。app-api の `export_friend_only_grant` の guard と同じ条件で、権限判定の正本は runtime のまま。pending 中は理由を `role="status"` で表示。`rotation_required` に次の行動（共有リンク作成で条件を満たす参加者だけへ配布、フォロー関係はプロフィールで確認）を追加。共有リンクの取扱い注意を追加。
- 作成・参加ダイアログと設定ダイアログの閉じたときの focus 復元を明示（`DesktopShellOverlays.tsx`）。Radix の既定復元はこの shell では body へ落ちるため、`TesterFeedbackDialog` と同じ方式で開いた要素へ戻す。
- i18n ja / en / zh-CN（`channels.json`、`shell.json`）、`mvp-user-quickstart.md` 手順 6 に入口を追記。
- 共通の設定ダイアログ入口 `openChannelSettingsDialog`（`DesktopShellPage.tsx`）を Control Center・Column header・ダイアログ内一覧が共用する。

## 受入条件と維持する契約

| ID | 実装 | test / evidence |
| --- | --- | --- |
| AC-1 | 上表の切り分けと変更前画像 | 本記録。実機欄は未記入 |
| AC-2 | Timeline Column header の入口、ダイアログの説明、Control Center の空状態・文言 | `DesktopShellPage.privateChannelEntry.test.tsx` "Timeline Column header opens the create/join dialog with guidance…" / "Control Center places explain the empty channel state…"、`PrivateChannelPanel.test.tsx` "explains the feature, the selected audience, and how to obtain an invite"、`TopicNavList.test.tsx` "shows the empty channel state…"、browser `private-channel-discoverability.spec.ts`（ja / en × dark / light × 1280 / 390） |
| AC-3 | channel scope Column の「設定と共有」、ダイアログ内一覧の「設定と共有」、設定ダイアログの理由表示 | `privateChannelEntry.test.tsx` "channel scope Column exposes settings & sharing…" / "create/join dialog lists joined channels and hands off to their settings"、`PrivateChannelPanel.test.tsx` "blocks mutuals share for non-owners with a reason" / "keeps invite-only sharing available to participants" / "shows a pending reason and the rotation next step"、browser spec の共有リンク作成 |
| AC-4 | 通常 GUI だけで到達する browser spec（実 pointer と Tab / Enter / Escape）、ja 画像 | `private-channel-discoverability.spec.ts` "reachable by keyboard and returns focus on Escape"、本記録の画像。実機は未実施 |
| INVAR-1 | 公開範囲・相互フォロー条件・招待の有効性判定は変更なし（Rust 差分 0）。client の disabled は runtime の guard と同条件 | `git diff` に `crates/` の差分なし。既存 `channels.test.tsx` 5 件 |
| INVAR-2 | 入口・説明を開くだけでは `createPrivateChannel` / `exportChannelAccessToken` / `importChannelAccessToken` を呼ばない。秘密値の追加露出なし（token は既存の Notice 内のみ） | `privateChannelEntry.test.tsx` の spy 0 回、作成時 1 回・共有時 1 回の既存回数維持 |
| INVAR-3 | Escape / 閉じるで URL・topic・scope が不変。focus は開いた要素へ戻る | `privateChannelEntry.test.tsx`（hash 不変、"Public · general" 維持）、browser spec（URL 完全一致、focus 復元）、既存 `escapeGuard.test.tsx` |

## 固定 surface inventory

| ID | 入口・trigger | shared helper | 読み書き・副作用 | guard / invariant | transition | test |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1 | Control Center「プライベートチャンネルを作成または参加」（既存） | `setChannelDialogOpen(true)` | dialog 表示のみ | なし | TR-1 | shellChrome test |
| INV-2 | Timeline Column header「プライベートチャンネル」/「設定と共有」（新規） | `setChannelDialogOpen` / `openChannelSettingsDialog`。非 active なら `activateColumn` + `activateWorkspaceColumn` | dialog 表示、Column activate | channel scope 判定 | TR-1, TR-3 | privateChannelEntry test、browser |
| INV-3 | 「場所」の空状態「作成または参加」（新規） | `handleSelectTopic(topic)` + `setChannelDialogOpen(true)` | topic 選択（既存 flow）+ dialog 表示 | なし | TR-6 | privateChannelEntry test、TopicNavList test、browser |
| INV-4 | 作成 / 参加 form submit（既存、変更なし） | `handleCreatePrivateChannel` / `handleJoinChannelAccess` → `createPrivateChannel` / `exportChannelAccessToken` / `importChannelAccessToken` | 既存 IPC | 空入力 guard（既存） | TR-4 | channels.test 既存 |
| INV-5 | ダイアログ内一覧「開く」「設定と共有」（新規） | `handleSelectPrivateChannel` / `openChannelSettingsDialog` | scope 選択、dialog 切替 | なし | TR-3 | privateChannelEntry test、PrivateChannelPanel test |
| INV-6 | 設定ダイアログ「共有リンク作成」（既存 handler、disabled 条件を追加） | `handleShareChannelAccess` → `exportChannelAccessToken` | 既存 IPC | `activePrivateChannel`、pending、friend_only は owner のみ（runtime と同条件） | TR-5 | PrivateChannelPanel test、channels.test 既存 |
| INV-7 | 閉じる / Escape（既存） | Radix Dialog + `restoreReturnFocus` | focus 復元のみ | editable guard（既存） | TR-2 | escapeGuard test、browser |

追加・削除: INV-2、INV-3、INV-5 の入口を追加。sink（IPC、永続化、外部送信）の追加・削除は 0。`openChannelSettingsDialog` の caller は Control Center、Column header、Overlays の 3 箇所（`rg openChannelSettingsDialog`）。

## 状態遷移

| ID | 事前状態 | event | 期待状態 | 禁止する副作用 | test |
| --- | --- | --- | --- | --- | --- |
| TR-1 | 未参加、公開 scope、開発者モード OFF | 入口を押す | 作成・参加ダイアログと説明が表示 | API 呼出、scope 変更 | privateChannelEntry / browser |
| TR-2 | ダイアログ表示中 | Escape / 閉じる | 閉じて URL・topic・scope 不変、focus は入口へ | 下書き消失、scope 変更 | privateChannelEntry / browser |
| TR-3 | 参加済み channel scope | header「設定と共有」/ 一覧「設定と共有」 | 設定ダイアログが該当チャンネルで開く | 別チャンネルへの切替、共有発行 | privateChannelEntry / browser |
| TR-4 | ダイアログ表示中 | 作成 / 参加 submit（既存） | 既存どおり channel 選択と URL 更新 | 二重呼出 | channels.test 既存 |
| TR-5 | 設定ダイアログ、pending / rotation_required / 非オーナー friend_only | 表示 | 理由と次の行動が読め、共有ボタンの条件は runtime と同じ | 権限迂回 | PrivateChannelPanel test |
| TR-6 | 未参加 topic | Control Center「場所」を展開 →「作成または参加」 | 空状態が表示され、その topic を選択してダイアログが開く | なし | privateChannelEntry / browser |
| TR-7 | ja / en × dark / light × 1280 / 390 | 表示 | 文言と操作を保持し、document の横 scroll なし、ダイアログが viewport 内 | clipping | browser |

## 実機観測（Linux .deb、日本語）

未実施。実施時は次を記録する: OS / WebKitGTK 版、theme、開始画面（Timeline Column / Control Center）、表示条件、クリック順、到達／未到達箇所、screenshot。browser と差があれば Existing-gap として同じ Issue に戻す。

## 追加発見と分類

- 作成・参加ダイアログと設定ダイアログを閉じたときに focus が body へ落ちる（Radix 既定の復元が効かない）: AC-4 の keyboard 到達に関わるため本 PR で修正（Existing-gap）。他の Dialog の focus 復元は対象外（Optional-hardening、起票しない）。
- `dev.md` の「Private channel manual verification」が ADR 0018 以前の UI 名称（`Create Invite`、`Join via Invite`、`View Scope`）のまま: 区分 A の文書修正として別 Issue（New-requirement）。
- 未使用 i18n key（`channels:title`、`channels:inspectHint`、`channels:selectChannel`、`navigation.channelCreateJoin`）: 挙動に影響しないため本 PR では触らない（Optional-hardening）。`channels:empty` は Control Center の空状態には使わず、`navigation.noChannels` を新設した（既存 key の文言が「このトピックには…」で場所に依存するため）。

## 証跡画像（browser mock）

- [Timeline ja dark 1280](assets/issue-966/after-timeline-ja-dark-1280.png)、[Timeline ja dark 390](assets/issue-966/after-timeline-ja-dark-390.png)、[Timeline en light 390](assets/issue-966/after-timeline-en-light-390.png)
- [作成・参加ダイアログ ja dark 1280](assets/issue-966/after-dialog-ja-dark-1280.png)、[同 390（全体）](assets/issue-966/after-dialog-ja-dark-390.png)、[同 en light 390](assets/issue-966/after-dialog-en-light-390.png)、[作成後（参加済み一覧）](assets/issue-966/after-dialog-created-ja-dark-1280.png)
- [Control Center 場所 ja dark 1280](assets/issue-966/after-control-center-places-ja-dark-1280.png)、[同 390](assets/issue-966/after-control-center-places-ja-dark-390.png)、[同 en light 390](assets/issue-966/after-control-center-places-en-light-390.png)
- [channel scope の Column header](assets/issue-966/after-channel-scope-ja-dark-1280.png)、[設定ダイアログと共有リンク](assets/issue-966/after-settings-ja-dark-1280.png)

## validation

| 検証 | 結果 |
| --- | --- |
| `tsc --noEmit` / `eslint . --max-warnings 0` | 成功 |
| 新規 Vitest（`PrivateChannelPanel.test.tsx` 6 件、`TopicNavList.test.tsx` 追加 2 件、`DesktopShellPage.privateChannelEntry.test.tsx` 4 件）と既存 `i18n/parity.test.ts`、`styles/css-vars.test.ts` | 成功 |
| `vitest run`（全件） | 1458 passed / 9 failed。9 件はすべて shell-integration の 5 秒・10 秒 timeout。同じ 7 ファイルを `--testTimeout=30000` で再実行すると 48 件成功、残る `profile overview aggregates…`（test 固有の `10_000`）も一時的に `60_000` にすると成功。#961 の記録と同じくこの環境の遅さと判断し、CI で最終確認する |
| Playwright browser（`private-channel-discoverability.spec.ts` 9 件 + `extended-flow` / `column-scope` / `hash-routing` / `community-index`） | 24 passed。環境の Chromium build が pin と異なるため、`PLAYWRIGHT_BROWSERS_PATH` に同 build への symlink を置いて実行 |
| Playwright browser（全 spec） | 下記 |
| Storybook build | 下記 |
| 視覚回帰 | ローカル `CI=1` 比較は 21 passed / 6 failed。失敗は ja / zh の glyph（`fonts-noto-cjk` なし）と `settings-notifications-wide-dark`（#964 の設定 nav 追加以降 baseline 未更新）で、本差分の layout 退行ではない。Timeline Column header と Control Center「場所」の変更で baseline が変わるため、`Kukuri Visual Baseline` workflow（`commit_to_branch=true`）で Linux baseline を再生成して同梱する。この環境は artifact の download 先（Azure blob）へ到達できないため、workflow に branch へ commit する option を追加した。再生成（run 34599715779、commit `8873318`）で更新されたのは `control-center-ja-dark` / `explore-long-policies-ja-dark` / `settings-notifications-wide-dark` の 3 枚。`timeline-*` は入口ボタンの差分が `maxDiffPixelRatio: 0.01` 内のため更新されない |
| Rust / Tauri | 変更なし（frontend と docs のみ） |
| `cargo xtask oversized-files`（ratchet gate） | 初回 push で `DesktopShellPage.tsx`（996 → 1019 行）と `shell-scoped-overrides.css`（999 → 1068 行）が 1000 行を新たに超え CI が fail。baseline は上げず、入口 handler を `page/usePrivateChannelEntries.ts` へ切り出し（995 行）、新規 CSS を `shell-phase1-part4.css` へ移して（999 行）解消。移した CSS は新規 selector のみで cascade 上の依存はない |
