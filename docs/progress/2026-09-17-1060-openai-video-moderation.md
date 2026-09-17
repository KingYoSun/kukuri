# #1060 OpenAI Moderation・内容判定再利用・動画フレーム抽出

## 状態と固定範囲

- 実装・ローカル検証完了。区分C、Scope revision `2026-09-17-expanded`。最終headの監査・CI・merge照合と現在判定は [PR #1080](https://github.com/kukuri-app/kukuri/pull/1080) / [Issue #1060](https://github.com/kukuri-app/kukuri/issues/1060) の記録を参照。
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
| T7 統合・実API検証 | 全AC/INVAR | cn-check / cn-test / cn-e2e成功。production imageのbenign live16requestと再利用時I/O 0を記録 |
| T8 独立監査・CI・merge照合 | 全AC/INVAR | 第1PRはPASS・CI成功・merge済み。第2PRの固定head監査・CI・merge照合はPR #1080の記録に集約 |

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

### 初期実装時点の検証（2026-09-17、後続で完了した項目は上記表を参照）

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
| P2-5 | OpenAI/Arachnid/VLM provider scan/scan_guarded → bounded fetch → decoder → shared client | queue前のbytes取得なし、fetch後・各frame/retry直前の参照guard。`provider_contract` |
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

## 第2PRの追加検証・監査対応

- [PR #1080](https://github.com/kukuri-app/kukuri/pull/1080) を作成し、固定head `28110c2c` の独立監査を実施。
- 監査でArachnid/VLMのfetch中失効後もHTTPが1回送られる失敗を再現。
  `scan_guarded` を各providerに実装し、取得前と送信直前のguardを共通処理で再確認する。
  `provider_reference_guards::{arachnid,vlm}_rechecks_guard_after_media_fetch` は修正後HTTP 0で成功。
  P2-5の逆引きcaller groupへ両providerを明記した。critical/known-matchの判定方式は変更しない。
- Arachnid endpoint変更でfingerprintが不変となる失敗も再現。非秘密のendpoint・timeout・credential環境変数名と
  判定versionをfingerprintへ追加し、`arachnid_endpoint_change_invalidates_content_identity` が成功。
- `ReadinessProbeRecord` のDB row型はclippy指摘に従って型aliasへ整理。
- operator configは既存validation入口への必須配線31行追加のため大型baselineを更新する。
  新しいmoderation値の検証本体は独立moduleへ置き、無関係な構造整理は混ぜない。ratchet gate自体は維持する。

### Production imageと無害な実API

[内容を含まない計測JSON](2026-09-17-1060-live-moderation.json) を参照。
Dockerfileからcn-cli release imageをbuildし、同runtimeのFFmpeg/ffprobeとtmpfsを使用。
第2PRのintegration test binaryをread-only mountし、実PostgreSQLとOpenAIへ接続した。
source replica/blobと検索投影はメモリ内、known-hashはbenign専用doubleであり、実ArachnidやP2Pネットワークの検証とは区別する。
既存Arachnid/P2Pを含む全構成検証は `cn-e2e` が所有する。

| fixture | frames | cold時間 | decode時間 | OpenAI attempts |
| --- | --- | --- | --- | --- |
| MP4 1frame | 1 | 3,725 ms | 106 ms | 2（本文1 + frame1） |
| WebM 1frame | 1 | 1,490 ms | 91 ms | 1 |
| MP4 12秒・合成音声付き | 3 | 59,397 ms | 137 ms | 3 |
| MP4 60秒 | 8 | 177,506 ms | 282 ms | 8 |

cold計14request。長尺の時間にはTPM予約待ちを含み、全体deadline 300秒以内。
serviceを破棄して再構築し、別著者の新投稿で全4blobを再利用した。
各投稿はindex可、本文/動画の再利用2件、元blob fetch・decode・APIはすべて0、時間は159/69/62/63ms。
データベースの再接続・service再構築を伴う再利用であり、OS再起動を実施した証拠ではない。

さらにproduction cn-cli自体のreadinessを専用DBで実行し、同梱MP4/WebM decodeとbenign本文/JPEGの2requestに成功。
provider記録はPASS、構成fingerprintあり。公開ノードの周辺サービスを用意していないため全体readinessはFAILのまま。
キーを渡さず期限内に再実行すると旧provider PASSは再利用されずFAIL、API予算予約は2件のままで追加0。
合計16requestに秘密値・利用者投稿・実在人物のmediaを含めない。

### CIの既存認証設定の修復

Terraform CIは実装のplan前に、GCP WIFが移転前 `KingYoSun/kukuri` のみを信頼しているため失敗した。
GitHub APIで旧URL/現URLのrepository IDがともに `1025894008` であることを確認し、providerのconditionと
CI service accountの既存 `roles/iam.workloadIdentityUser` principalを `kukuri-app/kukuri` へ置換した。
他のbinding/roleは変更せず、旧名の許可は残さない。VM apply・本番media送信は行っていない。
runbookの移転時の手順も同期した。CIの再実行で認証とplanの成功を確認してからmergeする。

## 最終ローカルvalidation

- `cn-check`（全CN crate / all-targets clippy、warningsをerror扱い）: 成功。
- `cn-test`（全CN crateと実PostgreSQL/Valkeyのintegrationを有効化）: 成功、最終実行3分16秒。
- `cn-e2e`（実PostgreSQL/Valkey/ArcadeDB/irohを使う既存全構成 + provider guard回帰）: 13件成功、最終実行1分30秒。
- Linuxの実FFmpeg抽出、静止画/動画OpenAI contracts、PG content cache/lease/budget、CLI/operator/readiness、query/worker、
  原則のgeneral trust 0・appeal訂正保護を上記suiteで確認した。
- production image build / benign live / real CLI probe、Compose config、Terraform fmt/validate、Terraform CI plan: 成功。
- `cargo fmt --all -- --check` / `git diff --check` / `oversized-files`: 成功。
- 下位blob変更の `rust-test` とmedia scenarioはPR #1079で成功し、第2PRではそのpathを変更していない。

全suiteで明らかになったfixture/互換差分も解消した。

1. `query_contracts` / `worker_contracts` のprivate fixtureがPublic・channel_idなしのenvelopeを使っていたため、
   実clientと同じPrivate・channel_id付き署名を作るよう更新。検索件数・失効時de-index等のassertionは維持。
2. `objects/not-a-post/state` 等の他domainは従来どおりIgnoredを維持する。投稿入口は生成・署名検証契約と同じ64hex ID。
   その実post keyにあるcorrupt値・偽object_idは、key側のIDで旧indexをde-indexする。
   `reference_guard_contracts`、`source_resolution_contracts`、`query_contracts` の15件が成功し、独立差分監査でもblocker 0。
3. 監査用testの `unwrap` は理由付き `expect` へ変更してclippy規約を満たした。
   大型baselineはconfig.rsの必要な31行追加を記録し、既存の縮小分も生成commandで下げた。

検証環境の失敗は製品成功と分離した。WSLでWindows worktreeのgitdirを読む際はgit.exeのwrapperを使い、
Docker credential helperを含むPATHを保持して専用Compose project/portを起動した。最初の環境不備やfixture失敗を成功扱いせず、
修正後に全suiteを完走した。WSLのtargetは再起動で消える `/tmp` から検証用cacheへ移した。

## CIで検出したdecoder設定差分

`7bb25896` のlinux-cnで、FFmpeg 6.1.1の通常fixture抽出がnonzero終了した。
Ubuntu 24.04でCPU数64を模した実decodeも失敗したため、入力decoderのthread自動選択を調査。
元の `-threads 1` は `-i` より後にあり出力JPEG側にしか適用されていなかった。
入力既定を64threadにした `input_decoder_threads_are_bounded` を追加すると、修正前は
`Resource temporarily unavailable` で失敗。ffprobeと `-i` 前の入力decoderも1threadへ固定後は
同じ512 MiB制限でMP4/WebMの60秒・8frameが成功した。

- メモリ・CPU時間・出力サイズ・network sandboxの上限は緩めない。
- 抽出設定の変更をcacheへ反映するため `video-midpoints-v2` に更新する。
- Ubuntu 22.04の全video suite（unit1 + integration7）と、CPU数64を模したUbuntu 24.04のintegration7が成功。
- production Debian runtimeでもPythonを必要とするcancel test以外の6件が成功。
  cancel/network境界はLinuxの上記suiteが担い、test skipをその成功の代わりにしない。
- このdecoder deltaの全CN検証、実API計測との照合、独立監査、CIを改めて確認してからmergeする。

### decoder修正後の再検証

`video-midpoints-v2` の `cn-check` / `cn-test` / `cn-e2e` / oversized はすべて再成功。
同じproduction Debian runtimeでv2のtest binaryを用い、実API cold14requestと再利用を再測定した。
MP4単一frame 3,241ms、WebM単一frame 932ms、音声付き12秒59,744ms、60秒177,982ms。
別著者・service再構築後の4件は67/69/70/64msで、元blob fetch・decode・API追加はすべて0。
計測JSONの `decoder_v2_recheck` に結果と関連sourceのSHA256を記録し、最初の計測も履歴として保持した。
初回readiness実API probeのclient/credential処理には変更がなく、v2の同梱decoder probeも実行済み。
本段階の実APIは初回16 + v2再検証14 = 30request（計画時の3requestは別の履歴）。

同じCIで既存 `theme-palette.spec.ts` の色判定も失敗したが、desktopの製品コードとtestに今回のdiffはない。
fixture/期待値の変更やskipで回避せず、更新headの通常CIでもう一度確認する。

### 補助CIテストの入力状態を固定

必須CIの `theme-palette.spec.ts`（390px）では、composerをclick→Escapeで閉じた後、
ポインタがprimary button上に残り、通常色 `#d77d45` の検査にhover色 `#c86f38` が混ざっていた。
製品CSSはDESIGNのdefault/hover契約どおりであり、このIssueのdesktop製品差分は0。
CIと同じhover状態に固定して遷移完了を待つと、元の通常色assertionが同じ値で失敗することを再現した。
CI保守として検査前にポインタを外し、`:hover` がfalseである前提を明示してから既存のkeyboard focusの検査を行う。
色・outline・draft・Column維持のassertion、timeout、snapshot baselineは変更しない。

- 変更はtest前提のみ（製品挙動・layout/tokenの変更はない）。UI review recordやTauri固有の実機確認は非該当。
- Playwright Chromiumの1600px / 390px、light/dark、invalid保存値の3caseを3回ずつ、計9件成功（22.3秒）。
- 対象specのESLintとfrontend TypeScript型検査も成功。
- 最終headの通常CIでも全browser / visualレーンを確認する。
