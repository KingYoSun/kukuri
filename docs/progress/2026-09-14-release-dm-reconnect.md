# リリース前の直接DM再接続修正

## 目的・対象外

完了。v0.2.3-preview.1の公開前検証で発見した、相手の停止中に送信したDMが再起動後に再接続しない問題を修正した（#1011、PR #1012）。relayを通常経路へ強制する変更、終了時のblob保存削除、testのtimeout延長・skip・条件弱体化は対象外。

## 固定条件

- リスク区分C / Scope revision: 2026-09-14-v1 / 基準: `165919cf09ae8f30ed45d970aef7a9e55a2cab38`。
- AC-1: 他ALPNの直接接続がactiveでも、未接続gossipのwarmupを実行する。実endpointで別ALPNを接続した状態の回帰testで確認。
- AC-2: 既存3 CLI daemon testの停止中送信→同一identity再起動→DM配送・私有境界が成立する。同じ既存testと失敗箇所を維持する。
- AC-3: 同一peerの同時dial抑制、全体2並列上限、direct／relayの既存backoffを保持する。既存coalescing／RAII／backoff testsとrelay connectivity testsを確認する。
- INVAR-1: wire、認証・同意・private境界、identity、blob保存と終了完了保証、Direct P2P→Relay Supported P2P→Relay Fallbackを変えない。

## Inventoryと遷移

| ID | 入口→helper→sink | 条件・検証 |
| --- | --- | --- |
| INV-1 | ensure_hint_topic→warmup_peers_once→warmup_peer→Endpoint::connect / Gossip::handle_connection | 新規購読・再購読、AC-1/2 |
| INV-2 | extend_active_topic_peers→warmup_peers_once→warmup_peer→同sink | discovery / ticketからの追加、AC-1/3 |
| INV-3 | warmup_peer→try_mark_peer_in_flight / Semaphore | 同一peer重複と全体2並列、AC-3 |
| INV-4 | TopicWarmupInFlightGuard::drop | 成功・失敗・取消時のin-flight解除、AC-3 |

sinkの逆引きは上記2 warmup caller。変更pathはcrates/transport/src/iroh/topics.rsと本文書。mod.rsのSemaphore上限は変更しない。

| 遷移 | 期待 |
| --- | --- |
| 別ALPN active / gossip未接続→warmup | 別ALPNのActive経路をgossip接続済みと誤認しない |
| 同一peer warmup中→重複要求 | 追加dialを抑制 |
| permit待ち→permit取得→接続 | 全体並列上限を保持 |
| 接続失敗／取消→次の既存backoff | guardを解放し再試行可能 |
| peer停止→port変更再起動→ticket更新 | 既存DMを再送生成せず元outboxから配送 |

## 再現と実施順

- main Fast 34771230688、Release 34771260824の初回が同じCLI DM再接続timeout。Linux単独の未変更testでも2回失敗し、backtraceでprocess_e2e.rs:534と確認。
- 前回公開版v0.2.2-preview.1は同じLinux環境で4回成功。終了前blob flushを外す比較は成功したが、保存保証を落とすため採用しない。受信task終了待ちだけでは繰り返し中に失敗し不採用。
- 別ALPNも含むActive経路判定を外す比較は既存DM testが3回成功。まだ最終修正の証明ではなく、AC-1の失敗testで原因を固定してから実装する。
- T1: AC-1の実endpoint testを変更前に失敗させる。T2: 最小修正とAC-1/2/3 targeted validation。T3: rust-test／community_node_public_connectivity等のpath必須CI。T4: 固定headで独立監査、CI成功後にmerge。
- 元公開run attempt2のRust testsは成功したが、ローカルで具体的な再現が残るため、後続の公開を止める目的でworkflowを取消。取消時の画面検証は未完了として保持する。

## 未決事項

既存tag／公開assetの上書きは禁止されているため、修正済みsourceの公開tagは修正と検証が揃ってからユーザーの指定を確認する。既存v0.2.3-preview.1のノードimageは上書きせず、VMも旧版を維持する。

後続でユーザーがv0.2.3-preview.2を指定し、この判断は解消済み。既存tagを保持したまま[修正版の全クライアント・CN公開とVM更新](./2026-09-14-v0.2.3-preview.2-release-rollout.md)まで完了した。

## 実装・検証

- Activeなtransport addressをgossip接続済みの根拠に使う早期returnを除去。新しいALPN、relay URL、peer、再試行schedulerは追加しない。既存のwarmup条件、同一peer in-flight guard、Semaphore 2並列、direct／relay backoffを使う。
- AC-1: `active_other_protocol_does_not_suppress_gossip_warmup` は実endpointで別ALPNの接続がActiveなことを先に確認し、gossipのALPNでの着信を要求する。変更前は3秒timeoutでFAIL、変更後はPASS。
- AC-2: 同じ既存CLI testは早期returnだけを除いた比較で3回連続PASS。最終差分で再確認する。
- AC-3 / INVAR-1: Linux `cargo test -p kukuri-transport --lib -- --nocapture` は53件PASS。既存のdirect、seeded DHT、relay、stale addr、3 clients multiple topics、coalescing、RAII、backoff testsを含む。終了時に上流gossipのcancelled task panicが観測されたため、比較起点での発生有無も確認する。
- fmtの初回checkで新testの1式の整形差分が出たため、整形して再checkする。Rust全suiteと実community-node scenarioはPR CIで補う。
- CIと独立監査は未完了。未分類inventoryは0、入口・sinkの追加削除は0。

## 完了記録

- 監査head `ff1d4fdb29876accedae873db4d92673fdd7a869`。fmt／fmt-check／diff-check成功。最終版の既存DM testが25.49秒で成功。
- 上流cancelled task panicは前回公開版のtransport 52件PASS（6.53秒）でも発生し、今回のRegressionではないと確認。
- [独立監査PASS](https://github.com/KingYoSun/kukuri/pull/1012#issuecomment-5655163842): inventory 4件すべて適合、不適合・未分類・blocker 0。source、caller/sink、実endpoint旧FAIL→新PASS、最終DM成功、53件成功の生ログを別コンテキストで再構築した。
- PR #1012の全13 checks成功: [Fast 34774138884](https://github.com/KingYoSun/kukuri/actions/runs/34774138884)、[CLI 34774139023](https://github.com/KingYoSun/kukuri/actions/runs/34774139023)、[Linux Package 34774139101](https://github.com/KingYoSun/kukuri/actions/runs/34774139101)。Rust全suite、community-node connectivity、Windows package、Linux AppImage／Debとupdater回復を含む。
- merge `0aa3fe183006874eb004f18dceade9ca6125f992` と監査headのtreeは `4f6035e1410374176c98c8f086825c0e063820a7` で一致。#1011をCompleteとしてCloseした。
- この完了は修正の実装・検証・マージに対するもの。リリースの版番号・公開・VM反映は[リリース記録](./2026-09-14-v0.2.3-preview.1-release-rollout.md)で別に追跡する。上の途中経過の未完了記述は本節で解消した。
