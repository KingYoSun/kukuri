# Community Index 復旧・再発防止

## 現在判定・範囲

- 本番復旧・再発防止反映済み。2026-09-14のユーザー依頼「復旧再発防止対応」を実行根拠とする。後続依頼によりブランチ・commit・push・PRとCI/独立監査成功後のmergeも承認されている。
- 基準: `8fee1cbf`。修正前証拠は[調査記録](2026-09-14-community-index-investigation.md)。
- リスク区分C（本番の公開索引対象の永続変更、運用監視の外部出力）。独立監査を実施する。
- 目的: default onboarding nodeの既定3トピックで検索・発見・おすすめを復旧し、対象登録漏れ・本文取得失敗を検出する。
- 対象外: 任意ノードへの既定トピックの強制登録、安全性判定の緩和、GUI変更。旧4トピック削除は[後続依頼の記録](2026-09-14-retire-legacy-community-topics.md)に分離する。

## 完了条件

- AC-1: 本番の既定3トピックを正式な管理操作で追加し、7対象と監査IDを確認する。
- AC-2: 既定3トピックそれぞれの公開投稿が索引に入り、同じobject IDが日本語/ASCII検索・発見・おすすめで返る。本文取得失敗が復旧を阻む場合は供給元まで追って解消する。
- AC-3: default nodeに明示設定した期待トピックの欠落と全索引の空状態を監視し、既存の運用監視へ接続する。一般の空ノードには適用しない。
- AC-4: 本文取得失敗を外部safety provider障害と区別して計測する。失敗中も正常と誤認しない運用手順・テストを置く。
- INVAR-1: allow-only、署名/hash検証、private channel境界、本文Blobの恒久非保持を維持する。
- INVAR-2: 本番の変更は既定公開トピック追加と監視に限定。既存データ、鍵、provider設定、コンテナimageは変更しない（調査で別の必須修正が判明した場合は記録を更新）。
- INVAR-3: 監視はSELECTと状態/logの参照のみ。投稿本文・秘密値・ピア識別子をmetricsへ出さない。検証投稿は許可済みのgeneral/test/devに限定する。

## 作業・固定surface inventory

| ID | 入口→処理→sink | 作業・検証 | 条件 |
| --- | --- | --- | --- |
| T1 / INV-1 | IAP admin GET→preview→apply→supported_topics/operator_actions | 追加前後の集合・preview・監査IDを確認 | AC-1、INVAR-1/2 |
| T2 / INV-2 | 専用CLI profile→公開投稿→docs/blob→indexer→利用者index API | 既定3トピックで往復確認。認証・同意は通常経路 | AC-2、INVAR-1/3 |
| T3 / INV-3 | Terraform設定→VM monitor(timer/manual/startup)→SELECT/status/log→Cloud Monitoring | 期待topic/空索引/本文取得失敗のfixtureテスト、Terraform検証、VM実行とmetric確認 | AC-3/4、INVAR-2/3 |
| T4 / INV-4 | rollout runbook→実投稿・検索検証 | 稼働確認と実動確認を区別。独立監査、記録 | 全条件 |

## 状態遷移

| ID | 前提・trigger | 期待・禁止 | 検証 |
| --- | --- | --- | --- |
| TR-1 | 既定3topic未登録→管理追加 | 3件追加、既存保持、監査記録 | 本番前後SELECT |
| TR-2 | 未索引→同期/本文取得/scan成功 | allowのみ索引、3 APIから取得 | CLI/APIとDB |
| TR-3 | 期待topic欠落/全件空/DB取得失敗 | 監視で異常・unknownを成功扱いしない、DB無変更 | fixture/実装監査 |
| TR-4 | 一部旧Blob取得不能、正常投稿あり | 取得失敗を別metricへ記録、provider障害と混同しない | log fixture、本番metric |
| TR-5 | 任意ノードで期待topic設定なし | 空ノードにdefault-node固有警報を強制しない | Terraform tests |
| TR-6 | 再起動/次回startup | 監視設定を再生成し、topicはDBから復元。秘密値を出さない | template検証、metadata同期 |

