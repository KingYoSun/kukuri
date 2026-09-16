# 2026-09-16 Column 内操作による active 化と route 同期

- Status: current
- Supersedes: None
- Superseded by: None
- PR: 本 record を追加した PR（Issue #1053）
- Issue / Scope revision: [#1053](https://github.com/KingYoSun/kukuri/issues/1053)、2026-09-16（INVAR-2 改訂）
- Preview: 見た目の変更は無く、操作結果の変化のみ。下表の変更前後の観測（Playwright の実 pointer 操作）を証跡とする。
- 対象 surface / 利用者 / 目的: Column Canvas 上の全 Column を使う desktop 利用者。別 Column のボタン・プルダウンを 1 回押せば、その操作が 1 回実行され、押した Column が active になる。
- 変更分類: 不具合修正と、header 操作の active 化規則の変更（ユーザー判断、ADR 0031 §11 に反映）。

## 採用した挙動

- Column 内の操作（本文・header のボタン、Timeline の topic 選択、Explore の検索元ノード選択を含む）は、その Column を active にし、URL をその Column の canonical target へ同期する。従来の `data-column-preserve-activation` による「header 操作では active を移さない」規則は廃止した。Metaverse の gesture 領域（`data-column-gesture-owner`）は従来どおり対象外。
- 操作対象（button 等）で active 化した場合、route の同期だけを行い、文脈の読み込みは押された handler に任せる。本文の非操作領域をクリックした場合は従来どおり読み込みも行う。
- pointer 押下で active 化した Column の `scrollIntoView` は押下の終了後（click 配送の後）に行う。押下中に canvas が動いて click が別要素へ落ちることを防ぐ。
- Timeline の topic 切替は Column id を変えずに scope だけを変えるため、route の scope に一致する Timeline の検索は id ではなく scope で行い、重複 Column を開かない。
- 後から操作した Column の push が未 commit の間に古い push が commit されても、古い URL を再投影して active を奪わない。
- 背景の通知 Column の更新ボタンは通知 Column を active にするため、本文クリックと同じく既読化される。

## 条件と証跡

- Platform: Chromium（Playwright、deterministic mock）と jsdom（Vitest）。Windows WebView2 / Tauri 実機は未確認（下記）。
- Viewport: 1280×800（Playwright `column-activation.spec.ts`）、900×760 / 900×900（既存 `shell.smoke` / `column-immersive`）。
- Theme / Locale: dark 既定 / en（操作結果のみを観測し、見た目は変更していない）。
- State: 既定 layout、Timeline の topic 切替後、private channel scope の Timeline 併置、DM の会話 Column 表示中、Timeline の push 未 commit 中。

| 条件 | 変更前 | 変更後 |
| --- | --- | --- |
| topic 切替後の Timeline 選択 → Explore 本文の「Discover」押下（Chromium） | Timeline Column が 2 本になり、「Discover」は選択されない | Column 数は 5 のまま、「Discover」が選択され、Explore が active、URL は `#/explore?...` |
| 部分表示の Column のボタン押下（Chromium、イベント記録） | `pointerdown` 対象は Discover、canvas が 152px scroll し `pointerup` は Recommendations、`click` は共通親の tablist へ配送 | `pointerdown` / `pointerup` / `click` とも Discover、その後に scroll |
| Timeline 選択 → 通知 header の更新（Chromium） | 通知 Column は Inactive のまま | 通知 Column が active、URL は `#/notifications?...` |
| 既定 topic で Timeline 選択 → Explore 本文のボタン（jsdom） | Explore は active だが URL は `#/timeline?...` のまま | URL も `#/explore?...` |
| private channel Column 選択 → Explore 本文のボタン（jsdom） | private Column が末尾へ移動して active を奪う | Column 順は不変、Explore が active |
| 非 active Timeline の topic 選択（jsdom / Chromium） | Profile が active のまま、URL も Profile | Timeline が active、URL は切替後 topic |

## Accessibility・性能・未確認事項

keyboard で別 Column の操作対象へ focus を移した場合も、その Column が active になり URL が push される（履歴が増えることはユーザー判断で許容）。focus 自体は移動せず、押した control に残る（profile 更新ボタンの focus 維持 test で確認）。

追加の polling や API 呼び出しは無い。操作対象での active 化は route 同期のみで、読み込みは増やさない。

未確認: Windows WebView2 / Tauri 実機での再現手順の解消、物理タッチ・ペン入力、screen reader の読み上げ。WebView2 は Chromium 系のため Playwright の実 pointer 観測を主証跡とするが、実機確認は Issue の Close 条件として別途記録する。

## Validation

- Vitest: `DesktopShellPage.columnActivation.test.tsx`（新規 4 件）、`DesktopShellPage.columnScope.test.tsx`（channel scope 1 件追加）、`ColumnCanvas.test.tsx`（押下後 scroll 1 件追加、プルダウン active 化へ書換）、`useRouteSynchronization.test.tsx`（古い push の再投影 1 件追加）、`workspaceState.test.ts`（scope 解決 2 件追加）。修正前に新規・追加 test が失敗することを確認済み。
- 旧仕様を固定していた test を新仕様へ書換: `ColumnCanvas.test.tsx`、`DesktopShellPage.topics.test.tsx`、`DesktopShellPage.messages.test.tsx`、`DesktopShellPage.notifications.test.tsx`、`DesktopShellPage.timelineView.test.tsx`（2 件）、`DesktopShellPage.profileRefresh.test.tsx`、`tests/playwright/shell.smoke.spec.ts`。
- Playwright: `column-activation.spec.ts`（新規 2 件、修正前は失敗）と Column / routing 関連 spec。

## Review result

- 一貫性: 本文・header・選択 control のどれを操作しても同じ規則で active と URL が決まる。
- エラー防止: 操作が別要素へずれて失われない。重複 Timeline を開かない。
- 主導権: 押した Column が focus 対象になり、戻る操作で直前の Column へ戻れる。

## Exceptions

None。必要な確認の未実施は「未確認事項」に記載した。
