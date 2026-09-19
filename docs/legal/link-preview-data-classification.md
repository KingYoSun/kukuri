# Feature Data Classification: 公開投稿のリンクプレビュー

ADR 0002（`docs/adr/0002-feature-data-classification-template.md`）とADR 0051に基づく分類。

### Feature Data Classification

- Feature 名: 公開投稿の外部URL link化とOGP preview
- Durable / Transient: URLは既存投稿本文の一部としてDurable。抽出segment、取得中状態、OGP metadata、raster image data、cacheはprocess内Transient
- Canonical Source: URL文字列は既存`PostView`本文。previewは取得時点のリンク先HTTP responseのOGP／HTML metadata
- Replicated?: 新規複製なし。OGP response／imageをdocs、blob、P2P、Community Nodeへ配布しない
- Rebuildable From: 投稿本文とリンク先responseから再取得可能。取得不能ならinline linkだけで動作する
- Public Replica / Private Replica / Local Only: 投稿本文の既存分類を維持。preview／cacheはLocal Only transient
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要
- 必須 contract: `fetch_link_preview`のtyped outcome、frontend external URL segment、public／visible／Ready gate、SSRF／redirect／size／MIME guard、既存`open_external_url`
- 必須 scenario: harness scenarioは追加しない。Tauri unit／IPC gate、frontend component／browser、Windows Tauri実機で許可URL、禁止URL、redirect、timeout、oversize、cache、表示gate、外部browser起動を確認する

## 外部送信と保持

- 送信先: 公開投稿の先頭URLのlink先hostと、そのpageがOGP imageとして指定したpublic host
- 契機: app-level legal consent後、対象の公開・表示可能・settledな投稿cardがviewport内で表示されたとき
- 送信・観測され得る項目: IP address、HTTP／TLS request metadata、URL path／query、固定User-Agent、preview閲覧の発生
- 送信しない情報: cookie、Authorization、Referer、公開鍵、account／topic／channel／post ID、他の投稿本文、private channel／DMのURL
- 端末内保持: sanitized metadataとbounded data imageをprocess-memory cacheへsuccess 10分／failure 60秒。128 entryかつ16 MiB上限。process終了で消える
- 第三者保持: link先、image host、通信経路事業者の方針に従う。kukuriから一括削除できない

## Security boundary

URL／DNS／実接続先と全redirectをTauri側で検証し、public-routable addressへconnectionを固定する。WebViewへremote image URLやraw HTMLを返さず、PNG／JPEG／GIF／WebPだけをbounded data URLとして返す。詳細と上限はADR 0051を正本とする。