## 検証の選定

現時点の実装対象はTerraform・monitor script・runbookであり、Rust/UI挙動は変更しない。scriptの再現fixture→修正後成功、Terraform fmt/validate/test/plan、VM実動、独立監査を実施する。Rust変更が必要と判明した場合は該当pathの必須validationを追加する。

## 実施証跡

### 公開索引の復旧（AC-1 / AC-2）

IAP経由の管理画面で3件のpreviewについてactor=`ops@kukuri.app`、公開topic追加、安全性/準備確認の維持を確認し、applyした。すべて `present:false → true`。

| topic | 管理操作の監査ID |
| --- | --- |
| general | `ecd52b7c-213a-480b-a239-70e1bff9087b` |
| test | `5362b63d-e97d-40ed-a6b6-d3e93e419673` |
| dev | `97269e05-f5c7-402f-bd49-08a8a7e6e017` |

既存4topicは保持。次の巡回で7scopeがopenし、画像にあったgeneralの既存投稿7件が索引に復帰した。
画像の「こんにちは」のobject IDは `14eddc8cb6aa560bed2ba70bf626736eb637d4ef53ef8fa4ebe4fb00598b5014`。

release v0.2.3-preview.2のLinux x86_64 CLIを取得し、SHA256
`c05ce3bebe95655116d50c5295791dc7c42c0c5ff747f4f73a3a1c0efa679eb2` を公開checksumと照合した。
WSLの専用profile `recovery`（日常GUIと別、データ/runtimeは `/tmp/kukuri-cn-recovery-20260914/`）を使用。
通常のapp/node同意・認証を通した検証identity `8d4f017829894ab7bf1505b4e3a22f913868241f0e905bc6acd3675aa90df862`
（表示名「検索復旧確認（運営）」）から、02:39 UTCに各1件投稿した。
本文は「コミュニティ検索の復旧確認です。 cnrecovery20260914 <topic>」。

| topic | object ID |
| --- | --- |
| general | `b52ce038ec01f78dd70aa94acc78070bcab492f2a4e48614640a3bb545d391d5` |
| test | `329e1a95aee34694232c4cf0a35b43668387da9401c3eb907cb9187110e44552` |
| dev | `d6f5b83540ac121bc0289d00b007b9668628f2d30f8408b2a9ece7a9a8e8c93e` |

CLIの投稿操作だけではtopic購読が始まらず、最初は `subscribed_topics=[]` だった。
3topicの `set_topic_gossip_enabled(enabled=true)` を呼び、通常の購読を開始して同期した。
postを重複送信していない。

02:45:22 UTCに以下を同一の認証済みCLIで照合した。

- ASCII `cnrecovery20260914` と日本語 `復旧確認` の横断検索: それぞれ3件、上記3 IDがすべて一致。
- topic指定検索: general/test/dev各1件で対象ID・scopeの一致。
- 発見・おすすめ: それぞれ10件、上記3 IDがすべて存在。
- 元の検索語 `こんにちは`: 画像の既存object IDが存在。
- DB: general 8 / dev 1 / test 1。新規3件の通常パイプライン到達を確認。

### 監視の実装と検証（AC-3 / AC-4）

- moduleとlow-cost envへ `index_expected_topics`（既定は空）を追加。本番tfvarsにだけ既定3topicを明示。
- 監視helperはread-only/10秒timeoutのSELECTで登録集合・索引件数を取得。取得失敗は0/-1で異常側へ倒す。
- 直近10分の本文取得失敗logは件数だけを送信。不明は-1。provider障害や有害判定として扱わない。
- 空ノード/対象欠落/正常索引＋本文失敗/DB失敗・不正応答/log失敗/注入拒否のfixtureを追加。修正前の本番証拠に加え、実装前にfixture未充足を確認し、実装後WSL Pythonの7 tests PASS。
- Terraform fmt/validate PASS、mock providerによる5 runs PASS。`-filter=tests/...`はWindowsで対象が一致せず0 testsとなったため、filterなしで全5 runを実行し直した。
- 独立監査agent `audit_index_recovery`: 監視実装はPASS、blocking findings=0、inventory適合1/不適合0/未分類0。監査対象は未コミット差分（ユーザーからcommit/PR/merge依頼なし）。Windows Pythonからの実行問題を指摘されたため、fixtureの説明とrunbookをWSL/Linux Pythonへ明確化した。

