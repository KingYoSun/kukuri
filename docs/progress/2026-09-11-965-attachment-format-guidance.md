# #965 添付の対応形式案内と非対応ファイルの理由表示

- Issue: [#965](https://github.com/KingYoSun/kukuri/issues/965)
- 判定: In progress（実装・ローカル検証完了。CI／merge の最終結果は Issue の Current status と PR を参照する）
- Scope revision: `965-r1 / 2026-09-11`（調査結果と推奨案をユーザーが承認。未決 4 点は推奨案で確定: 拡張子は列挙しない、Linux に「すべてのファイル」フィルタを追加、設定内ヘルプ画面は対象外、Windows 実機は browser test で代替し未確認と記録）
- 基準 commit: `704c72c`（main。観測環境は v0.2.1-preview.1）
- リスク区分: B。composer の表示と失敗文言、Linux native chooser のフィルタのみ。添付の判定条件（`image/*` / `video/*`）、送信、読取、永続化、認証、同意は変更しない
- UI 変更分類: 既存画面の改善（文言・state 追加）。利用者は投稿・返信・DM にファイルを付ける利用者。単一目的は、選ぶ前に対応形式が分かり、非対応ファイルを選んだときに理由が同じ場所に出ること
- 対象外: 非画像・非動画ファイルの添付対応、backend の mime 検証追加、Windows の OS ダイアログ表示自体の変更、設定内のヘルプ画面、動画 codec の列挙
- 正本: [Issue 運用手順](../runbooks/issue-lifecycle.md)、[PLANS.md](../../PLANS.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[ADR 0003](../adr/0003-image-post-data-classification.md)、[ADR 0004](../adr/0004-video-post-data-classification.md)、[DESIGN.md](../../DESIGN.md)、[UI 実装配置](../architecture/desktop-ui-implementation.md)、#915 r2 の native chooser 記録 [2026-09-10-915](2026-09-10-915-reopened-japanese-ui-labels.md)

## 調査で固定した事実

- `ComposerPanel.tsx` の添付欄は「ファイルを選択」ボタン、件数表示、`accept='image/*,video/*'` の hidden input だけで、対応形式は利用者に見えなかった。
- Linux（WebKitGTK）では #915 で入れた `src-tauri/src/file_dialog.rs` が WebKit の MIME filter を唯一の GTK filter（名前「対応するファイル」）として設定していた。「すべてのファイル」が無いため `.txt` は選択できず、アプリ側の判定・理由表示に到達しない。これが Issue の「ピッカーで拒否され、説明が出ない」の実体。
- Windows（WebView2）は Chromium のダイアログに「すべてのファイル」があり、`.txt` を選ぶと `handleColumnDraftAttachmentSelection` が `draft.error` に「未対応の添付タイプです: name」を入れて `composerError` として表示する。ただし対応形式を含まず、複数失敗は先頭 1 件だけ、error 段落は支援技術へ通知されない（`role` なし）。この経路には unit / browser test が無かった。
- 旧 DM composer（`DesktopShellAuxiliaryPanels`、現在は非表示）も `composeInteractions.ts` で同じ判定と文言を使う。
- backend（`crates/app-api/src/timeline.rs` の `put_blob`）は mime を検証しない。frontend の判定が唯一の guard であり、判定は FileReader より前にあるため非対応ファイルは読み込まれない。
- 動画は ADR 0004 に従い poster 生成に成功したものだけ添付できる。対応 codec は端末の WebView に依存するため、案内では拡張子・codec を列挙しない。

## 修正前の再現

- 2026-09-11、基準 commit に次を追加して Vitest を実行し、2 件が失敗することを確認した。
  - `useDesktopShellActions.test.tsx` "unsupported Column Draft attachments show one reason with the rejected count and are never read": `notes.txt` / `photo.png` / `report.pdf` / `unknown.bin` を選択すると `error` が先頭 1 件の旧文言になり、期待した「先頭名 + 残り 2 件」の文言と一致しない。
  - `ComposerPanel.attachments.test.tsx` "attachment control explains supported formats before choosing and announces a rejection": 案内文「画像と動画のみ添付できます。…」が存在しない（`getByText` で失敗）。
- DM 経路の test "unsupported DM attachment shows the same reason and keeps the DM draft untouched" は変更前から通る characterization（文言 key は同じで、locale 側の本文だけが変わる）。
- 本セッションは remote container のため、Linux .deb（WebKitGTK）と Windows の Tauri 実機で変更前のダイアログを観測していない。GTK `FileChooserNative` に単一 filter を `set_filter` した場合に対象外ファイルを選べないことは、`file_dialog.rs` の実装と GTK の仕様から固定した。

## 固定した受入条件・不変条件

| ID | 条件 | 作業 / 証跡 |
| --- | --- | --- |
| AC-1 | 添付欄に対応形式の案内（画像と動画のみ、テキスト等は不可）が常時表示され、ボタンの accessible description として渡る。ja / en / zh-CN で表示 | T2 / TR-6。`ComposerPanel.attachments.test.tsx`、`composer-localization.spec.ts` |
| AC-2 | 非対応ファイルを選ぶと、ファイル名と対応形式を含む理由が composer 内に `role="alert"` で出る。複数失敗は先頭名と残り件数を示す | T3 / TR-1〜3。`useDesktopShellActions.test.tsx`、`ComposerPanel.attachments.test.tsx`、browser spec |
| AC-3 | Linux native chooser の既定 filter 名が「画像と動画」になり、「すべてのファイル」を選べる。非対応を選ぶと AC-2 の理由がアプリ内に出る | T4 / TR-5。`file_dialog.rs` の locale test。実機は未実施 |
| AC-4 | 案内文言と対応形式が runbook にも記載され、実装（`accept` と判定）と一致する | T6。`mvp-user-quickstart.md`、`mvp-troubleshooting.md`。test で `accept='image/*,video/*'` を固定 |
| INVAR-1 | 画像・動画の添付、poster 生成、部分成功、削除、同一ファイル再選択、pending / 引用時の添付禁止を変えない | 既存 `ComposerPanel.attachments.test.tsx` 4 件、footer / actions の既存 test、browser 既存 7 件 |
| INVAR-2 | 非対応ファイルは読み込み・base64 化・送信されず、既存の本文・添付は失われない | actions test（`FileReader.prototype.readAsDataURL` 呼出し 0、`buildImageDraftItem` は png の 1 回のみ、`content` 保持）、browser spec（本文・添付 1 件の保持） |
| INVAR-3 | 選択・取消だけでは投稿しない。新しいファイル読取 IPC や filesystem 権限を追加しない | `file_dialog.rs` は filter を 1 つ足すだけで `select_files` / `cancel` の経路は不変。frontend の submit 経路は変更なし |

## Surface inventory

| ID | 入口・trigger | helper / owner | 読み書き・副作用 | 条件 / transition | 証跡 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | 投稿 / 返信 / DM の column composer「ファイルを選択」→ 選択 | `ColumnComposerFooter` → `ComposerPanel` → `handleColumnDraftAttachmentSelection` → `formatUnsupportedAttachmentMessage` | 対応ファイルだけ draft へ追加、`error` に理由、`attachmentInputKey` +1 | AC-1, AC-2, INVAR-1, 2 / TR-1〜3 | actions test、component test、browser |
| INV-2 | 旧 DM composer（非表示、`showComposer=false`） | `composeInteractions.handleDirectMessageAttachmentSelection` → 同 helper | `directMessageError` に理由、draft 不変 | AC-2 / TR-1 | actions test |
| INV-3 | Linux main WebView の file chooser 要求 | `file_dialog.rs::install` | 既定 filter「画像と動画」+「すべてのファイル」。選択結果は従来どおり WebKit の `select_files` へ | AC-3, INVAR-3 / TR-5 | Rust locale test |
| INV-4 | `columnDrafts` の `error` クリア（本文入力、展開、返信先・引用の解除） | 既存 `updateDraft` | `error: null` | TR-3 | browser spec（入力で alert が消える） |

期待 inventory 差分: 入口・sink の追加は 0。shared helper `formatUnsupportedAttachmentMessage` を追加し、caller は INV-1 と INV-2 の 2 か所（`rg formatUnsupportedAttachmentMessage` で確認）。判定条件は各 caller に残し、意味を変えていない。

## 状態遷移

| ID | 事前状態 / sequence | 期待状態 | 禁止する副作用 | 検証 |
| --- | --- | --- | --- | --- |
| TR-1 | 本文あり、添付なし → `.txt` 1 件 | 理由（単数形）、本文保持、添付 0 件 | 読取、送信、本文消失 | actions test、browser |
| TR-2 | 同上 → `.txt` + `.png` + `.pdf` | png だけ添付 1 件、理由「notes.txt ほか 1 件」（unit は 2 件） | pdf / txt の読取 | actions test、browser |
| TR-3 | 理由表示中 → 本文入力 | 理由が消え、添付は残る | 添付消失 | browser |
| TR-4 | pending / 引用中 | picker 無効（既存） | 読取 | 既存 test |
| TR-5 | Linux で「すべてのファイル」→ `.txt` | TR-1 と同じ | Rust 側での読取 | 実機未実施（Rust test は文言のみ） |
| TR-6 | ja ↔ en ↔ zh-CN | 案内・理由・filter 名が追従 | 下書き消失 | component test、browser（ja / en）、Rust test |

## 作業・検証の結果

- T1: 上記の失敗 test 2 件を基準 commit で確認。
- T2: `ComposerPanel.tsx` に `composer.supportedFormats` の段落を追加し、ボタンの `aria-describedby` を件数 + 案内の 2 要素にした。error 段落に `role='alert'`。`shell-phase1-part1.css` に `.composer-attachment-formats`（composer hint と同じ 0.8rem / `--muted-foreground`。新しい色 token なし）。
- T3: `lib/attachments.ts` に `formatUnsupportedAttachmentMessage` を追加。`useDesktopShellActions.ts` は非対応名を集めて 1 文にし、poster 失敗は別文として連結。`composeInteractions.ts` も同 helper を使う。
- T4: `file_dialog.rs` に「すべてのファイル」filter（`*`）を追加。既定は従来どおり MIME filter。filter 名は `composer.supportedFiles`（「画像と動画」）、追加 filter は `composer.allFiles`。Rust test に filter 文言を追加。
- T5: `ComposerPanel.stories.tsx` に `RejectedAttachment`。`composer-localization.spec.ts` に ja / en の非対応選択 2 件（案内、accessible description、alert 文言、部分成功、本文保持、input reset、focus、入力で alert が消える）。
- T6: runbook 2 件へ追記、本記録、Issue 本文の lifecycle 形式化。

### 実行結果（Linux container、Node 22.22、pnpm 10.16.1、Rust 1.92）

結果は PR 本文と同じ。以下は本記録作成時点のローカル実行。

| 検証 | 結果 |
| --- | --- |
| targeted Vitest（actions / ComposerPanel / footer / i18n / attachments） | 成功 |
| `composer-localization.spec.ts`（chromium） | 9 件成功（新規 2 件を含む） |
| 全体 `eslint` / `tsc` / Vitest / Storybook build / Playwright chromium / visual smoke | PR 本文の「検証とリスク」を参照 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib file_dialog` | PR 本文を参照 |
| `cargo xtask tauri-check` / `e2e-smoke` | PR 本文を参照 |

Playwright は repo 固定の chromium build が container に無いため、`/opt/pw-browsers/chromium` を `executablePath` に指定する未 commit の local config で実行した。

### 未確認の境界

- Linux .deb（WebKitGTK）実機で「すべてのファイル」filter の表示・切替、`.txt` 選択後の理由表示を観測していない。remote container のため Tauri 実 App を起動できない。GTK の filter 追加は #915 で実機確認済みの `add_filter` / `set_filter` と同じ API を使う。
- Windows（WebView2）実機での理由表示は未確認。browser（Chromium）の成功を実機成功とは記録しない。
- Linux visual baseline: composer は既存 visual spec の対象 surface に含まれないため baseline の変更はない。CI の視覚 step が赤くなった場合は `Kukuri Visual Baseline` で再生成する。

## 追加発見と分類

- `composerError` 段落に `role` が無く支援技術へ通知されない: AC-2 の「理由が出る」に含めて本 PR で修正（Existing-gap）。
- 複数失敗の先頭 1 件表示: AC-2 に含めて修正（Existing-gap）。
- 英語 error 文言の parity rule（大文字開始・終端句読点）に合わせて "The file “name” cannot be attached." とした。
- 設定内のヘルプ画面、拡張子・codec の列挙、backend の mime 検証は New-requirement / Optional-hardening として本 Issue に含めない。
