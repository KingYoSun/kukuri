# #960 見つける検索の空状態と次の行動

- 判定: In progress（実装・ローカル検証完了。CI／merge の最終結果は Issue の Current status と PR を参照する）
- Scope revision: `960-plan-v1 / 2026-09-11`（ユーザー承認済み。未決3点は推奨案で確定）
- 基準commit: `5a3f6a1`
- リスク区分: B。表示と既存導線の呼出しのみ。network / IPC / 認証 / 同意 / 永続化は変更しない。
- UI分類: 既存画面の改善。利用者は「見つける」で人や投稿を探す利用者。単一目的は、検索成功0件のときに何をどこで探したか・0件の理由・次の行動を同じ場所で示すこと。
- 対象外: CN への索引状況 read API、表示名／pubkey の索引化、DM／フォロー機能、ローカル投稿の全文検索、自動再試行、非 ready 状態の文言再設計。
- 正本: [Issue運用手順](../runbooks/issue-lifecycle.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[ADR 0025](../adr/0025-community-node-indexing-foundation.md)、[DESIGN](../../DESIGN.md)、[UI実装配置](../architecture/desktop-ui-implementation.md)。

## 調査で固定した事実

- 空状態は `CommunityIndexWorkspace.tsx` で query 成功かつ0件のときだけ `communityIndex.empty` を1行表示していた。検索先、範囲、索引状況、次の行動を持たない。
- CN の検索は索引済み投稿本文のみを全文照合する（`crates/cn-indexer/src/arcadedb.rs` の `SEARCH_INDEX([text])`）。表示名・pubkey は照合対象ではない。見つけるは選択ノードの supported 公開トピック横断、トピック内検索は scope 限定。
- 索引に載るのは operator の supported set または承認済み申請 × safety `allow` の投稿のみ（ADR 0025 §2.2 / §2.5）。索引はノード固有。
- 索引状況を読む手段はない。`POST /v1/indexing/requests` の `pending/approved/rejected` は申請時応答のみで、manifest にも supported topic は含まれず、client も申請結果を保存しない。
- 非 ready 状態（同意・認証・機能非提供・停止など）は `CommunityIndexAvailabilityNotice` が既に説明している。

## 修正前の再現

- 2026-09-11、基準 commit に `CommunityIndexWorkspace.emptyState.test.tsx` だけを追加して Vitest を実行。7件すべてが `role="status"` の空状態が無いことで失敗（`Unable to find role="status" and name "No matching posts found."`）。既存の `CommunityIndexWorkspace.test.tsx` 26件は成功。
- Issue 添付の 0.2.1-preview.1 画像（Debian 13、日本語）は「該当する投稿は見つかりませんでした。」の1行だけを示す。本セッションは remote container のため Tauri 実機の変更前観測は取得していない。

## 固定した受入条件・不変条件

| ID | 条件 | 作業 / 証跡 |
| --- | --- | --- |
| AC-1 | 空結果で、検索先ノード、検索範囲（このトピック／チャンネル／このノードが索引する公開トピック全体）、照合対象が投稿本文であることが読める | T2, T3, T4 / TR-1 |
| AC-2 | 0件の理由として「索引に未反映の可能性」「索引対象外の可能性」「ユーザー名・IDは本文に含まれない限り一致しない」を断定せずに示す。pubkey 入力時は照合対象外であることとユーザーを開けることを示す | T2, T3, T4 / TR-1, TR-5 |
| AC-3 | 次の行動が実動する: 再検索、タイムライン、コミュニティノード設定、接続診断、トピック内検索では索引登録申請、pubkey 入力ではユーザーを開く | T3, T5 / TR-2, TR-3, TR-5 |
| AC-4 | 取得失敗、非 ready、検索語未入力、loading、node 切替は空状態と混同されない | T1, T5 / TR-4 |
| AC-5 | ja / en / zh-CN、dark / light、1280 / 390px で意味と操作を保持し、Column 実幅に収まる | T4, T5, T6 / TR-6 |
| INVAR-1 | 空状態の表示・CTA は新しい外部送信、認証、同意、購読、永続化を起こさない | T3, T5 / TR-2 |
| INVAR-2 | 既存の結果表示、node 切替による結果失効、古い応答の破棄、通報導線を変えない | 既存 test 26件 / TR-4 |
| INVAR-3 | 空表示は query 成功時に限る | T1, T5 / TR-4 |

## Surface inventory

| ID | 入口・trigger | helper / owner | 読み書き・副作用 | 条件 / transition | 証跡 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | 検索／発見／おすすめの query 成功0件 | `communityIndexEmptyGuidance`、`CommunityIndexEmptyState` | 表示のみ | AC-1, AC-2, INVAR-3, TR-1 | guidance table test、component test |
| INV-2 | 「もう一度検索／試す」 | 既存 `runQuery`（同じ node・scope・検索語） | 既存 read API 1回 | AC-3, INVAR-1, TR-2 | component test、browser |
| INV-3 | 「接続診断を開く」「コミュニティノード設定を開く」 | `handleOpenSettingsSection`（既存 drawer section 切替） | drawer / URL / focus | AC-3, INVAR-1, TR-3 | shell test、browser |
| INV-4 | 「タイムラインを見る」 | 既存 `focusPrimarySection('timeline')` | column / URL | AC-3, TR-3 | shell test、browser |
| INV-5 | 「索引登録を申請」（topic mode のみ） | 既存 `setIndexingTarget` → `CommunityIndexingRequestDialog` | dialog 表示のみ。送信は dialog 内の既存操作 | AC-3, INVAR-1, TR-2 | component test |
| INV-6 | 「このユーザーを開く」（64桁 hex 入力時） | 既存 `openAuthorDetail` | profile column / URL | AC-3, TR-5 | component test、browser |

期待 inventory 差分は INV-1〜6 の追加のみ。新規 storage / network sink はない。production の見つけるは explore mode のため INV-5 の CTA は topic mode（Storybook・test）でのみ表示され、explore ではトピック一覧からの申請を文言で案内する。

## 状態遷移

| ID | 事前状態 / sequence | 期待状態 | 禁止する副作用 | 検証 |
| --- | --- | --- | --- | --- |
| TR-1 | ready → 検索 → 成功0件 | 検索先・範囲・照合対象・理由・CTA を表示 | 0件を故障や未使用と断定 | component / browser |
| TR-2 | 空状態 → 再検索 / 申請 | 同じ検索語を同じ node へ再送。申請は dialog 表示のみ | 送信・認証・同意・保存 | component（mutation spy 0）、browser（call 2回） |
| TR-3 | 空状態 → 設定 / 診断 / タイムライン → 戻る | drawer section・URL が一致し、閉じても検索語と空状態を保持 | 保存・同意・購読変更 | shell、browser |
| TR-4 | 空状態 → error / 429 / node 切替 / loading | 空状態を消し、error は error として表示 | 取得失敗の0件化 | component |
| TR-5 | 64桁 hex を検索 → 0件 | 「ユーザーIDは照合対象外」とユーザーを開く CTA | pubkey を本文照合の失敗として説明 | guidance / component / browser |
| TR-6 | 3 locale × 2 theme × 1280 / 390 | 文言・操作を保持し、Column 内に収まる | 横 scroll、clipping | browser、Storybook |

## 作業・検証の結果

- T1: 失敗 test 7件を基準 commit で記録（上記）。
- T2: `communityIndexEmptyGuidance.ts` を追加。mode / scope / operation / 検索語から scope、理由、action、索引登録申請 target、pubkey を決める純関数。table-driven test 15件。
- T3: `CommunityIndexEmptyState.tsx` を追加し、`CommunityIndexWorkspace.tsx` の1行 `empty-state` を `role="status"` の Notice へ置換。結果 state に実際に送った検索語を保持し、編集中の入力で説明を変えない。`DesktopShellPrimaryWorkspace.tsx` / `DesktopShellPage.tsx` で既存 `handleOpenSettingsSection('connectivity')`、`focusPrimarySection('timeline')`、`setIndexingTarget` を配線。新規 API / storage / timer なし。
- T4: ja / en / zh-CN の `shell.json` に `communityIndex.emptyState.*` を追加。既存 `communityIndex.empty` は見出しとして維持し、既存 browser test の完全一致を壊さない。
- T5: component 7件、guidance 15件、shell 1件、browser 7件（3 locale × 2 theme × 1280 / 390、pubkey）を追加。既存 `community-index-layout.spec.ts` の `empty` 完全一致は変更なしで成功。
- T6: `CommunityIndexEmptyState.stories.tsx` に 7 story。axe（story root scope、42条件 = 7 story × 3 locale × 2 theme）は[違反0・要確認0](assets/issue-960/story-a11y.json)。document 全体で走らせた場合の `landmark-one-main` / `page-has-heading-one` / `bypass` は Storybook iframe 由来で component の欠陥ではない。
- T7: `mvp-troubleshooting.md` に空状態の読み方を1段落追記。Issue #960 に Current status / AC / INVAR を置き、索引状況 read API は New-requirement として #975 を起票。

### AC / INVAR と証跡

| 条件 | 実装・証跡 |
| --- | --- |
| AC-1 / AC-2 | `CommunityIndexEmptyState`。`CommunityIndexWorkspace.emptyState.test.tsx` の topic / channel / explore / pubkey / discovery、`communityIndexEmptyGuidance.test.ts`。[ja dark](assets/issue-960/empty-state-ja-dark.png)、[pubkey 入力](assets/issue-960/empty-state-user-id-ja-light.png)。 |
| AC-3 | 同 test で CTA ごとの callback 呼出しと `runQuery` 再送を確認。`DesktopShellPage.communityIndex.test.tsx` で connectivity section の `aria-current` / URL、Escape 後の検索語保持、timeline route。`community-index-empty-state.spec.ts` で 3 導線と再検索の call 数。 |
| AC-4 / INVAR-3 | 429 error 後と node 切替後に空状態が消える test。loading 中は既存 button 無効化。 |
| AC-5 | browser 6条件で `expectIndexContentContained` と document 横 scroll なし。[390px en light](assets/issue-960/empty-state-en-light-390.png)。Storybook 42条件。 |
| INVAR-1 | component / shell test で `submitCommunityNodeIndexingRequest`、`acceptCommunityNodeConsents`、`authenticateCommunityNode`、`setCommunityNodeConfig` の呼出し0。 |
| INVAR-2 | 既存 `CommunityIndexWorkspace.test.tsx` 26件、`consentGate` 2件、browser 228件成功。 |

### 実行結果（Linux container、Node 22.22、pnpm 10.16.1）

| 検証 | 結果 |
| --- | --- |
| `eslint` 変更 file / `tsc --noEmit` | 成功 |
| targeted Vitest（component / guidance / shell / parity） | 5 files 108件成功 |
| 全体 Vitest | 1回目は browser suite と Storybook build を並走させた状態で `DesktopShellPage.socialGraph.test.tsx` 1件が失敗（170 files / 1415件成功）。単独再実行は成功。負荷なしで全体を再実行し 171 files / 1416件成功 |
| Playwright chromium 全体 | 228件成功（`community-index-empty-state.spec.ts` 7件を含む）。repo 固定の chromium build が container に無いため、`/opt/pw-browsers/chromium` を `executablePath` に指定する未 commit の local config で実行 |
| Storybook build + axe | build 成功、42条件 違反0 |
| `cargo xtask oversized-files` | 成功（新規 file はいずれも 200 行未満） |
| `git diff --check` | 成功 |

### 未確認の境界

- Tauri 実機（Ubuntu / Windows WebView2）での表示・focus は未確認。remote container のため実 App を起動できない。browser（mock API）と Storybook の成功を実機成功とは記録しない。
- Linux visual baseline は変更なし。既存 explore の visual test は検索未実行のため空状態を含まず、新規 baseline は追加していない（Linux workflow 生成の取り込みが必要になる場合は別途）。
- 実 CN で索引 pending の状態を再現した証跡はない。空状態の説明は ADR 0025 と実装から固定した事実に基づく。
