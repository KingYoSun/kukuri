# IPv6環境でirohのbootstrapとrelayを介してpeer接続を成功させる方法

## エグゼクティブサマリ

irohの「bootstrap」は、libp2pのDHT bootstrapノードとは少し概念が異なり、主に**発見（discovery）インフラ**としてのDNS（pkarr署名DNSレコードを配る仕組み）やDHTが該当します。iroh自体は**QUICでエンドツーエンド暗号化されたピア接続**を張り、必要に応じて**リレー（relay）**で接続確立と転送を補助し、状況が整えば**リレー経由→直接接続へ移行**します。citeturn16view0turn21search15turn12search1

IPv6環境でも、**「IPv6がある＝常に外部から到達可能」ではありません**。家庭用ルータや企業FWが「新規の受信UDP/QUIC」を落とす構成が普通にあり、iroh側もその前提で設計され、**relay（HTTP/HTTPS上の接続＋独自プロトコル）を使って最初の到達性を担保**しつつ、**ホールパンチとアドレス発見**（近年はSTUNよりQUIC Address Discoveryに寄せる）で直接経路を作ります。citeturn18view1turn16view0turn22search0

実務上の成功パターンは次の組み合わせです。  
- iroh: **自前DNS discovery（iroh-dns-server）＋自前relay（iroh-relay）**をIPv6で公開し、クライアントは同じdiscovery originとrelay URLsに揃える。citeturn27view2turn23view0turn21search15  
- libp2p（併用/比較対象）: **bootstrapノード（DHT参加の入口）**と**circuit relay v2（TURN相当）**をIPv6で立て、multiaddrで明示し、必要ならDCUtRで直接化を狙う。citeturn5search2turn5search32turn5search24  

## 調査範囲と対象バージョン

本レポートは「iroh（および関連するlibp2pコンポーネント）」を、**IPv6環境で bootstrap（=発見/初期合流）と relay（=到達性担保/中継）を使ってピア接続を確立**するための具体手順として整理します。irohとlibp2pは別スタックですが、irohのドキュメントでも「libp2pとの比較」や「libp2p handshake仕様の借用」が言及されるため、運用者が混同しやすいポイントを概念対応づけして提示します。citeturn14search6

対象バージョン（2026-03-02時点の“最新安定版”として、公開ドキュメント上の最新を採用）:
- iroh: **0.96.1**（0.96.1の変更がChangelogに記載：2026-02-06）citeturn16view0turn9view0  
- iroh-relay: **0.96.1**（iroh-relay crate / relay server & client）citeturn10view1turn19view0  
- iroh-dns-server: docs.rs上で0.96系（ドキュメント表示とソース表示に0.96.0が見えるが、設定形式・CLIは0.96.0ソースで確認可能）citeturn25view0turn33view0turn31view0  
- rust-libp2p（Rustのlibp2p実装）: **libp2p crate 0.56.0**（docs.rs上の最新版として表示）citeturn6view1  

概念対応（設計時に混乱しやすい点）:
- irohの「bootstrap」に近いもの: **DNS discovery（pkarr + DNS）やDHT discovery**。irohはEndpointIdをキーに、DNS TXT（relay URLや直アドレス）を引いて接続に必要な情報を得る。citeturn21search15turn23view1turn11search2  
- libp2pのbootstrap: **DHT(Kademlia等)に参加するための既知の安定ノード**（最初にそこへ接続し、ルーティングテーブルやピア情報を得る）。citeturn0search22turn5search25  
- irohのrelay: **必ず到達できる“ホームリレー”を保持**し、接続確立・ホールパンチ補助・必要なら中継転送を行う。citeturn16view0turn18view0turn12search1  
- libp2pのrelay v2（circuit relay v2）: **第三者ピアを経由して2者をつなぐ仕組み**で、文脈によってはTURN相当に扱われる（DCUtRで直接化も狙う）。citeturn5search2turn5search24turn5search32  

## 事前条件

