# #1056 タイムライン向け content advisory 一括照会・採用ノード設定・利用規約改訂の作業記録

## 現在の状態

- リスク区分 C、Scope revision: 2026-09-15（2026-09-16 追加要求を含む）、基準 commit: `04b403dc`。
- 2026-09-16 にユーザーが実装計画を承認した。Issue 操作・commit・PR 作成・CI 成功後の squash merge は承認待ちなしで行う。区分 C の独立監査 `PASS` は merge 条件のまま。
- AC / INVAR、固定 inventory と状態遷移の正本は [Issue #1056](https://github.com/KingYoSun/kukuri/issues/1056)。UI 採用記録は [review record](../ui-reviews/2026-09-16-1056-timeline-advisory-and-node-adoption.md)。
- 本番反映と統合手動確認は C5（[#1068](https://github.com/KingYoSun/kukuri/issues/1068)）へまとめる。本 Issue の Close 条件に本番確認は含めない。

## 追加要求と決定（2026-09-16）

| 項目 | 決定 |
| --- | --- |
| 採用ノードの設定 | node 設定に `content_advisory_enabled`（既定 true）を持たせ、設定画面のノード欄で切り替える。採用しない node へは照会せず、見つけるの advisory も採用しない |
| 照会中の表示 | 照会が確定するまでメディアを取得せずスケルトンにし、確定後の代替表示とは表示を分ける。本文は照会中も表示する |
| 照会と表示設定 | 照会は表示設定の ON / OFF にかかわらず行う（ON 中も advisory 付き hash は ephemeral 取得にするため） |
| legal bundle | version 6、施行日 2026-09-16 |

## 修正前の再現

- CN: 基準 commit には `POST /v1/advisories/lookup` が無く、`crates/cn-user-api/tests/advisory_lookup.rs` の 5 件は route 不在で失敗する（404 / 405）。
- client: 基準 commit で `timeline-advisory.spec.ts` を実行すると、推定を返す fixture でもタイムラインは画像をそのまま描画する（review record の変更前画像）。C3 の記録にある「見つける以外の経路では代替表示が出ない」既知の限界と同じ現象。
- 法務: 基準 commit の利用規約 第3条 4 項は「投稿者の自己申告によるラベルに基づきます」のみで、`LEGAL_BUNDLE_VERSION = 5`。

## 対応

| 条件 | 実装 | 検証 |
| --- | --- | --- |
| AC-1 / AC-2 / INVAR-3 / INVAR-4 | `cn-protocol` に wire 型（`AdvisoryLookupRequest` / `AdvisoryLookupResponse`、上限 200、`INVALID_ADVISORY_LOOKUP`）。`cn-core::list_content_advisories_for_subjects` は risk signal を読み、自 node 発行・nsfw / objectionable・post / blob 対象・`Cleared` と失効を除外・`classifier_score` のみを (target, id, category) ごとに最新 1 件へ畳む。`cn-user-api` の門は索引参照と同じ構成・有効化・安定コード + 認証 + 同意。manifest の `node_id` を発行元とする | `advisory_lookup_returns_only_configured_node_signals`、`advisory_lookup_reads_do_not_mutate_state`、`advisory_lookup_requires_auth_and_consent`、`advisory_lookup_rejects_oversized_or_empty_batch`、`advisory_lookup_is_not_found_when_index_query_is_not_configured`、`cn-protocol` の wire contract 4 件 |
| AC-3 / INVAR-1 | `desktop-runtime` の `lookup_community_node_content_advisories`。採用 ON の設定済み node ごとに、session 確認（同意未成立・Deferred は HTTP 前に停止）→ token → manifest で発行元確認（取得できなければ送らない）→ 200 件ずつ POST → 401 は 1 回だけ再認証。応答は発行元・要求 subject・表示語彙・根拠で絞り、blob 対象を取得ゲートへ登録する。発行元は設定保存で破棄する in-memory cache | `lookup_sends_only_visible_ids_to_enabled_nodes`、`lookup_skips_nodes_with_content_advisory_disabled`、`lookup_without_configured_nodes_makes_no_request`、`lookup_stops_before_http_when_consent_is_pending`、`lookup_stops_before_http_when_manifest_is_unavailable`、`lookup_drops_unrequested_subjects_and_mismatched_issuer`、`lookup_reauthenticates_once_after_unauthorized`、正規化・分割の unit 3 件 |
| 追加要求（採用設定） | `CommunityNodeNodeConfig.content_advisory_enabled`（serde 既定 true、旧 JSON 互換、重複指定は OFF 優先）、`SetCommunityNodeConfigNode.content_advisory_enabled: Option<bool>`（未指定は保存値を維持）。見つけるの `apply_content_advisories` も採用 OFF なら advisory を落とす。設定 UI は `CommunityNodeAdvisoryAdoptionField` | `legacy_config_without_content_advisory_field_defaults_to_enabled`、`duplicate_nodes_keep_content_advisory_disabled`、`set_community_node_config_keeps_or_updates_content_advisory_adoption`、`community_node_index_drops_advisories_from_node_with_adoption_disabled`、`CommunityNodePanel.contentAdvisory.test.tsx`、`DesktopShellPage.communityNodeAdvisoryAdoption.test.tsx` |
| AC-3 / INVAR-2 | frontend の `useTimelineContentAdvisoryLookup` が、タイムライン・スレッド・ブックマーク・プロフィール・開いている Column・通知の subject を 300ms でまとめて照会する。照会状態は `timelineAdvisoryLookup { active, settled }` で持ち、`active` の間は照会済みでない subject を照会中とみなす（描画と同じ計算で判定するため、照会開始前の取得も止まる）。起動直後に node 状態が未取得の間も `active`。失敗・10 秒超過で照会済みにする。`usePreviewableMediaAttachments` は照会中の投稿と、推定のある投稿（表示 OFF）の添付を取得しない。表示 OFF へ戻したときの破棄集合にも推定のある投稿の添付を含める | `useTimelineContentAdvisoryLookup.test.tsx`（6）、`usePreviewableMediaAttachments.test.ts` の 2 件、`DesktopShellPage.timelineAdvisory.test.tsx` の取得 0・照会中・失敗・ON→OFF |
| AC-4 | `buildPostCardView` が本文・添付・引用元・返信先の推定を合成し（自己申告を根拠として優先）、`contentAdvisory` と `gatedBy` を渡す。`buildPostMediaView` に `pending` 状態、`PostMedia` に照会中スケルトンを追加し、代替枠は静止表示にした。通知プレビューは対象投稿の推定で伏せる | `contentAdvisories.test.ts`（`content_advisories_are_separate_from_signed_content_labels` を含む）、`postMediaView.test.ts`、`DesktopShellPage.timelineAdvisory.test.tsx`（返信先・申し立て・通知）、Playwright `timeline-advisory.spec.ts` |
| AC-5 | 外部送信表示・データフロー突合表・プライバシーポリシーへ「成人向け表現の推定の照会」と送る識別子・送らない情報を追記 | Tauri `canonical_legal_documents_match_the_runtime_bundle_version` に必須文言を追加 |
| AC-6 | 利用規約 第3条 4 項を改訂、3 文書を version 6 / 施行日 2026-09-16 に同期、`LEGAL_BUNDLE_VERSION = 6`、i18n `legal` ミラー（ja / en / zh-CN）と分類文書を更新 | `App.test.tsx`（version 5 同意済み → 再同意 gate → version 6 で受諾）、`LegalDocumentView.test.tsx`、`parity.test.ts`、runtime host test |
| 合成の有効化 | `CONTENT_ADVISORY_SYNTHESIS_DEFAULT = true`（ADR 0046 §6.4 どおり利用規約改訂と同じ変更） | `community_node_index_synthesizes_advisories_by_default`、無効化経路は `community_node_index_strips_advisories_when_synthesis_disabled` |

## inventory / transition の差分

Issue の INV-1〜6 / TR-1〜6 に、計画で逆引き追加した INV-2a〜2d・3a・3b・4a・4b・5a と TR-7〜13 を対応付けた（計画ファイルの表）。実装中の追加:

- **INV-3c（起動直後の未確定状態）**: 初回描画時点で node 状態が未取得だと、照会前に取得が走る経路があった（既存 test の失敗で発見）。`adoptingContentAdvisoryNodes` の `undetermined` と `active` 初期値 true で塞いだ。状態取得が失敗した場合は手元の状態で確定させ、照会中のまま止めない。
- **INV-3d（見つけるの解決済み投稿）**: プリフェッチの単一入口に照会中判定をかけると、タイムライン向け照会の対象外である見つけるの投稿が照会中のまま残る。見つけるの投稿は判定から外した（#1055 の index 応答経路で扱う）。

TR-13（token 無し）は、runtime の門が token 読み出し失敗で `AUTH_REQUIRED` を返して送信しない実装で満たす。session 確立が認証を伴うため単独の再現 test は置かず、`lookup_stops_before_http_when_consent_is_pending` と同じ門の順序で担保する。

## 実装上の判断

- CN の読み口は verdict 行ではなく risk signal にした。#1054 監査指摘 5（`Cleared` 後も verdict 行の advisory が残る）の影響をタイムライン経路では受けない。
- 照会の門は索引参照（`INDEX_QUERY_NOT_CONFIGURED` / `INDEX_QUERY_NOT_ACTIVATED`）と共有した。advisory は索引の判定に伴って発行されるため。distance opt-out は利用者が既に保持する投稿への照会であり新たな surfacing ではないため適用しない。
- 照会中判定を「照会済み集合に無い」で行う形にした。当初の「照会中集合に有る」形では、投稿が描画された同じ commit でプリフェッチが走り、取得が 1 回起きた（page test で検出）。
- Tauri の lib test は、この Windows 環境では変更前の main でも `STATUS_ENTRYPOINT_NOT_FOUND` で起動できない。CI でも lib test は実行していない。追加した必須文言の条件は同じ文字列で Python から再現して確認し、`cargo check --tests` でコンパイルを確認した。

## 独立監査と是正

- 1 回目（対象 `a964ffab`、別コンテキストの subagent）: **FAIL**。blocker 1 件、inventory 8 件中 適合 6 / 不適合 2 / 未分類 0。記録は PR #1072 の comment。
  - B-1（Regression）: 設定 JSON の新欄が常に出力されるが、kukuri-cli の出力 schema が追加欄を拒否し、CLI の `get/set_community_node_config` が失敗していた。
  - 必須 CI 失敗: 新 Tauri command が CLI の command 対応表で未分類（`command_parity`）。
  - non-blocker 1（config 取得失敗で照会先が未確定のまま残る）と 2（採用 node の組だけが変わると再照会しない）は Regression として同じ PR で修正した。
- 是正（`b2897171`）: CLI の出力・入力 schema に `content_advisory_enabled`、CLI に `lookup_community_node_content_advisories`（Read）を追加して対応表へ登録（154 件）。index entry の出力 schema に `content_advisories` を追加した（#1054 以降、結果を含む CLI 検索が schema で拒否される Existing-gap を同時に解消）。
- 据え置き（Close 条件外）: 10 秒 timeout 後の fail-open 取得、取得ゲート集合が追加のみ、設定保存と照会開始の race、Deferred / token 無しの専用 test、en 参考訳の語の曖昧さ。

## 検証結果

実行結果は PR の検証欄に記録する（`cargo xtask check` / `cargo xtask test` / `cargo xtask cn-check` / `cargo xtask cn-test` / desktop Vitest / Playwright / `cargo xtask oversized-files`）。必須検証を省略して成功とみなさない。

## 引き継ぎ

- C5（#1068）: 本番反映時は CN（lookup route）と client（legal bundle version 6）を同じ回で反映し、再同意を 1 回にまとめる。別 client（表示 OFF）でタイムラインに代替表示が出ること、設定で採用を外すと照会が止まることを確認する。
- 親 #1051: C4 の独立監査結果と監査対象 commit を報告する。
