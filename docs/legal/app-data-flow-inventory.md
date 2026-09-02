# kukuri アプリ データフロー突合表

最終更新日: 2026-09-02
対象: kukuri デスクトップアプリ legal bundle version 4

この表は、`docs/legal/privacy-policy.md` と `docs/legal/external-transmission-notice.md` を現行実装へ照合するための記録である。各 Community Node が独自に処理する内容は、その Node の manifest と公開文書を正とする。

| データ種別 | Canonical source／端末内保持 | 複製・送信先と契機 | 目的 | 保持主体・削除限界 | 実装・仕様根拠 |
|---|---|---|---|---|---|
| アカウントを識別する秘密鍵 | OS keyring または端末内 identity file。通常の network payload に含めない | 利用者が鍵 export を明示実行した場合だけ暗号化ファイルへ出力する | 署名、同じアカウントの復元 | 端末または利用者が保存した export。P2P copy の削除とは無関係 | `crates/desktop-runtime/src/identity.rs`、`docs/legal/account-key-export-data-classification.md` |
| 公開鍵・endpoint ID・接続情報 | 公開鍵は署名済み object と account state、endpoint ID／address hint は runtime・discovery state | 公開鍵は対象 replica の参加者、endpoint 情報は接続 peer、DHT、選択した Community Node／relay。IP address は実通信の相手方・経路事業者から観測可能 | 署名検証、peer discovery、P2P 接続確立、relay | 相手方・DHT・Node・relay の保持は各主体の方針に従う。端末削除で第三者の接続記録は消えない | `crates/transport/src/{config.rs,discovery.rs,iroh/**}`、ADR 0008／0009 |
| プロフィール・公開フォロー関係 | author-owned public docs replica。SQLite は再構築可能な投影 | 同じ author replica／topic を取得する P2P peer。対応する Community Node が索引に参加する場合は当該 Node | 公開プロフィール・social graph の表示、同期、検索・発見 | 受信 peer と参加 Node が copy／index を保持し得る。削除・訂正は対応実装が認識する範囲に限られる | ADR 0013／0015／0019、`crates/app-api/src/service/**` |
| 公開投稿・返信・再投稿・リアクション | 対象 topic／author docs replica、添付は blob。SQLite は投影 | 対象 topic の P2P peer。Node が supported topic の docs sync／index に参加する場合は当該 Node。hint は更新通知 | 表示、同期、返信、検索・発見、添付取得 | 受信済み peer、Node index／cache に copy が残り得る。撤回は network 全体からの即時完全削除ではない | ADR 0003／0004／0016／0017／0025／0032、`crates/docs-sync/**`、`crates/blob-service/**` |
| private channel | channel capability と暗号化対象 replica。local projection を持つ | capability を持つ参加 peer。利用者が private indexing を明示許可した場合だけ対象 Node | 限定 audience 内の同期・表示、明示時の索引 | 参加 peer と明示許可した Node。参加者が既取得の copy を遠隔完全消去できない | ADR 0018／0025、`crates/app-api/src/private_channels.rs`、`crates/desktop-runtime/src/community_node/indexing_request_support.rs` |
| DM | pairwise DM docs／blob と local projection | 指定した相手の peer。保管機能を明示利用する Node がある場合は当該 Node の個別文書に従う | 1対1メッセージ同期・添付取得 | 送受信端末。相手が取得済みの copy は送信側だけでは消去できない | ADR 0020、`crates/app-api/src/direct_messages.rs` |
| DHT・P2P・relay 通信 | runtime の discovery／connection state | Mainline DHT、接続 peer、構成された iroh relay。経路は `Direct P2P -> Relay Supported P2P -> Relay Fallback` | 到達先探索、hole punching、P2P transport、成立しない場合の実データ中継 | DHT／peer／relay 側の保持方針に従う。relay URL があるだけでは実データ fallback を意味しない | `crates/transport/src/iroh/**`、`docs/architecture/p2p-first-community-node-responsibility-boundary.md` |
| Community Node 認証・consent・rendezvous | Node token／Node 別同意は端末 secure storage、subscriber／consent／presence は Node 側 | 利用者が保存し同意した Node。認証時の公開鍵・proof、session metadata、topic rendezvous key、接続用 endpoint 情報 | 認証、consent、bootstrap assist、topic rendezvous、Node capability | local token／同意と Node 側 record。topic presence は TTL。詳細は各 Node の manifest／privacy／retention | ADR 0009、`crates/desktop-runtime/src/community_node/**`、`docs/legal/community-node-legal-documents-data-classification.md` |
| Community Node 検索・索引・通報・tester feedback | Node-local index／case／feedback record。client は結果・local observation を保持し得る | 利用者が選択した Node。検索 query、対象識別子、indexing request、通報理由・任意詳細・任意連絡先、feedback 内容 | 検索・発見、明示的な索引参加、通報、安全性対応、Preview 改善 | 当該 Node の retention と法的保全に従う。他 Node や network 全体へ自動適用されない | ADR 0025／0030／0033／0039、`crates/desktop-runtime/src/community_node/{index_query_support.rs,indexing_request_support.rs,report_routing_support.rs,tester_feedback_support.rs}` |
| アプリ同意・18歳以上の自己申告 | `<db_path>.app-consent.json`。slug／version／時刻／表示言語／app version と年齢自己申告の事実だけ | network へ送信しない。端末 backup／restore でも再同意・再申告を求める | network 開始前 gate、重要変更時の再同意 | 端末内のみ。生年月日・身分証を収集しない | `apps/desktop/src-tauri/src/{state.rs,commands/app_consent.rs}`、`docs/legal/{app-consent,age-attestation}-data-classification.md` |
| 自動更新確認 | update state は memory。manifest URL は Tauri 配布設定 | 同意後の shell 起動時と30分ごと、および手動確認時に GitHub Releases へ request。IP address、HTTP／TLS 上必要な request metadata、現在版／platform に応じた updater request が相手方から観測可能 | 新しい署名済み Preview build の確認・取得 | GitHub と経路事業者の方針に従う。kukuri は取得結果と error を runtime 中だけ保持 | `apps/desktop/src-tauri/tauri.conf.json`、`apps/desktop/src/shell/{DesktopShellPage.tsx,useAppUpdateStore.ts}` |
| 診断レポート | UI が現在の state から生成。アプリ版、platform／user agent、接続・delivery／discovery path、peer／topic 件数、未読件数、更新・通知状態、Node URL／session／retry／error | 生成・preview・clipboard／file export は local。利用者が GitHub 等へ添付した場合だけその送信先 | Preview 不具合の調査 | export 後は利用者が選んだ送信先。秘密鍵、auth token、private capability、invite／share token、DM 本文、local DB path は既定 report に含めない | `apps/desktop/src/lib/releaseReadiness.ts`、`apps/desktop/src/components/settings/ReleasePanel.tsx` |
| 行動分析・自動 crash report | 現行アプリは収集基盤を持たない | 送信しない | 該当なし | 導入前に本文書改訂と必要な同意を行う | `apps/desktop/src-tauri/tauri.conf.json`、Community Node capability は各 Node manifest で別途開示 |

## 確認上の境界

- `api.kukuri.app` は現行配布物の初期候補であり、汎用 runtime の固定宛先ではない。利用者は候補を削除・置換できる。
- `iroh-relay.kukuri.app` を含む relay URL は、選択した Node の connectivity metadata や runtime config から得る。任意 Node が別の relay を提示し得る。
- Community Node 運営者と kukuri 運営者が同一とは限らない。Node ごとの受信項目、保持、外部送信、削除・通報は当該 Node の公開文書を確認する。
- P2P で受信済みの第三者 copy、第三者 Node の index／cache、経路事業者の log を kukuri アプリから一括消去することはできない。
