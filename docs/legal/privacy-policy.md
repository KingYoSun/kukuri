# kukuri プライバシーポリシー

最終更新日: 2026-09-03

施行日: 2026-09-03

Legal bundle version: 5
正文言語: 日本語

本ポリシーは kukuri デスクトップアプリ自体に適用されます。各 Community Node が個別に提示するプライバシーポリシーとは別のものです。日本語版を正文とし、アプリ内の英語版・簡体字中国語版は参考訳です。参考訳と日本語版に差異がある場合は日本語版を優先します。

## 1. 管理主体・問い合わせ先

- 管理主体: KingYoSun
- 問い合わせ先: ops@kukuri.app
- 運営主体の氏名・住所は、上記窓口へ請求いただいた場合に遅滞なく回答します。

## 2. 基本方針

kukuri は P2P を基盤とするアプリです。アカウントを識別する秘密鍵、設定、同意記録など端末内だけで扱う情報がある一方、プロフィール、公開フォロー関係、投稿、リアクション、添付等は、利用者が選択した公開範囲または audience に応じて P2P 参加者へ複製されます。Community Node の検索・索引等を利用する場合は、対象データや query が選択した Node に送信されることがあります。

データが特定の中央 server に一律集約されないことは、すべてのデータが端末内だけに留まることを意味しません。

## 3. 端末内に保存する情報

- アカウントを識別する秘密鍵、local database／docs／blob、下書き、bookmark、mute／block、表示・通知・workspace 設定。
- Community Node の設定、認証 token、Node 別同意、アプリ規約への同意履歴。
- 18歳以上である旨の自己申告記録と、成人向け表現の表示設定。
- P2P から受信し、local cache／projection として保持する情報。

これらのうち秘密鍵、アプリ同意記録、18歳以上の自己申告、成人向け表示設定等は network へ複製しません。一方、投稿等の local copy は元の共有範囲で P2P 複製されたデータの一部です。local 保存と network 非送信を一律に同じ意味では扱いません。

18歳以上である旨の確認は自己申告として端末内だけに記録します。kukuri は生年月日や公的身分証を収集せず、公的な年齢確認を行いません。新しい端末または restore 後には再度の申告を求めます。

## 4. P2P で共有・複製される情報

- **公開プロフィール・公開フォロー関係**: 公開 author replica を取得する peer に複製されます。
- **公開投稿・返信・再投稿・リアクション・添付**: 対象 topic／author replica に参加する peer に本文、署名 metadata、reaction、blob が複製されます。
- **private channel**: 対応する capability を持つ参加者だけを audience とします。利用者が private indexing を明示的に許可した場合は、選択した Community Node にも必要な情報を送ります。
- **DM**: 指定した相手との pairwise replica で扱います。相手端末が取得した copy を送信側だけで消去することはできません。
- **live、game、Dome 等**: 参加中の session peer に、状態、入力、chat、asset 参照等の機能に必要な情報を送ります。

## 5. 接続時に扱われる情報

公開鍵、endpoint ID、IP address、address hint、接続先・時刻・経路等の接続情報が、Mainline DHT、接続相手、選択した Community Node、iroh relay、通信経路事業者から観測される場合があります。

通信経路の優先度は `Direct P2P -> Relay Supported P2P -> Relay Fallback` です。Community Node や relay が接続補助に関与しても実データが必ず relay を通るわけではありません。Direct P2P と Relay Supported P2P が成立せず、Relay Fallback になった場合は、暗号化された実データ traffic が relay を経由します。

## 6. Community Node で扱われる情報

利用者が Node を保存して当該 Node の規約へ同意すると、認証、consent、bootstrap heartbeat、topic rendezvous、検索・発見、indexing request、通報、tester feedback 等の有効な機能に応じて、公開鍵、proof、endpoint 情報、topic key、検索 query、対象識別子、任意入力内容が送信されます。

Community Node 運営者と kukuri 運営者が同一とは限りません。Node ごとの処理、外部送信、保持期間、問い合わせ先は、その Node の manifest から開けるプライバシーポリシー、外部送信表示、保持文書を確認してください。現行配布物の `https://api.kukuri.app` は初期候補であり、固定接続先ではありません。

## 7. 自動更新確認と外部送信

アプリは同意後の起動時と30分ごと、および利用者の手動操作時に GitHub Releases へ自動更新確認を行います。更新確認では IP address と HTTP／TLS request metadata が GitHub や経路事業者から観測されます。送信先、目的、項目、保持主体の一覧は `docs/legal/external-transmission-notice.md` に記載します。

## 8. 診断レポート

利用者が任意で生成する診断レポートには、アプリ版、platform／user agent、接続・delivery／discovery 状態、active path、peer／topic 件数、通知・更新状態、Node URL／session／retry／error が含まれ得ます。

既定のレポートには秘密鍵、認証 token、private channel capability secret、invite／share token、DM 本文、local database path を含めません。コピーまたは書き出しだけでは自動送信されず、利用者が GitHub その他へ添付した場合にその送信先へ渡ります。

## 9. 保存期間

- 端末内情報は、利用者が個別に削除するか app data を削除するまで保持されます。cache 等には実装上の容量上限や再構築による置換があります。
- P2P 相手、DHT、relay、GitHub、各 Community Node が扱う情報は各主体の方針に従い、kukuri アプリはその保持期間を一括して制御しません。
- Community Node の一時 presence、index、report 等は機能ごとに異なります。実際の期間は当該 Node の retention 文書を確認してください。

## 10. 削除・訂正の限界

local data の削除や投稿撤回は、対応する client／Node が認識する範囲で将来の表示、同期、検索、取得等を停止させるものです。P2P の性質上、相手端末や第三者 Node が既に取得した copy／cache、DHT／relay／経路事業者の log を遠隔から完全に回収・消去することは保証できません。

## 11. 行動分析を行わない現行方針

現行の kukuri アプリは、既定で行動分析、広告 tracking、自動 crash report 送信を行いません。将来これらを導入する場合は、送信開始前に本ポリシーと外部送信表示を改訂し、必要な再確認・再同意を求めます。

## 12. 同意と変更

初回起動時は network 接続を開始する前に、利用規約と本ポリシーへの同意を確認します。重要な変更では文書版を上げ、network を再開する前に再表示・再同意を求めます。問い合わせは `ops@kukuri.app` で受け付けます。

## 13. 変更履歴

- version 5（2026-09-03）: 利用規約の全面改訂に合わせ、legal bundle の版・施行日と、アプリ本体／Community Node／P2P 上の第三者の責任分界に関する用語を同期しました。データフローと外部送信の実質的な変更はありません。
- version 4（2026-09-02）: 管理主体、実データフロー、P2P 複製、DHT／relay、Community Node、GitHub Releases の自動更新確認、診断情報、削除限界、行動分析を行わない方針、日本語正文と参考訳を明記しました。
- version 3（2026-09-01）: 18歳以上の自己申告と成人向け表現の既定非表示を追加しました。
- version 2: 投稿コンテンツの権利帰属と限定的な技術利用許諾を追加しました。
- version 1: 初版。
