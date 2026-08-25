# Community Node 発信者情報開示・保全対応

## 目的と停止条件

この runbook は、Community Node が現に保有する情報への開示・保全要請を受けたときの事実確認と操作手順を定める。法的判断や本人性・代理権の判断を自動化しない。請求主体、法的根拠、対象、期限、管轄、意見聴取の要否が確認できない場合、または保有情報だけでは対象との関連付けを説明できない場合は、開示せず法律専門家へ確認する。

一次資料は総務省の[発信者情報開示関係ガイドライン](https://www.soumu.go.jp/main_sosiki/joho_tsusin/d_syohi/ihoyugai_04.html)と[情報流通プラットフォーム対処法の解説](https://www.soumu.go.jp/main_sosiki/joho_tsusin/d_syohi/ihoyugai_03.html)を確認する。運用時点の法令・裁判所書式を別途確認し、この文書だけで法的結論を出さない。

## 保有情報 inventory

| 構成・経路 | 保有し得る情報 | 関連付けの限界 |
|---|---|---|
| report endpoint | 対象種別・ID、理由、本文、任意連絡先、受付・状態時刻 | 申告内容であり、投稿者本人性を証明しない |
| rights request endpoint | 対象ID、権利主張、連絡先、本人／代理権情報、証拠参照、判断履歴 | 外部参照先の内容や真正性を自動取得・保証しない |
| auth / rendezvous / relay | 構成に応じたpubkey、接続元IP・port、認証・接続時刻、topic presence | 保持期限後は不存在。NAT、VPN、共有回線を越えて個人を断定しない |
| Direct P2P | nodeを経由しないpeer間通信・内容 | 当該nodeは原則として観測・保有しない |
| relay supported / fallback | relayが処理した接続メタデータ。暗号化payloadは内容として扱わない | relay利用だけで投稿IDと接続元を関連付けない |
| community index / moderation | node-localな索引対象ID、verdict、risk signal、operator action | 投稿正本、他node、peer端末、既取得copyへの権限はない |

「保有していない」「保持期限切れで削除済み」「識別子間を関連付けられない」も調査結果として記録し、推測、過去ログの新規生成、第三者端末への照会を行わない。

## 要請の分類と初動

1. 受付原本を変更不可の案件記録へ保存し、受領時刻、提出者、対象識別子、回答期限を記録する。
2. 裁判外請求、弁護士会照会、裁判所の発信者情報開示命令、提供命令、消去禁止命令、捜査機関照会を区別する。名称だけで強制力を推定しない。
3. 本人性・代理権、対象URL／投稿ID、権利侵害の主張、開示を受ける正当な理由、管轄と送達の真正性を二者で確認する。
4. 対象recordが存在する間に、必要なデータ区分だけ legal hold を開始する。全DB、wildcard、無関係な案件を対象にしない。
5. 発信者への意見聴取が原則必要か、例外が成立するかを専門家と確認し、回答期限と矛盾しない計画を作る。通知で生命・身体、安全、捜査、証拠保全を害し得る場合は独断で連絡しない。

## legal hold

```powershell
cargo run -p kukuri-cn-cli -- legal-hold start `
  --target-kind rights_request --target-id <CASE_ID> `
  --data-categories rights_request,rights_request_contact,rights_request_identity,rights_request_evidence,rights_request_history,operator_audit `
  --basis "<命令・照会の識別子>" --release-condition "<確定・期限・取消条件>" `
  --actor legal@example.net
```

holdは物理削除だけを止める。期限切れ対象は通常API・管理一覧・公開statusには再表示されない。開始時に対象ID、区分、根拠、解除条件を別の確認者が照合する。

## 調査と限定 export

1. operator-config、manifest、DB schema、保持期限を確認し、請求対象時刻に実在した構成を確定する。
2. 対象IDから直接参照できる行だけを調べる。pubkey、IP、port、接続時刻、認証記録、投稿IDを結ぶ明示的な列・監査記録がなければ「関連付け不能」とする。
3. export実行者と確認者を分け、対象案件と区分を再確認する。

```powershell
$env:COMMUNITY_NODE_LEGAL_DATA_KEY = '<Secret Managerから安全に注入>'
cargo run -p kukuri-cn-cli -- legal-hold export `
  --hold-id <HOLD_ID> --actor reviewer@example.net --output <ABSOLUTE_OUTPUT_PATH>
```

exportはallowlist DTOだけを含む。暗号鍵、nonce、ciphertext、JWT、追跡secret hash、private channel secret、provider token、無関係な案件がないことを二者で確認する。提出物には抽出日時、node ID、対象期間、欠落・期限切れ・関連付け不能の説明、hashを付け、送付経路と受領確認を記録する。

## 解除と削除

法的根拠の失効、手続の確定、取消、定めた解除条件の成立を二者で確認してから解除する。

```powershell
cargo run -p kukuri-cn-cli -- legal-hold release --hold-id <HOLD_ID> --actor legal@example.net
cargo run -p kukuri-cn-cli -- retention sweep
```

二重解除は失敗する。解除後、すでに期限切れの対象は次のsweepで物理削除される。開始、export、解除のauditには機微内容を入れず、対象ID、区分、actor、時刻だけを残す。