ユーザー要件により未指定項目は「制約なし」としつつ、IPv6で“確実に接続成功”させるために、実務で最低限押さえるべき確認点を整理します（iroh/libp2p共通）。

前提（未指定は制約なし）:
- OS: **制約なし**（Linux/macOS/Windows いずれも可。サーバ側はLinuxが一般的）  
- ネットワーク: **制約なし**（ただしIPv6到達性は環境で大きく変わる）  
- IPv6アドレス割当: **制約なし**（GUA/ULA/リンクローカル等）  
- ファイアウォール: **制約なし**（ただし受信遮断だと直接接続は困難）  
- NAT64/NAT: **制約なし**（IPv6でもFWやNPTv6等で“実質NAT的”挙動があり得る）  

ただし iroH 側の設計意図としても「家庭用ネットワークはIPv6でも受信を塞ぐことが多い」前提でホールパンチ/リレー補助を行うため、**“IPv6があるのに直接つながらない”のは想定内**です。citeturn18view1turn16view0  

### 実務チェックリスト

| チェック項目 | 目的 | 代表的な確認方法（例） |
|---|---|---|
| 端末がIPv6を持つ（GUA/ULA） | iroh/libp2pでIPv6経路を使える前提 | `ip -6 addr` / `Get-NetIPAddress -AddressFamily IPv6` |
| 外向きIPv6疎通（UDP含む） | QUIC(UDP)直結の可能性を上げる | `ping -6` / `curl -6`（UDPは後述のiroh-doctor等で確認）citeturn18view0 |
| サーバ（relay/bootstrap）がIPv6でlistenしている | 「IPv6で提供」の要件を満たす | `ss -lnptu` で `[::]:PORT` を確認（irohのbind設計はIPv6 listenを許容）citeturn10view2 |
| FWで必要ポート開放 | relay/bootstrapが到達可能 | iroh-relay: 80/443/7842/9090等（後述）、iroh-dns-server: 53/443等（後述）citeturn19view0turn27view2 |
| DNS（AAAA/NS）設定 | discovery/relayの名前解決 | authoritative運用ではIPv6でのlisten・AAAA登録が必要（一般論）citeturn11search18turn31view0 |
| multiaddrの構文理解（libp2p） | IPv6+relayで事故を減らす | multiaddrは左→右へstackされる（/ip6…/tcp…/p2p… 等）citeturn0search10turn0search11 |

## irohのIPv6 bootstrapとしてのDNS discoveryサーバ起動手順

### 位置づけ

irohで「EndpointIdだけで相手に繋ぐ」ためには、接続先の**ホームリレーURLや直アドレス**を解決できる仕組みが必要で、デフォルトはDNS discovery（pkarr署名パケットをDNSで返す）が中心です。citeturn16view0turn23view1turn21search15  
この“発見基盤”が、libp2p文脈で言う「bootstrap（最初に合流するための既知の入口）」に近い役割を果たします（ただしDHTのbootstrapノードとは用途が異なる）。citeturn23view1turn0search22

irohのDNS discoveryは、概念ページ上「`_iroh.<z32-endpoint-id>.<origin-domain> TXT` を引き、`relay=<url>` や `addr=<socketaddr...>` を得る」形で仕様化されています。citeturn21search15  
また、pkarrを用いて署名済みDNSレコードをpublishし、DNSでresolveできるようにする設計がブログ等で説明されています。citeturn11search2turn22search17  

### iroh-dns-serverの役割と提供プロトコル

iroh-dns-serverは「pkarr relay + DNS server」で、公開サービスとしては少なくとも次を提供します。citeturn27view2turn31view0  
- DNSサーバ（UDP/TCPでDNS問い合わせを受ける）citeturn27view2turn31view0  
- HTTP/HTTPS（`/pkarr` でpkarr署名パケットのGET/PUT、`/dns-query` でDoH）citeturn27view2  

「IPv6でのbootstrap」を満たすには、**DNSとHTTP(S)のbind_addrをIPv6にし、必要ならAAAA/NSもIPv6で整備**します（DnsConfig自体がIPv6 bindとAAAAレコード設定を持ちます）。citeturn31view0turn11search18  

