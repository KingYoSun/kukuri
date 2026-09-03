# Implementation Plan

## Goal
#858 の再監査で判明した表示漏えいを閉じ、成人向け表示設定が既定 OFF の間は、成人向けラベル付き投稿の通知 preview、OS 通知本文、Community Index 検索／推薦結果、引用／埋め込みの本文・メディアを一貫して代替表示または非表示にする。Community Index は署名済み投稿とラベルの解決が完了するまで Node 由来の `entry.text` を表示しない fail-closed 契約とし、法務・設計・進捗文書を実装と自動検証へ再同期して #858 を再 Close 可能にする。

## Non-goals
- 18 歳以上の自己申告 gate、成人向け表示設定の永続化、composer の self-label UI を作り直さない。
- Community Index protocol や Community Node の index schema に `content_labels` を追加せず、Node 判定を投稿者の署名済み self-label と同じ信頼源として扱わない。
- ラベルなしコンテンツを自動判定・非表示にする VLM／moderation 機能や、DM の成人向け self-label を追加しない。
- 通知一覧、Community Index、PostCard のレイアウトや design language を再設計しない。

## Assumptions
- 成人向け判定の canonical source は署名済み投稿 envelope／projection の `content_labels` であり、表示設定の canonical source は Rust 側 `ContentDisplaySettings` のままとする。
- object-backed notification は生成時の content safety metadata を local projection に保持し、旧 row または欠損時は object projection から補完する。安全判定を完了できない場合、設定 OFF では preview を fail-closed で抑止する。
- Community Index の `IndexEntryView.text` は検索候補を返す Node 由来データであり、client が canonical post と署名済み label を解決するまでは user-facing 本文として使用しない。
- direct message／follow notification は object content label の対象外として既存表示を維持し、成人向け object notification の本文だけを抑止する。
- 不具合修正は通知、未解決／解決失敗の Community Index、引用／埋め込みの各漏えいを現行実装で再現する failing test を先に追加してから行う。

## Definition of Done
- 成人向け object notification は設定 OFF 中、通知一覧にも OS toast にも raw preview text を出さない。設定 ON では既存 preview を表示でき、DM／follow、既読判定、quiet mode、通知 cursor は回帰しない。
- 通知 candidate から store row、`NotificationView`、generated TypeScript contract まで、空ラベルと未解決を混同しない content safety metadata が伝播する。既存 DB migration と memory／SQLite backend parity が成功する。
- Community Index は post resolution の loading、missing entry、`post: null`、API error のすべてで `entry.text` を表示せず、解決済み canonical post だけを表示する。解決済み成人向け投稿は設定 OFF で共通 placeholder、ON で本文表示となる。
- 通常投稿、repost／quote source、reply preview／embed のいずれかが成人向けラベルを持つ場合、設定 OFF では raw text、添付取得、prefetch、decode、viewer 導線が発生しないことを回帰テストで証明する。
- ADR 0046、data classification、利用規約・プライバシーポリシーと ja／en／zh-CN のアプリ内法務文面、進捗記録、UI review record が実装結果と一致する。対象 test、IPC codegen check、`cargo xtask check`、`cargo xtask test`、`cargo xtask desktop-ui-check`、`cargo xtask e2e-smoke` が成功する。

