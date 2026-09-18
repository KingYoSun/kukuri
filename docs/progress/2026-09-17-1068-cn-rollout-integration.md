# #1068 CN 反映と content advisory の統合確認（#1090・#1093 を含む）

## 現在の状態

- 本番 VM を main `ac9f93ce`（#1051 C2〜C4、#1065、#1090、#1091 を含む）の CN image へ更新し、2026-09-17 に実機確認を行った。
- Issue は 2026-09-17 r2 で、#1090（一時的な取込み失敗で索引済み投稿を保持）と #1093（#1091 の本番確認）を同じ反映へ加えた。
- 一般 moderation は同日の v0.2.5 反映で OpenAI Moderation（`omni-moderation-latest`）へ切替済み（[v0.2.5 の記録](2026-09-17-v0.2.5-preview.1-release-rollout.md)）。
- 確認中に見つかった問題は #1097 / #1105 / #1106 / #1107 / #1108 / #1109 に起票した。AC-6 は #1107 の不具合により未達。
- 2026-09-18 に v0.2.6-preview.1（#1105 / #1107 / #1108 / #1109 の修正を含む）の反映後に AC-6 を再確認した（下記「v0.2.6 での再確認」）。表示は達成したが、表示 OFF の client が advisory 付き画像の bytes を projection 反映の経路で取得・永続化しており、取得ゲートは未達（#1152）。

## 反映

| image（ghcr.io/kukuri-app配下） | 配置digest（`sha-ac9f93ce…` = `latest`） |
| --- | --- |
| kukuri-cn-user-api | sha256:083118e6506ab5c388295bdfd4137c3ca162081d66db4dcf80f56202a3d7dd2f |
| kukuri-cn-iroh-relay | sha256:5a233dfa5cf0632ef6bc4b698bdb51cc0c96f10f7c608bb304d5bb2d84d8d796 |
| kukuri-cn-cli | sha256:2c085c04af4ec908f57adf1d182099f8537be5c5eb80278ab6a2a4d19861a248 |
| kukuri-cn-indexer | sha256:e78fbc6926ccef611e1aee89106a9a0ebbfd1cc2f92271b5e0511aba7993f2aa |