### 設定ファイル例（IPv6対応・プロダクション寄り）

以下は docs.rs 同梱の `config.prod.toml` をベースに、**IPv6でlisten**し、ゾーン側に**AAAA（rr_aaaa）**も付ける構成例です。元例では `dns.port=53`, `https.port=443`, `pkarr_put_rate_limit="smart"`, `mainline.enabled=false` が示されています。citeturn27view1turn31view0turn27view2  

```toml
# iroh-dns-server-config.toml（例）
pkarr_put_rate_limit = "smart"

[https]
port = 443
bind_addr = "::"                  # IPv6で待ち受け（IpAddrを取る）
domains = ["dns.example.jp"]       # 公開用FQDN
cert_mode = "lets_encrypt"
letsencrypt_prod = true

[dns]
port = 53
bind_addr = "::"  # IPv6で待ち受け（未指定なら 0.0.0.0 になる仕様）
default_soa = "dns1.example.jp hostmaster.example.jp 0 10800 3600 604800 3600"
default_ttl = 30
origins = ["dns.example.jp.", "."] # discovery origin として使うドメイン
rr_a = "203.0.113.10"              # Aレコード（任意）
rr_aaaa = "2001:db8::10"           # AAAAレコード（IPv6公開に重要）
rr_ns = "ns1.example.jp."

[mainline]
enabled = false
```

- `bind_addr` は `Option<IpAddr>` で「IPv4/IPv6を指定でき、未指定なら0.0.0.0」と明記されています（IPv6で確実に待ち受けたいなら `::` を明示）。citeturn31view0  
- `rr_aaaa` で「全originに対するAAAA」を返せることが明記されています（IPv6前提の運用では重要）。citeturn31view0  

### 起動コマンド例（CLI）

iroh-dns-server のCLIは `-c/--config` で設定ファイルパスを取ります。省略時はデフォルト設定で起動します。citeturn33view0turn30view0  

```bash
# 設定ファイル指定で起動
iroh-dns-server --config ./iroh-dns-server-config.toml
```

### 動作確認（IPv6でbootstrapが効いているか）

iroh側のDNS discoveryは `_iroh.<z32-endpoint-id>.<origin> TXT` を参照するため、まずDNSサーバがIPv6でlistenし、外部からAAAA経路で引けることを確認します。citeturn21search15turn31view0  

- DNSサーバがIPv6でlisten: `ss -lnu | grep ':53'` に `[::]:53` が出ること（一般的な確認観点）  
- `dig -6` で問い合わせ（DNS運用の一般論としてもIPv6では `dig -6` が推奨されます）citeturn11search18  

また、**irohクライアント同士が同じDNS discoveryサーバ（origin）を使っていないと、お互いをDNSで見つけられない**点が公式ガイドに明記されています。citeturn23view1  

## iroh-relayをIPv6で起動しrelay経由の接続確立を可能にする

### relayの役割とプロトコル（iroh視点）

irohは通常「最寄りのリレー＝ホームリレー」を選び、他のendpointはまずホームリレー経由で相手へ到達し、成立後に直接経路（必要ならホールパンチ）へ移行します。直接が無理ならリレー経由を継続します。citeturn16view0turn12search1turn18view0  

relayサーバへの接続は「HTTP 1.1 + TLSで開始し、その後プレーンTCPへupgradeして独自のリレープロトコルで転送する」とi roh crateの説明にあります。厳しいFW環境でも通りやすい設計意図が示されています。citeturn16view0turn22search12turn22search15  

加えて、アドレス発見は近年 **QUIC Address Discovery（QAD）**へ寄せられており、relayはQAD用のQUICエンドポイントを持ちます。QAD自体はIETFドラフトとして定義され、iroh側の移行方針もブログで説明されています。citeturn18view1turn17search12  

### iroh-relayのlistenポートとIPv6 bind

