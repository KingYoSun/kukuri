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
| T1 データ分類・修正前の再現 | 全AC/INVAR、追加AC-9/10 | ADR 0028 §9に仕様を固定。修正前contractは実行後に追記 |
| T2 上限付きephemeral取得 | AC-7、INVAR-3/4、TR-6/7 | 未実装 |
| T3 動画抽出器 | AC-1/3/5/7、INVAR-1/2/4、TR-1/3/5/7 | 未実装 |
| T4 D1専用provider・共有予算 | AC-2/3/5/6/9、INVAR-1/2/4、TR-1/2/3/7 | 未実装 |
| T5 D2共通内容cache・投稿合成 | AC-4/5/6/10、INVAR-2/3/5、TR-3/4/5/6 | 未実装 |
| T6 配備・readiness・運用 | AC-7/8/9、INVAR-1/2/4、TR-5/7 | 未実装 |
| T7 統合・実API検証 | 全AC/INVAR | 未実施 |
| T8 独立監査・CI・merge照合 | 全AC/INVAR | 未実施 |

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