- main の CN image run [35196002318](https://github.com/kukuri-app/kukuri/actions/runs/35196002318) と Fast [35196002302](https://github.com/kukuri-app/kukuri/actions/runs/35196002302) が成功。sha tag と latest の digest が一致し、linux/amd64、revision が source と一致。
- rollback 先は v0.2.5-preview.3 の 4 refs（[v0.2.5 の記録](2026-09-17-v0.2.5-preview.1-release-rollout.md)）。空き容量の確保のため、旧構成 revision `1b3aaddc` の 4 images を、不使用と再取得可能性を確認してから削除した（空き 5.9GB→12GB、新 images 取得後 5.0GB）。
- 新 CLI で導出した node ID は一致。更新前に、活性重複鍵の risk signal が 0 行であることを確認した。
- バックアップ `postgres/2026/09/17/083921.dump`、generation `1789634363633458`、574,364 bytes。
- operator-config／非公開 tfvars の差分は 4 image refs だけ。candidate plan の replacement 理由は `metadata_startup_script` だけで、展開後の差分は 4 image refs と deployment revision だけ。startup desired／metadata SHA-256: `97b9d3365c9a3be85f99400a31fa264ec94e7825dda77dbd6a454bca62563579`。safe plan・適用・最終 plan とも No changes。
- 2026-09-17 08:41:15 UTC に bootstrap complete。4 containers（migrate は exit 0）の revision が一致し、API／indexer healthy。

## AC ごとの証跡

| AC | 結果 | 証跡 |
| --- | --- | --- |
| AC-1 | 達成 | `_sqlx_migrations` に `202609160001`（v0.2.5-preview.1 反映時に適用）。`cn_safety.scan_verdicts.advisory_labels` 列があり、反映時点の既存27行は `[]`。migrate exit 0 |
| AC-2 | 達成 | 19:37 JST の明示的な成人向け画像投稿 `3d8d3cb1…` の verdict が `allow`・`2026-09-public-node-v3`、`advisory_labels` に blob `f6a38ae2…` の `adult`（nsfw、confidence 100）と `sensitive`（objectionable、confidence 35）。blob verdict は `general_moderation` の `allow`。対応する `cn_index.index_entries` 行あり（10:38:04 UTC）。旧 `exclude` の 2 件（`6f0b…`、`4956…`）は再 scan で `allow`・advisory なしで索引へ戻った |
| AC-3 | 達成（client 表示で確認） | 運営者の client で、当該投稿に「api.kukuri.app による推定、性的表現の可能性、確信度 100、根拠 自動分類のスコア、異議を申し立てる」が表示された。wire の直接照会は認証が必要なため、client 経由の確認とした |
| AC-4 | 達成（client 表示で確認） | 投稿者の trust 情報で総合 0.000、絶対成分 0.000、相対成分 0.000、重み 1.000。basis に nsfw（low ×1、high ×2）と objectionable（low ×1）が並び、trust の値は投稿の前後で変化なし（運営者確認） |
| AC-5 | 達成 | 新規 signal は blob `f6a38ae2…` の nsfw と objectionable の 2 件（`severity=low`、`basis=classifier_score`）で、鍵ごとに 1 件。10:40:34 UTC の risk_signals 4 件・signed_moderation_events 612 件・index_entries 21 件は、general の 2 pass（10:42、10:47、いずれも `scans_fresh=0`）後の 10:49:27 UTC でも同数 |
| AC-6 | 未達（#1107） | advisory 付き投稿はタイムラインで成人向け代替表示になり、OFF のままスクロールした古い投稿も代替表示になった。一方、ON→OFF 切替時の表示済み画像のちらつき、表示 OFF のままでの不定期なちらつき（ON にすると止まる）、通常画像投稿のスケルトン残留を確認した。ちらつく投稿 `d16efa12…`（advisory なし）は、旧 signal が残る `6f0b…` / `4956…` と同じ blob `4c8fd3bf…` を添付しており、投稿単位の advisory による blob のゲートと、advisory なしの投稿の表示が入れ替わっていると推定した。代替表示の UI 改善要望は #1108 |
| AC-7 | 達成（Linux）。Windows は #1105 | Linux client は更新後に legal bundle version 6 の再同意を求め、CN の文書（外部送信・moderation-policy version 2、snapshot `391a5f54…`）の再同意も 1 回で完了した。Windows は、開発ビルドが同じ app data に version 6 の同意（2026-09-16 16:54 UTC、app_version 0.2.4）を記録済みのため再提示されなかった。同意画面の表示改善は #1106 |
| AC-8 | 本記録 | |
| AC-9 | 達成 | 19:22 JST の本文投稿 `ff0f9d00…` は変更通知で 1 件だけ取り込まれた（`scanned=1 indexed=1 scans_fresh=1`）。添付付き投稿（19:23、19:37）は `changed keys are not object-scoped … reason=manifests/media` で scope 全体へ倒れた（runbook §5.6 の仕様どおり）。本文だけの投稿で `not object-scoped` の log は出ていない |
| AC-10（#1090） | 達成 | 反映直後は general 1 件、test 0 件。投稿者の client が接続すると 10:22 UTC 以降に general 15 件、test 5 件へ回復し、その後の pass で減少なし（10:40 時点で合計 21 件）。`deindexed=0`、`failed to resolve post body` 0 件。`temporarily failed to ingest object record` は client 接続前の 25 件（本文取得不可で、索引済み entry を保持） |
| AC-11（#1093） | 達成 | decoder を事前起動せず、image 取得直後の初回 `readiness --force-probe` で `ready=true fail=0 unknown=0`。general は「MP4/WebM decode、認証、本文/画像応答の解析に成功」。`readiness_probe_cache` の detail に秘密値・応答本文なし |
| INVAR-1 | 維持 | 非 allow・critical の索引 0（readiness の `非許可・重大の表出=0`）。CHECK 制約は変更なし |
| INVAR-2 | 維持 | 反映前 backup、preview.3 の rollback 用 image を保持、replacement plan は不適用 |
| INVAR-3 | 維持 | client 接続後の初回 pass で `scans_fresh` 23（policy v3 の再 scan）。以降の pass は `scans_fresh=0`、`scans_reused` へ戻った。OpenAI の `api_attempts`（最終 25）と `scans_completed` は一致し、`scans_failed=0` |
| INVAR-4 | 維持 | readiness の `permanent_blob_storage_disabled` pass。`media_fetch_success` 14、各 blob は `fetch local miss` → `ephemeral fetch remote transfer completed` |

## 反映中に見つけた問題

- readiness timer の停止（#1097）: startup を再実行すると、readiness timer の次回実行が決まらなくなる。04:51 UTC 以降は手動実行の直後（05:20、07:51、08:41）しか activation が作られず、有効期限 900 秒を過ぎると公開 read surface（index／trust）が閉じていた。08:42 に `systemctl start kukuri-readiness.service` を実行し、08:48 の timer 起動を確認した。修正は PR #1098 で main へ入った。
- 旧 provider の signal の残存（#1109）: 旧 VLM が 2026-09-15 に発行した nsfw high の signal 2 件（confidence 84）は、OpenAI の再 scan で allow・advisory なしになった後も有効なままである。タイムラインの advisory 照会はこの 2 件を返すため、「見つける」（verdict 由来）と表示が食い違う。
- client の表示（#1107、#1108）、同意画面（#1106）、開発ビルドの app data 共有（#1105）。
- 旧 VLM の `unknown_csam` 行が `readiness_probe_cache` に残っている（04:44 UTC の pass、現在の slot 構成では参照されない）。

## v0.2.6 での再確認（2026-09-18）

反映の記録は [v0.2.6-preview.1 の記録](2026-09-18-v0.2.6-preview.1-release-rollout.md)。CN は source `4b751946`、client は Windows NSIS／Linux Deb とも 0.2.6。

### #1109 の本番反映

- migration `202609170003` の適用時刻（03:16:02 UTC）で、旧 VLM の nsfw high signal 2 件（`6f0b…`、`4956…`）の `expires_at` が設定された。appeal・operator 調整の対象外で、行は削除されていない。
- 有効な signal は `3d8d3cb1…` の画像 `f6a38ae2…` の nsfw／objectionable（low）の 2 件だけになった。`6f0b…`／`4956…`／`d16efa12…`／`3d8d3cb1…` はいずれも `allow`・非 critical で索引にある。

### 新規の advisory 付き投稿

- 閲覧側（Linux、アカウント `e8700632…`、成人向け表示 OFF）を停止した状態で、投稿者（Windows、アカウント `bcdde13a…`）が画像投稿 `184508a5…`（画像 `5ca47f2e…`、3,449,739 bytes）を general へ行った（13:43 JST）。
- CN は 04:47:30 UTC に post／blob とも `allow`・policy v3、advisory `adult`（nsfw、confidence 100、`classifier_score`）を付け、索引した。signal は 1 件（low）。OpenAI の `api_attempts=2`、`scans_failed=0`。
- 投稿直後の変更通知では、indexer の取得先 peer が直前に終了した Linux client（`13c1ee7e…`）だけで、本文を取得できず `temporarily failed to ingest object record` になった。投稿者の Windows client は bootstrap に登録済み（TTL 90 秒で更新）だったが、取得先 peer は全件見直しのときにだけ更新されるため、次の pass（04:47）で取得・索引された。`last_index_lag_secs=227`（runbook §5.6 の「数十秒以内」を超えた）。
- signal／event の件数は 2 巡後も増えていない（`risk_signals` 5 件うち有効 3、`signed_moderation_events` 613、索引 22）。

### client 実機（AC-6）

運営者が Linux client で確認した結果:

| 項目 | 結果 |
| --- | --- |
| 新規投稿と `3d8d3cb1…` がタイムライン・見つけるの両方で代替表示（枠上の短い表示、クリックで詳細 dialog） | 達成 |
| 同じ画像 `4c8fd3bf…` を使う advisory なしの投稿（`6f0b…`／`4956…`／`d16efa12…`）が通常表示でちらつかない | 達成 |
| 通常画像のスケルトン残留なし | 達成 |
| 表示 ON→OFF の切替でちらつきなし | 達成 |
| 表示 OFF の間に advisory 付き画像の bytes を取得しない | **未達（#1152）** |

- Linux client は 13:48:05 JST に起動し、13:48:07 に `5ca47f2e…` の `.data` を blob store へ永続保存した。表示を ON にしたのは 13:49:08 で、そのときの `get_blob_media_payload` は既にローカルにあるため `fetch hit` だった。13:48:07 前後の画面表示用の取得に `5ca47f2e…` は無い。
- 原因は、投稿 projection 反映時の添付状態確認（`best_effort_blob_cache_status` → `IrohBlobService::blob_status`）がローカルに無い blob を `fetch_blob` で remote 取得・永続化すること。成人向けゲートは `blob_media_payload` にだけあり、この経路を通らない。self-label の成人向け media も同じ経路で取得される。
- 前日の `f6a38ae2…` も表示 OFF の Linux client に 19:37:59 JST（CN の advisory 付与の 5 秒前）に保存されていた。

### AC-7（Windows）

#1105 の修正は既存の app data を移動しない。Windows の配布版には開発ビルドが記録した version 6 の同意（2026-09-16）が残っており、再同意は表示されない。今後の開発ビルドは `<identifier>.dev` を使うため、同じ混入は起きない。AC-7 は Linux の実機確認で達成とする。

### 判定

- AC-6 は表示について達成、取得ゲートについて未達。#1152 の修正後に、表示 OFF の client が新しい advisory 付き画像の bytes を保存しないことを再確認する。
- それ以外の AC・INVAR の状態は前回と同じ。