### 本番反映

- 事前backup: 2026-09-14 02:44:24 UTC、generation `1789353864904869`、460,890 bytes。backup service `Result=success / ExecMainStatus=0`。
- Terraformの変更は3 metric descriptor・2 alert policyと、monitorを埋め込むstartupの1行だけ。初回planのVM replacement理由は `metadata_startup_script` のみ。
- runbook 3節に従いstartup metadataをin-place同期。生成済みstartup SHA256は `e69baa83dbeeeeb8e66f26a5a9be039693ca9e3f7805b0c212ad4c1aa9b922ec`。
- 再planは5 create / 0 change / 0 destroy。置換planは適用していない。
- runtime monitorは旧fileをbackupし、生成物をroot:root / 700で配置。SHA256 `d7fd948ba315d2cf78f95e7f70613889b322a5489c758eeb05ae3f92d6326e8e`。
- 02:47 UTCのmonitor serviceは `Result=success / ExecMainStatus=0`。Cloud Monitoring APIで `index_expected_topics_present=1`、`index_expected_topics_entries=10`、`body_fetch_failures_recent=7` を確認した。
- 既存7件の旧topic本文取得失敗は継続しているが、既定3topicの復旧とは切り分けた。取得できない本文をallowに変更する・BlobをCNへ恒久保存する等の迂回は行っていない。
- Terraform apply: 5 added / 0 changed / 0 destroyed。metric descriptorの作成は5分台で完了。
- alert policy: 空索引=`12959405895127805618`、期待topic欠落=`15874404324009402131`。Monitoring APIで両方enabled、LT 0.5 / 300秒、既存通知channel保持を確認。配送試験のメール送信・受信確認は行っていない。
- 最終Terraform planは `No changes` / exit 0。metadata上のstartup hashが生成物と一致し、runtime monitorはroot:root / 700。7containerは従来の稼働時刻を維持し、restartなし。
- 02:53:15 UTCの次回定期実行でmetricは1 / 10 / 14（本文失敗は10分window内に7件×2巡）。timerはenabled / active。
- 02:54:49 UTCに同じ検索・発見・おすすめの8確認を再実行してPASS。DBは3topic計10件、すべて `allow / critical=false`。
- runbook・本番反映証拠の追加独立監査もPASS、blocking findings=0。監視helper本体は初回監査後不変。
- 検証CLIをSIGTERMで正常終了し、VMの転送用一時scriptを削除。検証profile・投稿ID・生成物・backupは保持した。検証投稿は通常P2P投稿なので、供給元が不在なら本文の再取得・継続表出を保証しない。

## 完了判定と残る制約

AC-1〜4を上記の本番・fixture・Terraform・独立監査証拠で確認。Rustの索引・safety・retention処理を変更せず復旧した。既存7件の旧topic投稿は本文供給不能のままであり、その復元は今回成功した既定3topicの復旧と区別する。本文の安全性確認を迂回して検索に戻す処理は追加していない。任意ノードの空状態は警報対象にせず、本番で明示した期待集合だけを監視する。

## PRへの反映

後続の公開依頼に合わせ、`Kukuri Terraform` に監視fixture・mock Terraform contracts・旧docs保守補助のRust testsを接続した。保守補助のpathも起動対象に含め、実DBやVMに接続しない検証jobとして実行する。PR headに対する監査・CIの最終結果はPRへ記録する。

CI用 `CN_LOW_COST_TFVARS_B64` に期待topic設定がなかったため、既存の他設定を保持したまま3topicの集合だけを追記し、APIで保存値の一致を確認した。非公開のtfvarsや秘密値はcommitしていない。
