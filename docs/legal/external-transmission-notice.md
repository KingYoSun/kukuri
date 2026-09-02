# kukuri 外部送信表示

最終更新日: 2026-09-02

施行日: 2026-09-02

Legal bundle version: 4
正文言語: 日本語

本表示は kukuri デスクトップアプリの外部送信を説明するものです。各 Community Node が行う外部送信は、その Node の manifest から開ける外部送信表示・プライバシーポリシーを確認してください。

## 1. 管理主体・問い合わせ先

- 管理主体: KingYoSun
- 問い合わせ先: ops@kukuri.app
- 運営主体の氏名・住所は、上記窓口へ請求いただいた場合に遅滞なく回答します。

## 2. 現在行われる外部送信

| 送信先 | 送信契機 | 目的 | 送信・観測され得る項目 | 保持の考え方 |
|---|---|---|---|---|
| GitHub Releases | アプリ同意後の起動時、30分ごとの自動更新確認、手動確認、更新 download | 署名済み Preview update の確認・取得 | IP address、HTTP／TLS 通信に必要な request metadata、更新確認に必要な app／platform 情報 | GitHub と通信経路事業者の方針に従います。kukuri は結果と error を runtime state と診断表示に使用します |
| Mainline DHT | `seeded_dht` を有効にして接続先を探索するとき | P2P endpoint の発見 | endpoint ID、署名済み address record、通信元 IP address 等 | DHT 参加者に分散して扱われるため、kukuri が一括した保持・削除を制御しません |
| P2P 接続相手 | topic、profile、public post、private channel、DM、live／game／Dome 等へ参加するとき | 選択した audience 内の同期・表示・添付取得・real-time 通信 | 公開鍵、endpoint／IP address、対象範囲の署名済み metadata・本文・reaction・添付。private channel／DM は対応する capability／暗号鍵の範囲 | 受信端末が copy を保持し得ます。送信後の完全な遠隔回収・一括削除は保証できません |
| 構成された iroh relay | Direct P2P の接続補助、または Direct P2P と Relay Supported P2P が成立しない場合 | hole punching／endpoint assist、必要時の Relay Fallback | IP address、endpoint ID、接続 metadata。Relay Fallback では暗号化された実データ traffic | relay 運営者の方針に従います。relay URL があるだけでは実データが relay を通ったことを意味しません |
| 利用者が保存・同意した Community Node | manifest／policy 取得、認証、consent、bootstrap／rendezvous、検索・発見、明示した indexing、通報、tester feedback 等 | 選択した Node capability の提供 | 公開鍵、proof、endpoint 情報、topic rendezvous key、検索 query、対象識別子、通報／feedback の入力項目等。機能ごとに異なります | 当該 Node の privacy／外部送信／retention 文書に従います。Node 運営者と kukuri 運営者が同一とは限りません |

現行配布物は `https://api.kukuri.app` を Community Node の初期候補として含みます。これは固定接続先ではなく、利用者は削除または他の Node へ置換できます。実際の relay と Node の開示 URL は、その時点の配布設定、利用者設定、取得した Node manifest から確認できます。

## 3. 診断レポート

診断レポートは、利用者が設定画面でコピーまたは書き出すまで端末外へ自動送信されません。レポートには、アプリ版、OS／WebView の platform／user agent、接続状態、delivery／discovery／active path、peer／topic 件数、未読通知件数、更新状態、OS notification 状態、Node URL／session phase／retry／error が含まれ得ます。

既定のレポートには、秘密鍵、認証 token、private channel capability secret、invite／share token、DM 本文、local database path を含めません。利用者がレポートを GitHub その他へ添付した後は、選択した送信先の取扱いに従います。

## 4. 送信しない情報と現行の分析方針

- アプリ同意記録と18歳以上の自己申告記録は端末内にだけ保存し、network へ送信しません。
- 生年月日、公的身分証、公式な年齢確認情報は収集しません。
- 現行の kukuri アプリは、既定で行動分析、広告 tracking、自動 crash report 送信を行いません。将来導入する場合は、送信開始前に本文書とプライバシーポリシーを改訂し、必要な同意を求めます。

## 5. 変更履歴

- version 4（2026-09-02）: 実装上の更新確認、DHT／P2P／relay、Community Node、診断レポートを送信先・目的・項目・保持主体ごとに整理し、管理主体、Node 別責任、削除限界を追加しました。