## Plan
| ID | Task | Outcome | Files / Areas | Acceptance Criteria | Validation | Depends On |
|---|---|---|---|---|---|---|
| T1 | 通知 preview 漏えいの fail-first contract を追加する | object notification の label 伝播欠落と、アプリ内／OS の raw preview 表示を決定的に再現できる | `crates/app-api/src/tests/notifications.rs`、`crates/store/src/tests/backend_parity/`、`apps/desktop/src/shell/DesktopShellPage.notifications.test.tsx`、`apps/desktop/src-tauri/src/commands/background_notifications.rs` | 成人向け reply／mention／repost candidate が安全 metadata を失うこと、設定 OFF で `preview_text` と OS body に raw text が残ることを修正前に失敗させる。設定 ON、unlabeled object、DM／follow、preview-body OFF の既存期待値も fixture に含める | `cargo test -p kukuri-app-api tests::notifications -- --nocapture`、`cargo test -p kukuri-store backend_parity -- --nocapture`、`cargo test -p kukuri-desktop-tauri background_notifications -- --nocapture`、`cd apps/desktop && npx pnpm@10.16.1 test -- DesktopShellPage.notifications.test.tsx` | None |
| T2 | 通知の content safety metadata を永続化し、共通 gate を IPC と OS 通知へ適用する | 通知生成時の署名済み label を失わず、すべての通知 consumer が同じ表示可否を使える | `crates/app-api/src/service/{mod.rs,notifications_support.rs,direct_messages_delivery_support.rs,direct_messages_subscription_support.rs}`、`crates/app-api/src/views.rs`、`crates/store/{migrations,src/models.rs,src/row_mapping.rs,src/sqlite/notifications.rs,src/memory/notifications.rs}`、`apps/desktop/src/lib/api/{types.generated.ts,__fixtures__/views/notification_view.json,__fixtures__/viewsContract.ts}`、`apps/desktop/src/shell/page/DesktopShellAuxiliaryPanels.tsx`、`apps/desktop/src-tauri/src/commands/background_notifications.rs` | object candidate から nullable／明示状態付き safety metadata を store と `NotificationView` へ伝播し、migration 前 row は projection で補完、補完不能時は OFF で preview を抑止する。成人向け設定 OFF では Rust view が raw preview を返さず、frontend は共通の安全な placeholder、OS toast は raw 本文なしとなる。ON へ切替後は保存済み preview を再利用できる | T1 の全 test、`cargo test -p kukuri-store migrations -- --nocapture`、`cargo xtask ipc-types --check` | T1 |
| T3 | Community Index と引用／埋め込みの fail-first contract を追加する | Node 由来本文の先行表示と nested label の見落としを、loading・failure・resolved の境界ごとに再現できる | `apps/desktop/src/components/core/{communityIndexPostCardView.test.ts,CommunityIndexWorkspace.test.tsx}`、`apps/desktop/src/shell/DesktopShellPage.adultContentGating.test.tsx`、必要なら `DesktopShellPage.communityIndex.test.tsx` | resolution pending、response に key なし、`post: null`、resolver reject の各状態で `entry.text` が見える現行挙動を修正前に失敗させる。解決済み adult/unlabeled post と、adult-labeled `repost_of`／`reply_preview` を fixture 化し、OFF 時の raw text 不在と media API hit 0、ON 時の復帰を assert する | `cd apps/desktop && npx pnpm@10.16.1 test -- communityIndexPostCardView.test.ts CommunityIndexWorkspace.test.tsx DesktopShellPage.adultContentGating.test.tsx DesktopShellPage.communityIndex.test.tsx` | None |
| T4 | Community Index を canonical resolution 完了まで fail-closed にし、nested post gate を統一する | 検索候補本文を安全確認前に描画せず、通常投稿と quote/embed に同じ判定を適用できる | `apps/desktop/src/components/core/{CommunityIndexWorkspace.tsx,communityIndexPostCardView.ts,PostCard.tsx,types.ts}`、`apps/desktop/src/shell/{media.ts,viewModels/useTimelineViewModels.ts,data/usePreviewableMediaAttachments.ts}`、関連 locale (`apps/desktop/src/i18n/locales/{ja,en,zh-CN}/`) | Workspace が entry ごとの `loading`／`resolved`／`failed` を保持し、resolved post がない状態では `entry.text` と content action を出さず、loading と取得不能を区別した安全な状態を表示する。成功時は `resolvedPost.content` と署名済み labels を使い、adult gate は top-level、repost source、reply preview を覆う。OFF 時は nested attachment を prefetch／decode しない | T3 の全 test、`cd apps/desktop && npx pnpm@10.16.1 typecheck`、`cd apps/desktop && npx pnpm@10.16.1 lint` | T3 |
| T5 | 設計・法務・監査証跡を実装へ再同期する | 「通知は対象外」「Community Index は保護済み」という旧説明を除去し、既定 OFF と信頼限界を一貫して説明できる | `docs/adr/0046-age-attestation-adult-content-gating.md`、`docs/legal/adult-content-display-data-classification.md`、`docs/legal/{terms-of-service.md,privacy-policy.md}`、`apps/desktop/src/i18n/locales/{ja,en,zh-CN}/legal.json`、`docs/progress/2026-09-01-issue-858-age-attestation-adult-content-gating.md`、`docs/ui-reviews/2026-09-01-issue-858-age-gate-safety.md` | ADR と classification に通知 safety metadata、OS body 抑止、Community Index の resolution fail-closed、quote/embed contract を明記する。canonical 法務文書と 3 locale mirror が「自己申告と表示許可は別」「既定 OFF」「ラベルなしの安全は保証しない」を同義で保つことを差分または test で確認する。進捗／UI review は追加 test と before/after 証跡を反映し、未実装を実装済みと記さない | `cd apps/desktop && npx pnpm@10.16.1 test -- LegalDocumentView.test.tsx`、法務文書と locale mirror の既存 contract test、`git diff --check` | T2、T4 |
| T6 | 全体 validation と #858 の Close 判定を確定する | 再監査の未チェック項目を command/result と UI 証跡から再評価できる | 全変更、GitHub Issue #858 本文、PR 本文 | 通知一覧／OS toast／Community Index loading・error・adult resolved／quote・embed を ja・en・zh-CN、dark・light、代表 desktop viewport で確認し、PR に変更分類、対象 state、before/after、keyboard・Accessibility、未確認事項を記録する。全 gate 成功後に Issue 本文の監査残件と受入条件を実装証跡で更新し、未充足がゼロの場合だけ Close する | T1〜T5 の targeted test、`cargo xtask doctor`、`cargo xtask check`、`cargo xtask test`、`cargo xtask desktop-ui-check`、`cargo xtask e2e-smoke`、`cargo xtask oversized-files`、`git diff --check` | T5 |

## Decision Needed / Blockers
None

## Out of Scope
- Community Node index protocol／schema、CN advisory、VLM moderation、relay／DHT の変更。
- DM 自体への成人向け label、ラベルなしコンテンツの推定判定、既存 blob の削除。
- 通知／検索／PostCard の全面的な UI 再設計と、#854／#855／#857／#860 の残作業。

## Single Next Action
T1 の成人向け reply／mention／repost notification fixture を追加し、現行実装で app preview と OS notification body に raw text が残る failing contract を固定する。
