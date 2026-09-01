# Issue #858 age attestation and adult content gating

## Summary

無償 Preview 公開のブロッカー(#853 Phase A)として、(1) 初回起動時の「18歳以上である」旨の自己申告を必須化し、(2) 成人向け表現を明示的に許可するまで安全に非表示とする実装を行った。仕様は ADR 0046(`docs/adr/0046-age-attestation-adult-content-gating.md`)に固定し、ローカル状態 2 件の ADR 0002 分類を `docs/legal/age-attestation-data-classification.md` / `docs/legal/adult-content-display-data-classification.md` に置いた。法務文書は Legal bundle version 3 として利用資格(18歳以上・自己申告は公的年齢確認ではない)と成人向け表現の既定非表示を追記し、既存同意ユーザーには再同意が発火する(i18n `legal` namespace の ja / en / zh-CN ミラーも更新)。

意図的にやらなかったこと: CN advisory / VLM NSFW 判定のラベル配信(ADR 0046 でスコープ外と明記)、DM の self-label、`blobs.db` に既存の blob の削除(iroh-blobs 0.103 の公開 API で不可。成人向けメディアは ephemeral fetch で最初から永続化しない)、通報起点のラベル付与。ラベルなしコンテンツは通常表示(fail-open)であり、その限界は ADR・規約・設定画面の文言で明示した。

## 実装内容

- 年齢自己申告(#858 要件 1-3): `<db_path>.app-consent.json` に文書同意とは独立した `age_attestations` レコードを追加(`AGE_ATTESTATION_VERSION = 1`)。`accept_app_consents` は `age_attested` を受け取り、未申告なら拒否する。startup gate は申告完了まで `DesktopRuntime` を構築しない(fail-closed、既存の app consent gate と同一経路)。`ConsentGate` に必須チェックボックスと「公的な年齢確認ではない」注記を追加し、現行版で申告済みの再同意時には再表示しない。`AboutPanel` に申告状態を表示。
- self-label(ADR 0046 §3): 署名対象 content(`KukuriPostEnvelopeContentV1` / `RepostSourceSnapshotV1` / profile post 系)に `content_labels`(既知値 `adult`、空なら省略で旧 wire と一致)を追加し、`PostView` / `ReplyPreviewView` / `RepostSourceView` に伝搬。`create_post` はラベルを検証し(既知値のみ)、composer に「成人向けとして申告する」トグルを追加。
- 取得ゲート(#858 要件 5, 7): store migration `20260901000000_adult_content_labels` で `object_index_cache.content_labels_json` と逆引きテーブル `adult_media_hashes` を追加。projection 書き込み(引用 snapshot 含む)と profile timeline / 引用ビュー構築時に対象 hash を記録し、`blob_media_payload` は「対象 hash かつ表示設定 OFF」で blob 取得を行わず `None` を返す(fail-closed バックストップ)。表示設定 ON の間の成人向けメディア取得は `fetch_blob_ephemeral` を使い `blobs.db` へ永続化しない。
- 表示設定(#858 要件 3-4): canonical source は `<db_path>.content-display.json`(既定 OFF)。`get_content_display_settings` / `set_adult_content_display_enabled` コマンドを追加し、frontend は mirror(`adultContentEnabled` slice)。設定画面に「セーフティ」セクション(`SafetyPanel`)を新設。
- 表示経路の一貫性(#858 要件 6): `buildPostCardView` が成人向けラベル付き投稿(引用元ラベル含む)のメディアを `gated` 状態にし、`PostMedia` が共通プレースホルダーを表示、gallery を空にして `MediaViewerDialog` への流入も止める。テキストは `PostCard` が本文・引用埋め込みを代替表示にする。検索/発見/推薦(`communityIndexPostCardView`)の解決済み投稿にも同じゲートを適用。プリフェッチ(`usePreviewableMediaAttachments`)は対象添付を除外し、OFF へ戻すと表示済み object URL を revoke して取得記録を消す(ON へ戻せば再取得)。
- IPC codegen: `cargo xtask ipc-types` 再生成(`ContentDisplaySettings`、`content_labels`、`CreatePostRequest.content_labels`)。

## 検証

- `cargo xtask check` / `cargo xtask test`(workspace 全体、完走)
- `cargo xtask tauri-check`
- `cargo xtask ipc-types` 後の `git diff` 差分なし(CI 契約)
- Rust: `crates/app-api/src/tests/media.rs` に取得ゲート契約 3 件(既定 OFF でラベル付き hash のバイト列を返さない / ON で返し OFF で再び止まる / ラベルなしは非影響 / 未知ラベル拒否)、`src-tauri/src/state.rs` に自己申告の独立判定・round-trip、store に migration round-trip + schema golden 更新
- frontend: `npx pnpm@10.16.1 exec vitest run src/shell/DesktopShellPage.adultContentGating.test.tsx`(4 件: 対象 hash への `getBlobMediaPayload` 不発火とプレースホルダー、テキスト単独の代替表示、ラベルなし非影響、設定 ON/OFF の取得開始・停止と表示破棄)、`src/App.test.tsx`(申告チェック必須・再同意時の非表示・payload 形状)、`src/i18n/parity.test.ts`、`cargo xtask desktop-lint` / `desktop-test`

関連: #853(親)、#857(文書単位同意)、#609(ephemeral fetch)、ADR 0046、`docs/ui-reviews/2026-09-01-issue-858-age-gate-safety.md`