iroh-relayのデフォルト値（重要な運用基準値）:
- HTTP: `80`  
- HTTPS: `443`  
- QAD用QUIC: `7842`（“QUIC”をテンキーで打った番号）  
- metrics: `9090`  
citeturn19view0turn15view0  

さらに `--dev` を使うとHTTPポートは `3340` がデフォルトで、TLS設定は無視されます（ローカル検証向け）。citeturn15view0  

### 設定ファイル例（IPv6・TLS・QAD有効）

iroh-relay はTOML設定をserdeで読み込みます。QAD（`enable_quic_addr_discovery`）を有効にする場合、TLS設定が必須で、無いと起動時にエラーになります。citeturn15view0turn19view0  

以下は「IPv6でlisten」「HTTPS（ACME/Let’s Encrypt）」「QAD用QUICポート7842」を明示する例です（フィールドは iroh-relay の `Config` / `TlsConfig` に対応）。citeturn15view0turn19view0  

```toml
# iroh-relay-config.toml（例）
enable_relay = true
http_bind_addr = "[::]:80"
enable_quic_addr_discovery = true
enable_metrics = true
metrics_bind_addr = "[::]:9090"

# 誰でも使える（デフォルト）
access = "everyone"

[tls]
https_bind_addr = "[::]:443"
quic_bind_addr = "[::]:7842"

# CertMode: Manual / LetsEncrypt（Reloadingはfeature次第）
cert_mode = "LetsEncrypt"
hostname = "relay.example.jp"
contact = "admin@example.jp"
prod_tls = true
```

補足:
- `enable_relay=false` にすると「中継転送はしないが、QAD等のNAT traversal補助だけ行う（ホールパンチ専用relay）」用途がコメントで説明されています。citeturn15view0  
- QAD用QUICポートのデフォルトが `7842` と定義されています。citeturn19view0  

### 起動コマンド例（IPv6）

iroh-relayのCLIは `--config-path` を取り、`--dev` ではHTTPのみ・3340番がデフォルトです。citeturn15view0  

```bash
# 本番構成（TLS/QADあり）
iroh-relay --config-path ./iroh-relay-config.toml

# ローカル検証（HTTPのみ、既定3340）
iroh-relay --dev
```

### 認証・ACL（allowlist/denylist/HTTP判定）注意点

iroh-relayはアクセス制御として、少なくとも以下の形をサポートします。citeturn15view0turn12search3  
- allowlist: 特定 EndpointId のみ許可  
- denylist: 特定 EndpointId を拒否  
- HTTPアクセス判定: 接続してきた EndpointId をヘッダに載せてHTTP POSTし、`200` かつ本文が `"true"` の場合のみ許可（任意でBearer Tokenを `IROH_RELAY_HTTP_BEARER_TOKEN` 環境変数から渡せる）citeturn15view0  

例（allowlist）:
```toml
access.allowlist = [
  "YOUR_ENDPOINT_ID_HEX_OR_FMT",
]
```

HTTP判定ではヘッダ名が `X-Iroh-NodeId`、トークン環境変数が `IROH_RELAY_HTTP_BEARER_TOKEN` と定義されています。citeturn15view0  

### relay起動ログの確認ポイント（例）

実際のログ例として、relayがIPv6で待ち受けていることを示す行（`[::]:80`）や、起動フェーズが確認できます。例えば過去のissueでは次のような行が示されています（ここでは短い行のみ引用）。citeturn14search0  

```text
INFO ... relay: serving on [::]:80
```

## libp2pでIPv6 bootstrapノードとrelay v2を構築する

ここからは「irohと併用する/比較する」目的で、libp2p側の **bootstrap** と **relay v2（circuit relay）** をIPv6で成立させるための実務要点をまとめます。

### multiaddrの前提（IPv6で事故を減らす）

multiaddrは「左から右へ、プロトコルスタックを順に積む」ため、たとえば `/ip6/.../tcp/.../p2p/...` のように読みます。IPv6だからといって `[]` を使うのではなく、multiaddrのルールで表現します。citeturn0search10turn0search11  

