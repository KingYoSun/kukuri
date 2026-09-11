# Issue #964: 投稿作成の Esc / Tab focus / ショートカット案内

- Issue: [#964](https://github.com/KingYoSun/kukuri/issues/964)
- Scope revision: `2026-09-10-first-look-keyboard-guidance-v2`（Issue 本文で固定。実装計画は 2026-09-11 に承認）
- 基準 commit: `f2cdb5cacf02218b34291a35754b55a15c2f564e`（v0.2.1-preview.1、観測環境）。実装は main 先端 `dabf4d0` 以降に対して行う
- リスク区分: B
- UI 変更分類: 不具合修正（Esc / focus 可視性）+ 既存画面の改善（案内）
- 状態: 実装・browser 検証完了。Linux .deb 実機観測（AC-1 / AC-6 の実機欄）は未実施

工程: [Issue lifecycle](../runbooks/issue-lifecycle.md)、[PLANS.md](../../PLANS.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[DESIGN.md](../../DESIGN.md)。

## 修正前の切り分け（AC-1）

初見報告のキーごとに、変更前の実装から原因を固定した。「未対応」と「動作不良」を区別する。

| キー | focus 対象 | 変更前の実装 | 分類 | 変更後 |
| --- | --- | --- | --- | --- |
| Esc | 投稿作成の本文（textarea） | `useMentionAutocomplete` が候補表示中だけ消費。global cascade（`useDesktopShellRouting.ts` の Escape handler、#765）は editable 要素を明示的に無視するため何も起きない | 未実装（投稿作成を閉じる経路は「閉じる」ボタンのみ） | 投稿作成を閉じ、下書きを保持し、開始元へ focus を戻す |
| Esc | 投稿作成内の非 editable control（閉じる、ファイルを選択、成人向け申告、送信） | global cascade が動き、設定 / author pane / thread pane を閉じる。投稿作成は閉じない | 動作不良（意図しない対象が閉じる） | 投稿作成側で消費し、pane は閉じない |
| Esc | IME 変換中 | 判定なし | 未確定 | `isComposing` 中は投稿作成を閉じない |
| Tab / Shift+Tab | 投稿作成内 | `base.css` の `:focus-visible { outline: 2px solid var(--ring) }` と Textarea の `focus-visible:ring-2`。dark の `--ring` は alpha 0.45 で、主要背景に対する contrast が 2.1〜2.4:1（3:1 未満） | 動作不良（focus 表示はあるが視認性が不足） | dark の `--ring` を alpha 0.8 へ変更（主要背景で 3.7〜4.8:1）。contrast test に dark ring を追加 |
| Ctrl+Enter | 本文、本文か添付あり | `ComposerPanel.tsx` の handler（#819）で `requestSubmit()` | 実装済み | 変更なし（IME guard のみ追加） |
| Ctrl+Enter | 本文、本文も添付も空 | `useDesktopShellActions.ts` の submit が無言で早期 return。送信ボタンも同じ | 動作不良に見える（理由が表示されない） | composer error に理由を表示 |
| Ctrl+Enter | IME 変換中 | 判定なし | 未確定 | `isComposing` 中は送信しない |
| Ctrl+Enter | pending / disabled | `submitDisabled` で送信しない（既存 test） | 実装済み | 変更なし |
| `?`、Ctrl+/、Ctrl+K、Ctrl+N | 任意 | handler が存在しない（`ctrlKey` を見る箇所は ComposerPanel のみ） | 未対応 | 追加しない。案内に「割り当てなし」と明記 |

Linux .deb（Debian 13、日本語、WebKitGTK）での再観測は本記録の「実機観測」欄に記入する。browser（Chromium）の結果で実機観測を置き換えない。

## 受入条件と維持する契約

| ID | 実装 | test / evidence |
| --- | --- | --- |
| AC-1 | 上表の切り分け | 本記録。実機欄は未記入 |
| AC-2 | `ColumnComposerFooter.tsx`: `.shell-column-composer` の keydown で Escape を消費（`defaultPrevented` / `isComposing` / 設定 drawer 表示中は除外）、`expanded=false`、折りたたみボタンへ focus 復元。閉じるボタンも同じ復元 | `ColumnComposerFooter.test.tsx` "keyboard dismissal (#964)" 5 件、`DesktopShellPage.escapeGuard.test.tsx` "closes only the composer and keeps the thread selection"、browser `composer-keyboard.spec.ts` |
| AC-3 | `tokens.css` dark `--ring` alpha 0.45 → 0.8、`DESIGN.md` token 表を同期 | `contrast.test.ts` "dark theme focus ring contrast (Issue #964)" 5 背景 + light に 2 背景追加、browser で Tab 巡回 6 control の `:focus-visible` と描画（outline / box-shadow）を ja・en × dark・light で確認 |
| AC-4 | 設定 drawer に section `keyboard`（`KeyboardPanel.tsx`）。Control Center の「システム」にボタン。投稿作成 heading 下に hint 文と案内 link。URL `?settings=keyboard` | `KeyboardPanel.test.tsx`、`DesktopShellPage.shellChrome.test.tsx` "keyboard guidance opens from the Control Center and from the composer hint"、browser "keyboard guidance is reachable ... by pointer and keyboard" |
| AC-5 | 案内の各行は実装済み handler に対応（下表）。未対応キーは `unassigned` で明示。Ctrl+Enter に `isComposing` guard、空下書きは理由表示 | `ComposerPanel.test.tsx` "Ctrl+Enter submits ... exactly once" / "during IME composition does not submit" / "disabled submit blocks"、`useDesktopShellActions.test.tsx` "empty Column Draft submission shows a reason" / "pending Column Draft rejects"、browser "Ctrl+Enter sends once, empty send shows a reason" |
| AC-6 | ja / en / zh-CN の文言、browser の ja・en 検証 | `i18n/parity.test.ts`、browser spec、Storybook `Settings/KeyboardPanel`。実機は未実施 |
| INVAR-1 | Enter / Shift+Enter の改行、メンション候補、IME を変更しない | `ComposerPanel.test.tsx` 既存 test、`ColumnComposerFooter.test.tsx` "Escape while mention suggestions are open only closes the suggestions" |
| INVAR-2 | Esc と案内の開閉で送信しない。下書き・返信先・投稿先は store に残る | 上記 footer / shellChrome / browser test の draft 保持 assertion |
| INVAR-3 | 案内を開く操作は link / ボタンのみ（新しいキー割り当てなし）。閉じる / 送信の pointer 操作は維持 | `ColumnComposerFooter.test.tsx` "shows the keyboard hint and opens the keyboard guidance"、既存 Close / Post ボタン test |

### 案内と実動作の対応（AC-5）

| 案内の行 | 実装 | test |
| --- | --- | --- |
| 投稿作成 Esc | `ColumnComposerFooter.onComposerKeyDown` | footer test、escapeGuard test |
| 投稿作成 Ctrl+Enter | `ComposerPanel.onComposerKeyDown` + `handleSubmitColumnDraft` | ComposerPanel test、actions test |
| 投稿作成 Tab / Shift+Tab | DOM 順（閉じる、案内 link、本文、ファイルを選択、成人向け申告、送信） | browser spec の Tab 巡回 |
| メンション候補 ↑ ↓ / Enter / Tab | `useMentionAutocomplete.onKeyDown` | ComposerPanel test（既存） |
| 画面 Esc | `useDesktopShellRouting`、`DesktopShellControlCenter`、`ColumnMenu`、`context-action-menu`、Radix Dialog | escapeGuard / shellChrome / ColumnMenu test（既存） |
| 先頭 Tab のスキップリンク | `DesktopShellPage.tsx` の `.shell-skip-link` | 既存 |
| カラムメニュー ↑ ↓ Home End Esc | `ColumnMenu.tsx` | `ColumnMenu.test.tsx`（既存） |
| タイムライン表示 ← → | `TimelineViewIconTabs.tsx` | shellChrome test（既存） |
| 投稿 Shift+F10 | `context-action-menu.tsx` | `PostCard.test.tsx` / ComposerPanel test（既存） |
| メディア ← → | `MediaViewerDialog.tsx` | 既存 |
| 割り当てなし（?、Ctrl+/、Ctrl+K、Ctrl+N） | handler なし | `rg ctrlKey` で ComposerPanel 以外に存在しないことを確認 |

## 固定 surface inventory

| ID | 入口・trigger | shared helper | 読み書き・副作用 | guard / invariant | transition | test |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1 | 折りたたみボタン（timeline post / thread reply / conversation message の 3 target） | `ColumnComposerFooter` → `setColumnDraft` | `expanded=true` | 既存 | TR-1, TR-9 | footer test |
| INV-2 | 閉じるボタン | 同上 | `expanded=false` + focus 復元 | INVAR-2 | TR-1 | footer test |
| INV-3 | 投稿作成内 Escape keydown（新規） | 同上 | `expanded=false` + focus 復元 + `preventDefault` | `defaultPrevented`、`isComposing`、`settingsOpen` で除外 | TR-1〜5 | footer / escapeGuard / browser |
| INV-4 | global Escape cascade（変更なし） | `useDesktopShellRouting` | 設定 / author / thread pane を閉じる | `defaultPrevented` と editable guard（既存） | TR-4 | escapeGuard test |
| INV-5 | Ctrl+Enter と form submit | `ComposerPanel.onComposerKeyDown` → `handleSubmitColumnDraft`（caller は footer のみ） | 送信 / pending / error | `submitDisabled`、`isComposing`、空下書き | TR-6 | ComposerPanel / actions / browser |
| INV-6 | 案内の入口（Control Center、設定 nav、composer hint link、URL `?settings=keyboard`） | `handleOpenSettingsSection` | 設定 drawer 表示、route 更新 | INVAR-2, 3 | TR-8 | shellChrome / browser |
| INV-7 | `columnDraftPersistence` の再起動復元（変更なし） | — | `expanded` / `content` の復元 | 復元時は focus を奪わない（`restoreFocusRef`） | TR-9 | 既存 persistence test |

追加・削除: INV-3 と INV-6 の入口を追加。sink（送信 IPC、永続化）の追加・削除は 0。

## 状態遷移

| ID | 事前状態 | event | 期待状態 | 禁止する副作用 | test |
| --- | --- | --- | --- | --- | --- |
| TR-1 | 展開、本文 focus、候補なし | Esc | 折りたたみ、下書き保持、折りたたみボタン focus | 送信、pane close | footer / browser |
| TR-2 | 候補表示中 | Esc → Esc | 候補だけ閉じる → 折りたたみ | 送信 | footer |
| TR-3 | IME 変換中 | Esc / Ctrl+Enter | 変化なし / 送信なし | 折りたたみ、送信 | footer / ComposerPanel |
| TR-4 | 閉じる等の非 editable control に focus | Esc | 折りたたみ、thread / author pane は残る | pane close | footer / escapeGuard |
| TR-5 | 設定 drawer 表示中（hint link から開いた直後） | Esc | drawer が閉じ、下書きは残る | 折りたたみ | shellChrome / browser |
| TR-6 | 本文あり / 空 / pending | Ctrl+Enter | 1 回送信 / 理由表示 / 送信なし | 二重送信、無言 | ComposerPanel / actions / browser |
| TR-7 | 展開 | Tab / Shift+Tab | 6 control を順に focus、ring 可視 | focus trap | browser |
| TR-8 | 任意 | 案内を開く / 閉じる | drawer 表示 / 非表示、下書き・展開状態不変 | 送信、下書き消失 | shellChrome / browser |
| TR-9 | 再起動で `expanded=true` 復元 | Esc | TR-1 と同じ。復元時に focus を奪わない | — | footer（`restoreFocusRef`）、既存 persistence test |

## 実機観測（Linux .deb、日本語）

未実施。実施時は次を記録する: OS / WebKitGTK 版、theme、IME の有無、キーごとの focus 対象と結果、案内の発見経路と開閉、戻り focus、screenshot。browser と差があれば Existing-gap として同じ Issue に戻す。

## 追加発見と分類

- 投稿作成内の非 editable control から Esc で thread / author pane が閉じる: Existing-gap（AC-2）として本 PR で修正。
- hint link から案内を開いた直後の Esc が composer 側で消費される: 実装中の Regression として本 PR で修正（drawer 表示中は composer の Esc を無効化）。
- 空下書きの無言 return: AC-5 の「送信可能な状態」に関わるため本 PR で理由表示に変更。
- 設定 drawer を閉じたときの戻り focus は既存契約どおり Control Center trigger（hint link には戻らない）。Optional-hardening として追加起票しない。

## validation

- `apps/desktop`: `eslint . --max-warnings 0`、`tsc --noEmit`、`vitest run`（全件）、`storybook build`、`playwright test --project=chromium`（全 spec。pre-installed Chromium を executablePath で指定）。結果は PR 本文を参照。
- 視覚回帰: composer hint、設定 nav の section 追加、Control Center のボタン追加、`--ring` 変更で baseline が変わるため、`Kukuri Visual Baseline` workflow で Linux baseline を再生成して同梱する。
- Rust / Tauri: 変更なし（frontend のみ）。
