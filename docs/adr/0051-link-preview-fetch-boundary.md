# ADR 0051: 公開投稿のリンクプレビュー取得境界

## Status

Accepted

## Context

Issue #1174では、投稿本文中の外部URLをlink化し、OGP metadataを投稿カードに表示する。WebViewから任意URLやremote imageを直接取得すると、CORS／CSPだけでなく、投稿を表示した端末からprivate network、loopback、metadata service等へ到達するSSRF、redirect／DNS rebinding、無制限response、cookie／Referer送信、private contentの閲覧漏えいが起こり得る。

現行の投稿本文は`SmartReferenceText`が内部route、topic、access token、mentionを扱い、外部browser起動はTauriの`open_external_url`が絶対HTTP(S)とReady状態をsink側で検証する。profile／mediaは外部画像URLをview contractへ持たず、WebView CSPも汎用remote imageを許可しない。この境界を維持したままpreviewを追加する必要がある。

## Decision

### 1. 対象surface

- 本文中の資格情報を含まない絶対`http://`／`https://` URLは、`PostCard`内でlinkとして表示する。内部referenceとmentionを優先し、invalid URLは通常textのまま残す。
- 自動OGP取得は、app-level legal consent後のReady状態で、公開投稿（`channel_id == null`）のprimary contentが表示可能・settledで、Timeline／Profile／Bookmarks、Thread、Community Indexのcardがviewport内にある場合だけ行う。
- previewはprimary contentの先頭URL 1件だけとする。private channel、DM、Composer参照preview、adult-content gate、trust collapse、withdrawn、missing、local pending／syncing／failed、document hidden、viewport外では取得しない。
- inline linkとpreview cardは、元URLを既存`open_external_url`からOS browserへ開く。`og:url`、redirect後URL、image URLをnavigation先にしない。

### 2. 取得境界

- WebViewはpage HTMLとremote imageを取得しない。Tauri command `fetch_link_preview`だけがpage HTMLと任意OGP imageを取得し、sanitized metadataとbounded raster `data:` imageを返す。
- WebView CSPの`img-src`へ汎用`http:`／`https:`を追加しない。raw HTML、script、style、iframe、SVG、event attributeを描画・実行しない。
- requestはcookie store、Authorization、Referer、system proxyを使用しない。固定の最小User-Agentと`Accept`だけを送る。URL以外のpost／topic／account情報をheaderやqueryへ追加しない。

### 3. URL、DNS、redirect

- absolute HTTP(S)、資格情報なし、空白／制御文字／backslashなし、4,096 bytes以下、scheme既定port（HTTP 80／HTTPS 443）だけを許可する。
- `localhost`、single-label、`.localhost`、`.local`、`.internal`、`.home.arpa`と、loopback、private、CGNAT、link-local、documentation、benchmark、multicast、unspecified、reserved addressを拒否する。
- DNS応答が空、またはpublic／non-publicの混在なら拒否する。検証済みaddressをHTTP clientの名前解決へ固定し、実接続先addressが検証集合に無い場合も拒否する。
- redirect auto-followは無効にし、最大3回をapplication loopで処理する。各`Location`を現在URL基準で解決し、URL／DNS／実接続先を再検証する。HTTPSからHTTPへのdowngradeは拒否する。

### 4. resource budgetとcache

| 項目 | 上限 |
| --- | --- |
| connect timeout | 3秒／request |
| request total timeout | 6秒／request |
| redirect | 3回 |
| HTML body | 512 KiB |
| image body | 1 MiB |
| title／site／description | 200／100／500文字 |
| 同時取得 | 4件 |
| distinct URLのin-flight待機 | 32件 |
| process cache | 128 entryかつ16 MiB |
| success／failure TTL | 10分／60秒 |

- cache keyはfragmentを除いたnormalized original URL。queryはrequestの一部なので保持する。
- 同一URLのin-flight requestは共有する。cacheとin-flight stateはprocess-memoryだけで、restart、account data、DB、localStorage、backup、diagnostic reportへ残さない。
- page取得失敗、非2xx、非HTML、oversize、metadata無しはtyped `unavailable`とする。image取得だけの失敗はtext previewを維持する。UIは無期限skeletonや空cardを表示せず、inline linkへfallbackする。

### 5. metadata

- `og:title`、`og:description`／`description`、`og:site_name`、`og:image`系をHTML tokenizerで読む。titleは`<title>`、siteはfinal page hostへfallbackできる。
- control／余分な空白を除き、上限で切る。最低限titleが無ければpreview unavailableとする。
- imageはfinal page URL基準で相対解決し、pageと同じ取得guardを通す。PNG、JPEG、GIF、WebPのdeclared MIMEとmagic bytesが一致する場合だけdata URLへ変換する。SVG、HTML、AVIFその他はv1では表示しない。

### 6. consentと開示

自動previewは、リンク先pageとOGP image hostに対する新しい外部送信である。legal bundle version 8で、送信先、表示を契機とすること、IP address、HTTP／TLS metadata、path／query、保持主体を開示する。cookie、Referer、account／topic／post情報、他本文を送らないこと、private channel／DMを自動取得しないことも明記する。version 7以下では再同意前にruntimeを開始せず、invoke gateによりpreview commandも拒否する。

## Consequences

- 公開投稿の表示はリンク先から観測され得るため、previewの利便性と引き換えにlegal再同意が必要になる。
- private contentと非表示contentのURLは自動取得しない。利用者がinline linkを明示操作した場合だけ既存browser経路を使う。
- OGPはtransientな補助表示であり、取得不能でも投稿のcanonical contentとP2P同期は影響を受けない。
- site固有embed、永続cache、proxy service、複数previewが必要になった場合は、別Issueで送信先・保持・security boundaryを再決定する。