### bootstrapノード（DHT参加の入口）のIPv6起動要点

libp2pのbootstrapは、ネットワーク参加者が最初に接続する「既知の安定ノード」です。bootstrapノードが到達不能だとネットワーク参加が詰むため、**到達可能性（公開IPv6・FW設定）が最重要**です。citeturn0search22turn5search25  

IPv6のbootstrapノードmultiaddr（例）:
- TCPで待ち受ける場合:  
  `/ip6/2001:db8::10/tcp/4001/p2p/12D3KooW...BOOTSTRAP`  
- QUIC(v1)で待ち受ける場合:  
  `/ip6/2001:db8::10/udp/4001/quic-v1/p2p/12D3KooW...BOOTSTRAP`  

（注）libp2pの実装・サンプルでTCP/QUICの両方をlistenし、複数multiaddrを広告する構成が一般的です。citeturn6view1  

### relay v2（circuit relay）のIPv6構成要点

libp2pのcircuit relay v2は「中継ピア（relay）を挟んで二者をつなぐ」仕組みで、文脈上TURN相当として紹介されます。citeturn5search2turn5search24turn5search32  

relay経由multiaddr（典型形）:
- 「relay自身へ到達するmultiaddr」＋「/p2p-circuit」＋「（宛先peer指定）」という合成になります。multiaddrは左→右にstackされるため、“relayに繋いで回線を張る”ことを表現できます。citeturn0search10turn5search32  

例（概念例）:
- relay: `/ip6/2001:db8::20/tcp/4001/p2p/12D3KooW...RELAY`  
- 宛先peerがrelayに予約（reservation）を持っている状況で、発信側が宛先へ:  
  `/ip6/2001:db8::20/tcp/4001/p2p/12D3KooW...RELAY/p2p-circuit/p2p/12D3KooW...DEST`  

relay v2では、宛先側（DEST）がrelayへ「予約」を張ることで、発信側がそのrelayを通じて宛先へ到達できるモデルが仕様・ガイドで説明されています。citeturn5search2turn5search32  

また、relay経由で繋いだ後に **DCUtR**（Direct Connection Upgrade through Relay）を使って、可能なら直接接続へアップグレードする流れは、ホールパンチ実装と合わせて解説されています。citeturn5search24  

## 実験的な接続シナリオ別フローと期待アドレス

ここでは「直接IPv6」「bootstrap経由」「relay経由」を、irohとlibp2pでそれぞれ“どう成立するか”を可視化します。

### 期待されるアドレス形式まとめ（比較表）

| 項目 | bootstrap（発見/初期合流） | relay（中継/到達性担保） |
|---|---|---|
| iroh | DNS discovery（pkarr署名DNS）で `_iroh.<id>.<origin> TXT` を解決し、`relay=<url>` や `addr=<sockaddr...>` を得る citeturn21search15turn23view1 | RelayUrlで指定（例 `https://relay.example.jp/`）し、HTTP/HTTPSで接続確立→必要なら中継転送。QAD用QUIC(7842)を併設可能 citeturn16view0turn19view0turn15view0 |
| libp2p | 既知のbootstrap peer multiaddrへdialし、DHT等へ参加 citeturn0search22turn6view1 | circuit relay v2: `/.../p2p/<RELAY>/p2p-circuit/...` で中継。TURN相当として解説される citeturn5search2turn5search24turn5search32 |

### シナリオのフロー図

```mermaid
flowchart LR
  subgraph S1["直接IPv6接続"]
    A1[Peer A] -->|IPv6 QUIC/TCP 直結| B1[Peer B]
  end

  subgraph S2["bootstrap経由で発見してから接続"]
    A2[Peer A] -->|問い合わせ/合流| Boot[Bootstrap/Discovery]
    Boot -->|相手の到達情報| A2
    A2 -->|直結またはrelayで接続| B2[Peer B]
  end

  subgraph S3["relay経由で接続"]
    A3[Peer A] -->|dial relay| R[Relay]
    B3[Peer B] -->|register/reservation or home relay| R
    A3 -->|relay経由のストリーム| B3
    A3 -. 可能なら .->|直接経路へ移行| B3
  end
```

