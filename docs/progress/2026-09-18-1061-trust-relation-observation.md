# #1061 trust 絶対値と閲覧者別 relation 値の合算・ブロック/ミュート観測の作業記録

## 現在の状態

- リスク区分 C、Scope revision: 2026-09-15-r2、基準 commit: `d5e6806a`（PR1 の分岐元）。
- 2026-09-17 にユーザーが実装計画を承認した。3 PR を順に merge する（stacked にしない）。各 PR の head で
  独立監査を行い、Issue は PR3 の merge 後確認を終えてから Close する。
- AC / INVAR、固定 inventory（INV-1〜6）と状態遷移（TR-1〜8）の正本は
  [Issue #1061](https://github.com/kukuri-app/kukuri/issues/1061)。
- 本番反映は別 Issue にまとめる。本 Issue の Close 条件に本番確認は含めない。

| PR | 範囲 | 状態 |
| --- | --- | --- |
| PR1 | ADR 0026 §8 / ADR 0022 追補、観測 envelope、CN の観測受付・取消・保持、T/R の合算と一括評価、wire 追従 | merge 済み（[#1125](https://github.com/kukuri-app/kukuri/pull/1125)、`1599ff5a`） |
| PR2 | client の提供トグル（任意文書への同意）・送信待ち・取消 | merge 済み（[#1129](https://github.com/kukuri-app/kukuri/pull/1129)、`a35604d4`） |
| PR3 | CN の採用順位、6 経路と引用元の折りたたみ・再表示・著者例外 | [#1130](https://github.com/kukuri-app/kukuri/pull/1130) |

## 利用者決定（2026-09-17）

| 項目 | 決定 |
| --- | --- |
| 観測提供の同意 | CN の同意カタログの任意文書（slug `trust_observation_sharing`、`required: false`）に含め、client は CN ごとのトグルでその文書への同意・取消を行う |
| 既存分 | 有効化時に「既存のブロック/ミュートも送る」を選べる（既定は送らない） |
| 表示 | 6 経路で投稿を折りたたみ、理由・採用 CN・「表示する」を出す。作者詳細で常に表示する例外を設定・解除できる |
| PR 分割 | 3 PR を順に merge |

## 修正前の状態（基準 commit）

- `trust_user_read` は `UniformRelationWeight(1.0)` で T だけを返し、閲覧者で値が変わらない。
- CN に block / mute を受け付ける route・保存先が無い（`/v1/trust/observations` は 404）。
- `RelationStore` に複数 candidate の proximity を読む口が無い。
- 同意カタログに観測提供の文書種別が無い。

## PR1 の対応

| 条件 | 実装 | 検証 |
| --- | --- | --- |
| AC-1 / INVAR-1 / INVAR-3 | ADR 0026 §8.3 / §8.5・ADR 0022 追補。`kukuri-core` の `mute-observation` envelope と `parse_trust_observation`（署名・id・署名者 = subject を検証、block-edge を block 観測として受理）。`cn-core` migration `202609180001_trust_observations.sql`（観測・対象 revision・任意同意の取消）。受付は bearer = 署名者・必須同意・任意文書の現行版同意を observer 単位の advisory lock 下で確認してから保存。取消は本人認証だけで全削除 | core unit 4 件、`observation_intake_requires_matching_signer_and_sharing_consent`（拒否時の行数 0）、`observation_revocation_deletes_rows_and_blocks_intake` |
| AC-1（任意文書） | `cn-operator` の `LegalDocumentKind::TrustObservationSharing`（`ALL` に含めない、slug 固定、required 禁止）と本文生成。operator runbook に設定例 | `trust_observation_sharing_document_is_optional_and_published_only_when_configured`、`trust_observation_sharing_document_must_be_optional_with_fixed_slug`、`sharing_policy_slug_matches_operator_catalog` |
| AC-2 / TR-2 / TR-6 | `cn-trust::compose_relation_adjustment`（proximity 重み・下限 0.1・block 1.0 / mute 0.5 の max・半減期・上位 5 件の noisy-OR）。`(observed_at_ms, envelope_id)` が新しい観測だけ採用、未来時刻は拒否、保持 active 180 日 / revoked 30 日 | `cn-trust/tests/relation_adjustment.rs` 13 件、`observation_upsert_is_idempotent_and_ignores_stale`、`observation_retention_purges_expired` |
| AC-6 / AC-7 / INVAR-2 / INVAR-5 / TR-7 / TR-8 | T = 従来の `build_trust_read`（一様重み）、R は観測と proximity だけから算出、`apply_viewer_relation` が S = clamp(T + R) と `evaluation`（policy / trust / relation 版、期限、`hide_recommended`、理由）を付ける。`GET /v1/trust/users/{pubkey}` の `trust` を S に変更し、`POST /v1/trust/evaluations`（最大 100 件）を追加。`RelationStore::proximity_scores` を追加 | `trust_read_sums_absolute_and_viewer_relation`（A/C 比較、T 共通、解除で復帰、同意取消で除外、observer 非開示、pull 不変）、`relation_proximity_scores_match_pairwise_reads`、ArcadeDB の `arcadedb_relation_store_satisfies_shared_contracts` |
| AC-6（wire） | `TrustReadView.evaluation`（旧応答互換）、`TrustEvaluation*` 型、path / 安定コード。desktop-runtime の TS 生成、CLI の JSON schema、harness mock、`CommunityNodeAdvisoryPanel` は S を主値にし、内訳を「ノード共通の評価の内訳」として分けた | `trust_evaluation_wire_contract_is_optional_and_carries_no_observer`、`community_node_trust_relation_client_preserves_wire_contract_and_methods`、`CommunityNodeAdvisoryPanel.test.tsx` の 2 件 |

## 既知の制約（ADR 0026 §8 に記録）

- relation snapshot は `relation analyze` の上書き更新を版として使う。解析中の read で更新前後の proximity が混在しうる。
- revoked 観測は 30 日で削除するため、それより古い active envelope の再送は復活しうる。client の送信待ちは
  対象ごとに最新 1 件へ集約するので、通常の再送では起きない。
- 1 対象あたり評価に使う active 観測は新しい順に 200 件まで。

## PR2 への引き継ぎ（PR1 独立監査の指摘）

- 既存の同意ダイアログは提示した文書をすべてローカル同意記録へ書く（`useCommunityNodeConsentFlow.ts` →
  `record_community_node_local_consents`）。サーバ同期は必須文書だけ（`policy_slugs = []`）なので現状は外部へ効かないが、
  PR2 ではこのローカル記録を提供トグルの状態として扱わず、一括受諾の対象から任意文書を外す。

## PR2 の対応

| 条件 | 実装 | 検証 |
| --- | --- | --- |
| AC-1 / INVAR-1 / INV-1 / TR-1 | `trust_observation_support.rs`: 提供状態と送信待ちを `<db>.trust-observations.json`（account ごと）に保存。mute / block 操作の後に CN × 対象 × 種別で最新 1 件へ集約して積み、scheduler の 1 tick で session が Ready のときだけ送る。有効化は任意文書の版照合 → `POST /v1/consents`（slug 指定）→ ローカル記録 →（選んだ場合）既存分の署名。一括同意からは任意文書を除く | `observation_not_sent_without_sharing_consent`、`existing_mutes_are_sent_only_when_opted_in`、`general_consent_acceptance_excludes_sharing_document`、`not_offered_node_cannot_be_enabled` |
| AC-5 / TR-2 / TR-3 | 送信失敗・403（同意切れ）・404（未公開）・400 の分類。403 / 404 では送信待ちを破棄して削除を要求し、送信済みの観測も CN に残さない。無効化・同意取消・CN 削除で送信待ちを破棄して削除を要求し、完了まで新しい観測を送らない。401 は 1 回だけ再認証 | `outbox_coalesces_and_resumes_after_restart`、`revocation_pending_blocks_new_posts`、`sharing_consent_required_revokes_and_stops_sending_until_reenabled`、`reauth_once_on_401`、`local_mute_succeeds_when_cn_unreachable`、`consent_withdrawal_and_node_removal_request_observation_deletion` |
| AC-1（UI・IPC） | Tauri / CLI の 3 command（取得・有効化・無効化）と parity、TS 型、設定画面の `CommunityNodeObservationSharingField`（文書本文と「既存分も送る」チェック、再同意・削除要求中の案内） | `CommunityNodePanel.observationSharing.test.tsx`、CLI の `command_parity` |
| 外部送信表示 | 送信契機に「ブロック・ミュートの提供」を補記し、データフロー突合表に行を追加（版は PR3 でまとめて上げる。2026-09-18 のユーザー判断） | Tauri の法務 bundle テスト |

## PR2 の残課題（独立監査の non-blocker）

- `reconsent_does_not_send_observations_that_no_longer_match_local_state` は、提供が止まる全経路で送信待ちを
  破棄するようになったため、有効化時の突き合わせ（`pending` の retain）自体は到達しない多重防御になっている。
  将来どれかの経路が破棄をやめても気づけるよう、状態を直接作って突き合わせだけを検証する test を足す余地がある。
- 再同意の案内文は、止まった時点で CN 側の記録を削除したことに触れていない。

## PR3 の対応

| 条件 | 実装 | 検証 |
| --- | --- | --- |
| AC-3 / AC-6 / AC-7 / INV-4 / TR-4 / TR-5 / TR-7 | `CommunityNodeConfig.trust_node_priority`（設定済み node に正規化、空なら機能オフ）と `trust_gate_support.rs` の `evaluate_author_trust_gates`。優先順に一括評価を読み、viewer・対象・期限を照合して最初の有効値を採る。失敗・401・期限切れは次の選択済み node へ進み、全滅なら未評価（折りたたまない）。cache は (node, target) 単位で、設定・同意・認証の変更で世代を進めて捨てる | `trust_gates.rs` 7 件（優先順・未選択 0 件・失敗 fallback・viewer/期限の照合・cache と設定変更・restart 復元・social state 不変） |
| AC-4 / INVAR-1 / INVAR-4 / INV-5 / INV-6 | `resolvePostTrustGate`（著者と引用元）、`AuthorTrustGateNotice`（理由・採用 CN・表示する・作者を開く）、live / game 一覧の同じ案内、作者詳細の「この作者を常に表示する」、設定画面の採用順位 UI | `DesktopShellPage.authorTrustGate.test.tsx` 4 件、`authorTrustGates.test.ts` 3 件、`CommunityNodeTrustPriorityField.test.tsx` 3 件、[ui-review record](../ui-reviews/2026-09-18-1061-author-trust-gate.md) |
| AC-5 | 判断は評価の期限まで使い、期限切れは照会し直して応答で差し替える（応答までは前の判断のままで、折りたたんだ投稿を一瞬開かせない）。照会に失敗したら判断を捨てる（fail-open）。node の状態を読み終えるまで照会しない。採用順位・認証・必須同意が変わったら全部捨てる。作者ごとの例外は設定・解除の時点で表示へ反映する | `useAuthorTrustGateLookup.test.tsx` 6 件、`DesktopShellPage.authorTrustGate.test.tsx` の例外 test |
| 法務 | legal bundle を version 7 へ（利用規約 第3条に第 5・6 項、プライバシーポリシーと外部送信表示に「信頼評価の照会」「ブロック・ミュートの提供」の送信項目・送らない情報・取消時の削除要求、データフロー突合表に行を追加、i18n 本文とミラーと同意 fixture を同期）。2026-09-18 のユーザー判断どおり PR2 分と合わせて 1 回で上げる | Tauri の法務 bundle テスト（必須句に version 7 分を追加）、`App.test.tsx`、Playwright の同意 fixture |

### PR3 独立監査（commit `ed71c6ec`）の指摘と対応

| 指摘 | 対応 |
| --- | --- |
| Blocker: legal bundle version 7 の本文が changeSummary の主張と一致しない（プライバシーポリシー本文と i18n の `sections` が未更新） | `privacy-policy.md` の Community Node 節へ信頼評価の照会とブロック・ミュートの提供を追記し変更履歴を追加。3 locale の `documents.terms.sections` / `documents.privacy.sections` を更新。`state.rs` の必須句へ version 7 分を追加して本文未更新を CI で検知できるようにした |
| Blocker: クライアント側の判断が `expires_at` も同意取消も反映せず、session 中は無期限に再利用される | `useAuthorTrustGateLookup` が判断ごとに期限を持ち、期限切れは store から捨てて照会し直す。採用順位に加えて認証・必須同意の状態も作り直しの契機にした |
| Major: 作者詳細の「常に表示する」が表示中の投稿へ反映されない | 例外の設定・解除で返る判断を `authorTrustGates` へ書き戻す。test に折りたたみが解ける assert を追加 |
| Major: debounce timer が cleanup 後に再設定されず、照会が丸ごと落ちうる | 待ち行列が残っていれば張り直す条件に変更し、再現 test を追加 |
| Minor: 新規再利用 component に Story が無い | `AuthorTrustGateNotice` / `AuthorTrustDisplayExceptionField` / `CommunityNodeTrustPriorityField` の全 state Story を追加し、ui-review record の Preview を差し替えた |
| Minor: 外部送信表示の変更履歴で v6 補記と v7 の記述が重複 | v7 を先頭へ移し、重複記述を落とした |
| Minor: 採用順位の保存が編集中の下書きを送る | 保存済みのノード一覧を送り、保存後に下書きを再同期する |
| Minor: `trust_node_priority` の 1 件でも URL として読めないと runtime 起動が失敗する | 表示設定なので読めない値は落とすだけにした（`normalize_trust_node_priority` は `Result` を返さない） |
| Nit: game 一覧の案内に作者導線が無い / 100 件超の切り捨てが無記載 / 末尾の余分な空行 | いずれも修正した |

### PR3 差分再監査（`ed71c6ec` → `ee9a34a9`）の指摘と対応

1 回目の指摘はすべて解消と確認。追加で次を直した。

| 指摘 | 対応 |
| --- | --- |
| Blocker: `cargo fmt --check` が落ちる | `cargo fmt` を適用した |
| Minor: 期限切れの掃除で折りたたみが一瞬解け、本文が数百 ms 見える | 期限切れでは判断を捨てず、照会の応答（または失敗）で差し替えるようにした |
| Minor: 起動直後に node の状態が確定して判断を一度捨て、同じ著者を二度送る | node の状態を読み終えるまで照会しないようにした（`statusesLoaded`） |
| Minor: 外部送信表示と i18n が live / game 主催者の公開鍵を挙げていない（過小記載） | 表・変更履歴・3 locale の本文に加え、プライバシーポリシーの文言も主催者と投稿の作成者を分けた |
| Minor: 優先順位の fail-soft に test も log も無い | 破損した設定からの復元 test を追加し、落とした値を `warn!` で残すようにした |
| Nit: 英語本文だけ curly apostrophe / 版の帰属のズレ | ASCII に統一し、同意分類表の version 7 記述を外部送信表示の扱いに揃えた |

## 検証記録

- PR1（[#1125](https://github.com/kukuri-app/kukuri/pull/1125)）: 実行結果は PR 本文に記録する。独立監査（commit `6fbab3e4`）は
  PASS・blocker 0。non-blocker のうち ADR の一括評価 path の記載違いは同 PR で修正した。
