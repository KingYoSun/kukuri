# #1060 OpenAI Moderation・内容判定再利用・動画フレーム抽出

## 状態と固定範囲

- In progress、区分C、Scope revision `2026-09-17-expanded`。
- 基準commit: `7f13c8821bbc6cdfb164156611293254662ca601`。
- [Issue #1060](https://github.com/kukuri-app/kukuri/issues/1060) の AC-1〜10、INVAR-1〜5、INV-1〜5、TR-1〜7、S1〜S5 を固定する。
- ユーザーは2026-09-17に計画・推奨案を承認し、D1（本文/静止画provider・共有予算）とD2（共通内容hashキャッシュ）も本Issueへ統合した。Issue作業・commit・PR・mergeは追加承認不要。mergeには必須CIと独立監査PASSが必要。
- APIキーは `COMMUNITY_NODE_VLM_API_KEY`。値を記録しない。
- 非対象: 音声検査・文字起こし、全フレーム検査、scene detection、client poster置換、未知CSAM専用検出器、#1061/#1062の対策。

## 作業と証跡

| 作業 | 条件 | 現在の証跡 |
| --- | --- | --- |
| T1 データ分類・再現 | 全AC/INVAR | ADR 0028 §9、boolean閾値・取得上限・抽出時刻のred/greenを記録済み |
| T2 上限付きephemeral取得 | AC-7、INVAR-3/4 | PR #1079で完了。実irohのlocal/remote上限、永続保存0を確認 |
| T3 動画抽出器 | AC-1/3/5/7 | PR #1079で完了。Linux実decode・色分布・cancel cleanup・seccomp |
| T4 D1専用provider・共有予算 | AC-2/3/5/6/9 | providerはPR #1079、PostgreSQL共有予算とproduction resolverは第2PR |
| T5 D2共通内容cache・投稿合成 | AC-4/5/6/10 | 第2PRで実装。memory/PGの再利用・並行・restart・lease fencing・カテゴリ別signal・参照guardのtargeted test成功 |
| T6 配備・readiness・運用 | AC-7/8/9 | 第2PRで実装。operator/CLI test、合成decoder probe、Compose構文、Terraform validate成功 |
| T7 統合・実API検証 | 全AC/INVAR | 全CN suite・production image・benign live計測を実行中 |
| T8 独立監査・CI・merge照合 | 全AC/INVAR | 第1PRはPASS・CI成功・merge済み。第2PRは未完了 |

## 事前調査と計測

- 現行 `build_user_content` はmedia bytesをそのまま `image_url` に包み、動画をdecodeしない。
- `BlobMediaFetcher` のサイズ検査は全量取得後。remote ephemeral取得は永続storeへ書かないが、`.bytes()` による全量確保が先行する。
- 現行再利用はsubject verdictと構成fingerprint。本文はpost state hashであり、別投稿間の本文hash再利用ではない。
- 無害な合成動画をDockerのnetworkなし・read-only・tmpfs環境で抽出済み。FFmpeg 4.4.2、MP4/H.264とWebM/VP9、短尺/12秒音声付き/60秒の6条件で1/3/8枚を生成。512px JPEGは約13〜15 KiB。
- 実APIは単画像HTTP 200、4画像/requestは `too_many_images`（最大1画像）。1frame/requestを採用する。
- これは計画時probeであり、製品extractor・production image・cold/reused E2Eの証跡ではない。キー名変更前に3requestを実施し、生応答・media・credentialは記録していない。

## Surface と sensitive sink

| ID | 登録点・入口 → helper → sink | guard / 検証 |
| --- | --- | --- |
| INV-1 | worker full pass / key変更 / restart → `ingest_object_record` → `scan_or_reuse` | scope、署名、撤回、削除、送信防止を先行。本文も内容hashへ接続 |
| INV-2 | cache miss / retry → provider → bounded fetch / decoder / OpenAI | S1取得、S2process、一時data、S3HTTP。上限・形式・coverage・全件完了 |
| INV-3 | 結果 → `route` / artifacts → verdict / advisory / index | S4/S5。booleanとconfidenceを分離、元subject、一般advisory trust 0 |
| INV-4 | 同内容の別投稿/著者、並行miss → 共通cache → 参照関連付け | S4/S5。完了のみ再利用、構成一致、元author/appealを転写しない |
| INV-5 | resolver / validate-config / readiness / restart / rollback | S2/S3/S4。合成probe、共有予算、credential、decoder依存を確認 |

TR-1 正常cold、TR-2 posterと対象frame/カテゴリ混在、TR-3 部分失敗と復旧、TR-4 別参照/並行/restart、TR-5 構成変更/旧cache、TR-6 mixed scope/撤回、TR-7 timeout/cancel/起動不備を適用。D1/D2では本文/静止画にも同じ遷移を適用し、budget枯渇とretryを確認する。各sinkの逆引きと最終test名は実装と同時に更新する。

## 検証記録

- 変更前 `cargo xtask cn-check`: 成功。
- 必須: `cn-check` / `cn-test` / `cn-e2e`、下位blob変更の `rust-test` と関連media scenario、production image / decoder smoke、Compose/Terraform確認、benign実API、`git diff --check` / `oversized-files`。
- 実装・検証・監査の未実施を成功扱いしない。

### 初期実装の検証（2026-09-17）

- `category_boolean_survives_low_confidence`: 変更前は70閾値でラベルが落ちて失敗、判定方式を型で分離後に成功。`cargo test -p kukuri-cn-safety` 全suite成功。
- `media_fetch_bounds_ingress_before_allocating_whole_blob`: 変更前は無上限/永続取得fallbackが1回で失敗、bounded取得へ切替後は0回で成功。既存source resolutionの6 contractも成功。
- 実irohの `ephemeral_fetch_returns_bytes_without_persisting_them_locally`: local/remoteの2 MiB blobを1 KiB上限で拒否、上限拡大後の回復、受信側durable blob不在を確認。
- 新OpenAI providerの7 contract: 専用endpoint/単画像、booleanとconfidence、coverage欠落、秘密値非掲載、認証障害の非retry、共有予算、動画途中failureと直接動画送信0を確認。production resolver/DB予算への接続は未完了。
- Linux実FFmpeg: MP4/WebM、単一frame/短尺/音声付き/60秒、JPEG上限、全区間の色分布、cancel時のkill/wait/一時data回収、通常disk拒否、seccompによるsocket禁止を確認（unit1 + integration5）。
- 抽出位置の試験で初期fps filterが半区間ずれることを再現。PTSを最初の目標時刻だけ移動して切上げ量子化する方式へ修正し、赤/緑/青の3区間が期待通り3/2/3枚になることを確認。VFRは目標時刻に表示されるframeを採り、最終表示frameを1区間まで延長して単一frame動画にも対応する。
- 最終cn-check/cn-test/cn-e2e、production image、監査、CIは未実施。

## 第1段階の完了

PR [#1079](https://github.com/kukuri-app/kukuri/pull/1079)を `4bc44d40a942339b66e33d23ae0f9373caf11df9` にmerge。
独立監査対象 `be388d9573ccc0f78da8638e0a686c97496cbe34` とmerge treeは差分0。
全必須CI、cn-check/cn-test/rust-test、media関連DM scenarioが成功。監査でAPNGの部分検査完了とWindows samplingのpath依存を発見し、修正後のdelta監査はPASS（12適合/0不適合/0未分類）。
監査全文は [PR comment](https://github.com/kukuri-app/kukuri/pull/1079#issuecomment-5705390610)。

## 第2段階の作業

- D2はnode + 内容hash + 全scan構成のキーで正規化結果だけを共有する。UUID leaseで並行missを束ね、完了writeをownerでfenceする。失敗・不完全・構造破損は再利用しない。
- 同本文の別投稿/別著者、独立service instance、再接続後の再利用をmemoryとPostgresで確認。元author/appealをコピーしない。
- 2カテゴリ検知を1 risk signalへ落とす問題を失敗contractで再現し、カテゴリ別signal/confidence/appeal IDとして記録する処理へ変更。lookupでも両カテゴリを確認。
- 全OpenAI呼び出しにPostgresの同一rolling RPM/RPD/TPM予約を適用する構築経路を追加。別client・競合・restart・日次上限のintegration testが成功。
- 参照guardはscope対応/送信防止、署名済みenvelopeとmaterialized stateの一致、署名内scope、撤回、manifest変更を再確認する。cache待ち、fetch、HTTP/retry、signal/verdict/indexの前に適用する。
- 既存private-channel fixtureがchannel IDを署名へ含めていなかったため、実clientと同じin-channel envelope constructorへ更新した。Unicode上限・ephemeral-only・scope分離のassertionは維持。
- CodeGraphが別worktreeのindexを返したため、この作業treeではrg/readを使用し、新規indexは作成していない。
- Postgresの共有内容cache/lease/budgetの3 integration、runtimeの内容再利用/多カテゴリcontract、既存ingestion/source/advisory/blob-text/reuse contractsが成功。最終全suite・image・E2E・監査は残る。

## 運用上限の確認

2026-09-17、運営者のOpenAI Platform organization/default-project Limitsを読み取り専用で確認。
`omni-moderation-latest` / `omni-moderation-2024-09-26`は500 RPM・10,000 TPM、organizationの当該bucketは10,000 RPD。
設定の変更・保存は行っていない。初期共有枠400 RPM / 8,000 TPM / 8,000 RPDを採用する。
アカウント識別子・費用情報・credentialは本記録へ含めない。

## 第1PRの完了

- [PR #1079](https://github.com/kukuri-app/kukuri/pull/1079) は独立監査PASSと必須CI成功後にmerge。
  監査対象 `be388d9573ccc0f78da8638e0a686c97496cbe34` とmerge `4bc44d40a942339b66e33d23ae0f9373caf11df9` はtree差分0。
- 独立監査でAPNGの単一frame誤認とWindows sampling path依存を検出し、修正後のdeltaもPASS。
- `cn-check`、`cn-test`、`rust-test`、`pairwise_dm_offline_text_image_video_delivery_and_local_delete` は成功。
  第1PRの低層blob/transportの必須検証はここに対応し、第2PRはCN内部・配備のみを変更する。

## 第2PRの境界と検証

構造上の追加は共通内容cache coordinator、PostgreSQL content store/budget、参照guard、専用readiness。
既存VLMは内容純粋性を保証しないため共有内容cacheへopt-inしない。OpenAIとArachnidだけがopt-inする。
旧subject verdictの再利用は維持し、node署名IDもscan fingerprintへ含める。

| ID | 固定入口・全caller group → sink | guard / 対応test |
| --- | --- | --- |
| P2-1 | worker periodic/restore/key event → `ingest_scope` / `ingest_changed_keys` → `ingest_object_record` | current supported scope、署名envelopeとstate/manifest、withdrawal、送信防止。`reference_guard_contracts` |
| P2-2 | ingest → `scan_or_reuse_guarded` → subject reuse / content coordinator | lookup前・関連付け前にguard。`verdict_reuse_contracts`、`reference_guard_contracts` |
| P2-3 | service `scan_or_reuse` / `scan_and_record` / `scan_and_record_for_author` → orchestrator / recording | 未公開の内部service入口。ingestはguard付き入口のみ。既存service/orchestrator/appeal contracts |
| P2-4 | coordinator → ContentScanStore load/claim/complete/release | 完了provider/capability/coverage構成、owner fencing、320秒lease。`content_cache`、`moderation_content` |
| P2-5 | OpenAI provider scan/scan_guarded → bounded fetch → decoder → shared client | queue前のbytes取得なし、fetch後・各frame/retry直前の参照guard。`provider_contract` |
| P2-6 | OpenAI client moderate/moderate_guarded → PgModerationBudget → HTTP send | 本文・静止画・動画・probe・retryの全attemptを同一DB予算で予約。`moderation_content` atomic budget |
| P2-7 | report/再利用 → `record_signals` → signal/event/verdict/advisory/author persistence | 内容cacheにauthor/scope/appealを含めない。複数categoryは個別signal。`content_cache`、PG advisory test |
| P2-8 | ingest → index truth store → projection / de-index | association/index直前にもguard。不正stateもkey identityの旧rowを除去。`reference_guard_contracts`、既存ingestion contracts |
| P2-9 | runtime / validate-config → `resolve_safety_providers_with_pool` | general専用・key必須・Linux decoder/tmpfs必須。legacy resolverとmock production拒否を維持 |
| P2-10 | CLI readiness → PreparedProbe / ReadinessProbeRecord | 同梱合成MP4/WebM+本文+JPEG。config変更、future/stale時刻、key/decoder欠落でPASS再利用不可。`readiness_reuse_requires_current_configuration_and_time` |
| P2-11 | operator validation/docs/tfvars → Terraform env/Compose → runtime | 既存secret ID、Tier 1上限、第三者送信開示、tmpfs。`dedicated_moderation_deploy_requires_secret_and_general_slot`、Compose/Terraform |
| P2-12 | provider metrics → IndexerRuntimeState → status snapshot | 内容を含まないcounterのみ。restartでcounter reset、予算/cacheはDB維持。live probeのcold/reused差分 |

S1 bounded fetchはP2-1/2/5、S2 processはP2-5/10、S3 HTTPはP2-6/10、S4 DBはP2-2/3/4/6/7/10、
S5 index/advisoryはP2-7/8へ逆引きする。登録表・trait実装・`scan*` / `moderate*` / `persist*` callerを突合し、追加surfaceの未分類0。
独立監査は固定headでこの対応を再構築する。

- TR-1/2: 実FFmpegとmock provider、複数カテゴリのconfidence、本文＋blob/thumbnailの和集合は既存/追加contract。
- TR-3: frame途中失敗、401/403、429/retry、cache failure→回復、cancel→claim解放を確認。
- TR-4: 異なる投稿・著者・service並行miss、同内容のrestart再利用、expired ownerの上書き拒否をmemory/PGで確認。
- TR-5: provider構成/issuerの変更とcorrupt cache、readiness構成・時刻変更を確認。
- TR-6: unsupported scope、scan中の送信防止、署名と異なるstate、別の有効参照を確認。
- TR-7: tmpfs/decoder起動制約、実decode cancel、socket拒否は第1PR。合成readiness decodeを第2PRで追加。

再現と修正:

- `issuer_change_invalidates_subject_and_content_reuse` は修正前に2回目provider呼出0で失敗。
  node署名IDをsubjectとcontent共通の構成fingerprintへ入れ、1回で成功。
- malformed stateを再取り込みすると旧index rowが残る失敗を再現。parse失敗をper-entry de-indexへ接続し、
  正しく型付けした署名不一致stateとcorrupt stateの両方で削除を確認。object内の偽IDを削除対象に使わない。
- 2カテゴリ結果のsignalが1件に落ちる失敗を再現し、カテゴリ別のID・confidence・appeal状態を保存するよう修正。
- 通常CIで `live_moderation` はgate未設定のため実API未実行。live結果は別途記録し、mock/skipを実API成功と数えない。

実projectのread-only確認（2026-09-17）ではomni-moderationの500 RPM / 10000 TPM、organization bucketの10000 RPDを確認。
設定変更は行わず、project ID・請求情報・資格情報は記録しない。初期local予算は400 / 8000 / 8000を維持。