上図の“bootstrap/discovery”は、irohでは **iroh-dns-server（DNS/DoH + pkarr relay）**が該当し、libp2pでは**DHT bootstrapノード**が該当します。citeturn27view2turn23view1turn0search22  
“relay”は、irohでは **iroh-relay（HTTP/HTTPS上で接続確立→中継）＋QAD(7842/UDP/QUIC)**、libp2pでは **circuit relay v2** が該当します。citeturn16view0turn19view0turn5search2turn5search32  

### シナリオ別の期待アドレス（例）

#### 直接IPv6接続
- iroh: 相手の **EndpointId** と、（必要なら）直アドレス（`[2001:db8::b]:7777` のようなsockaddr）を含むEndpointAddrで接続。irohは「直結できるならrelayなしでも可能」と明記。citeturn16view0turn7search23  
- libp2p:  
  `/ip6/2001:db8::b/udp/4001/quic-v1/p2p/<PeerId>` など。multiaddrは左→右に積む。citeturn0search10turn0search11  

#### bootstrap経由（発見してから接続）
- iroh: `_iroh.<z32-endpoint-id>.<origin> TXT` を解決し、`relay=<url>` や `addr=` を得る（DNS discovery仕様）。citeturn21search15turn23view1  
- libp2p: bootstrapノードのmultiaddrへdial → DHT参加 → 目的peerを発見してdial。bootstrapノードは到達可能である必要（一般的注意点）。citeturn0search22turn6view1  

#### relay経由
- iroh: RelayUrl（例 `https://relay.example.jp/`）をRelayMapに含める。接続はまずホームリレー経由で確立し、成功後に直接化を試み、無理なら中継継続。citeturn16view0turn23view0turn22search3  
- libp2p: relay: `/ip6/.../tcp/.../p2p/<RelayPeerId>` を基点に `/p2p-circuit/p2p/<DestPeerId>` を重ねる（circuit relay v2）。citeturn5search2turn5search32turn0search10  

## トラブルシューティング集と参考ソース

### よくある失敗例と対処

iroh（DNS discovery/relay）:

- **QADを有効にしたのにTLS設定が無い（relay起動が失敗）**  
  - 症状: iroh-relay起動時に「TLS必須」系のエラー  
  - 原因: `enable_quic_addr_discovery=true` なのに `tls` 未設定（コード上で明示的にbail）citeturn15view0turn19view0  
  - 対処: `tls` セクションを追加するか、QADを無効化する。ローカル検証なら `--dev`（HTTPのみ）で切り分け。citeturn15view0  

- **DNS discoveryで相手が見つからない（origin不一致）**  
  - 症状: EndpointIdだけで接続できない・解決できない  
  - 原因: 2ノードが同じDNS discoveryサーバ（origin）を使っていないとDNSで発見できない、と公式ガイドに明記。citeturn23view1  
  - 対処: iroh-dns-server側の `dns.origins` と、クライアント側の参照先DNS/originを統一（システムDNSをそのサーバに向けるか、カスタムresolverを使う）。citeturn31view0turn11search13turn26view1  

- **IPv6があるのに直接接続しない/繋がらない**  
  - 背景: 家庭/企業ネットワークはIPv6でも受信を塞ぐことが多く、irohはrelayで到達性を担保する設計。citeturn18view1turn16view0turn12search1  
  - 切り分け: 公式の `iroh-doctor report` で `ipv6`, `ipv6_can_send`, `global_v6` などを確認できる（出力例が公式ドキュメントに掲載）。citeturn18view0  
  - 対処: 直接を狙うなら受信UDP/QUICを許可（FW設定）し、それが無理ならrelay前提で設計（帯域・コスト・冗長化）。citeturn12search1turn23view0  

libp2p（bootstrap/relay v2）:

- **multiaddrの組み立て・解釈を誤る（特にIPv6 + /p2p-circuit）**  
  - 症状: dialしても接続が張れない、アドレス解決が意図通りに働かない  
  - 原因: multiaddrは左→右へ積まれる。relay経由アドレスの順序が崩れると意味が変わる。citeturn0search10turn0search11  
  - 対処: `/ip6/.../(tcp|udp)/.../p2p/<RelayPeerId>/p2p-circuit/p2p/<DestPeerId>` の順序で作る、という原則で見直す。citeturn5search32turn5search2turn0search10  

- **bootstrapノードに到達できない**  
  - 症状: DHT参加に失敗してピア発見が進まない  
  - 原因: bootstrapノードは到達可能である必要がある（到達不能だと参加が詰む）。citeturn0search22  
  - 対処: bootstrapノードをIPv6で公開し、FW/ルーティングを確認。複数bootstrapを用意。  

- **relay v2が動かない（reservation無しで期待している等）**  
  - 症状: relay経由アドレスでdialできない  
  - 原因: circuit relay v2は宛先側がrelayへ予約を張る前提のモデルがあり、要素が欠けると成立しない。citeturn5search2turn5search32  
  - 対処: 宛先側がrelayに接続/予約を維持しているかを確認し、必要ならDCUtRで直接化を検討。citeturn5search24  

### 診断コマンド・ログ確認の実践セット

- iroh側のログ有効化: 公式Troubleshootingでは `tracing` を使い、`RUST_LOG=iroh=info` / `debug` で詳細化する手順が示されています。citeturn18view0  
- irohネットワーク診断: `iroh-doctor report` の出力例では `ipv6: true` や `global_v6`、relayレイテンシなどが示され、ホームリレー選定の状況も読み取れます。citeturn18view0  
- libp2p: 実装（rust-libp2p）によってログカテゴリは異なりますが、multiaddr組み立て不具合・重複等がissueとして報告されることがあり、該当症状では既知issueを参照して切り分けるのが有効です。citeturn0search6  

### 参考ソース（優先度順）

公式iroh/関連リポジトリ・ドキュメント:
- iroh crate docs（接続確立、relayの動作、EndpointId/EndpointAddr、QAD言及）citeturn16view0  
- iroh公式ドキュメント（DNS discovery、Custom relays、Troubleshooting/iroh-doctor）citeturn23view1turn23view0turn18view0  
- iroh-dns-server README（DNS/HTTP(S)の提供エンドポイント、/pkarr・/dns-query）citeturn27view2  
- iroh-dns-server CLI（`--config`）citeturn33view0  
- iroh-relay設定/デフォルトポート/ACL（Config/TlsConfig/AccessConfigと定数）citeturn15view0turn19view0  
- QAD移行の背景（STUN→QAD）citeturn18view1turn17search17  

libp2p公式/一次ソース:
- rust-libp2p（libp2p crate 0.56.0）citeturn6view1  
- circuit relay v2ガイド（libp2p公式ドキュメント）citeturn5search2  
- circuit relay v2仕様（libp2p specs）citeturn5search32  
- DCUtR/ホールパンチとrelay v2の位置づけ（IPFSブログ：TURN相当としての説明を含む）citeturn5search24  
- multiaddrの読み方（左→右に積む）citeturn0search10turn0search11  

日本語ソース（補助的）:
- DNSサーバをIPv6でlistenさせる一般的注意（entity["organization","JPRS","dns registry operator japan"] 技術資料）citeturn11search18  
- libp2pの概観（entity["organization","LayerX","japanese tech company"]の研究メモ）citeturn5search25  

組織・プロジェクト背景（最低限）:
- irohの主要開発元として entity["company","n0.computer","iroh developer company"] が公開インフラ（公開relays/DNS）を提供し、自己ホスト（dedicated relays/DNS）も推奨している旨が公式ドキュメントに明記されています。citeturn12search1turn23view0turn23view1
