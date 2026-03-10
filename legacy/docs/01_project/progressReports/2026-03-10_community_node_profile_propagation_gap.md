# Community Node プロフィール伝播の実機差分とテストギャップ監査

作成日: 2026年03月10日
最終更新日: 2026年03月10日

## 1. 概要

- 実機では、`https://api.kukuri.app` 配下で peer 間接続、投稿伝播、realtime mode の timeline 自動更新までは確認できた。
- しかしプロフィール伝播は失敗しており、相手側 timeline/thread の display name / avatar が反映されない。
- 現行の `community-node.profile-propagation.spec.ts` はこのユースケースを担保しておらず、「実機無視のテストPASS」が発生している。これは重大インシデントとして扱う。

## 2. 実機検証結果

### 2.1 成功した点

- `https://api.kukuri.app` を設定した実機で、peer 間接続は成立した。
- 投稿伝播は成立し、realtime mode の timeline も自動更新された。

### 2.2 失敗した点

- 両クライアントで、あらかじめ自分自身のプロフィールだけをローカル保存した状態から投稿すると、相手側で author の display name / avatar は解決されなかった。
- 今回の実機検証には profile update アクションは含まれていない。したがって、session 中の metadata push に依存した確認は要件を満たさない。

### 2.3 同時に観測された relay warning

- 実機ログ
  - `2026-03-09T16:17:13.991662Z ERROR nostr_relay_pool::relay::inner: Connection failed. url=wss://api.kukuri.app/relay error=HTTP error: 404 Not Found`
  - `2026-03-09T16:17:15.424673Z WARN kukuri_lib::presentation::commands::community_node_commands: Failed to apply community node Nostr relay configuration error=No configured Nostr relay connected within 3s: wss://api.kukuri.app/relay relay_count=1 reason="authenticate"`
- ただし、この warning が出ていても P2P の post 伝播と realtime timeline 更新は継続していた。

## 3. 現行テストの監査

### 3.1 `community-node.profile-propagation.spec.ts` が見ているもの

- local docker の Community Node 環境を使う。
  - `scripts/test-docker.ps1` は `E2E_COMMUNITY_NODE_URL=http://127.0.0.1:18080` と `E2E_COMMUNITY_NODE_EXPECTED_RELAY_URL=ws://127.0.0.1:18082/relay` を注入する。
- test は明示的に profile update アクションを実行する。
  - settings を開き、`profile-display-name` と `profile-about` を更新して submit する。
- 観測点は 2 つだけである。
  - 自クライアントの `getAuthSnapshot()` が更新 display name を返すこと。
  - `waitForPeerHarnessSummary()` が listener peer の `recent_contents` に更新後 display name / about を含むこと。

### 3.2 その test で担保できていないもの

- `api.kukuri.app` のような公開 Community Node / 公開 relay 構成
- profile update を伴わない通常投稿シナリオ
- もう一方のクライアント UI で、timeline/thread の author 表示名・avatar が解決されること
- 既存 profile metadata を post 受信側が取得・再利用できること

### 3.3 他の Community Node E2E も代替になっていない

- `community-node.timeline-thread-realtime.spec.ts` は propagated post の本文が realtime 描画されることだけを見ており、author 表示は検証していない。
- 一部の Community Node test fixture は `registerE2EBridge.ts` で `authorDisplayName` を直接埋め込んだ post を seed できるため、profile 解決の欠落を隠し得る。

## 4. 実装観点で見たギャップ

- `useP2PEventListener.ts` は author profile 解決時に、まず `TauriApi.getUserProfileByPubkey()` / `getUserProfile()` でローカル保持プロフィールを引く。
- 取得できない場合の fallback は、受信 payload を metadata(kind=0) として parse した内容であり、既存プロフィールの network side lookup ではない。
- `postMapper.ts` も同様にローカル保存プロフィールへ依存している。

このため、「各クライアントが自分自身のプロフィールしかローカル保存していない」実機ユースケースでは、明示的な profile update / metadata push がない限り相手プロフィールを解決できない可能性が高い。

## 5. 結論

- 現時点で、プロフィール伝播について実機相当の検証ができているとは言えない。
- `community-node.profile-propagation.spec.ts` の PASS は、今回の実機シナリオに対する品質保証にはならない。
- 本件は「実機無視のテストPASS」が発生した重大インシデントとして扱い、live-path に対応した test gap fill を優先する必要がある。

## 6. 2026年03月10日の対応

- Rust 側で、topic post publish 時に保存済みローカル profile metadata を同一 topic へ先行 broadcast する処理を追加した。
- `community-node.profile-resolution.spec.ts` を追加し、profile update を伴わない 2-client 実機相当シナリオを固定した。
  - 前提: publisher / listener がそれぞれ自分自身の profile をローカル保持済み
  - 操作: publisher が通常の topic post を publish
  - 期待: listener 側の timeline/thread で author display name / avatar が自動解決される
- 同 E2E では peer harness summary の固定 count 前提を避け、post-time metadata propagation が timeline/thread 描画へ反映されることを直接 assertion する。
- 検証結果:
  - `./scripts/test-docker.ps1 rust`: PASS
  - `./scripts/test-docker.ps1 ts`: PASS
  - `./scripts/test-docker.ps1 e2e-community-node`: PASS
  - `gh act --workflows .github/workflows/test.yml --job format-check`: PASS
  - `gh act --workflows .github/workflows/test.yml --job native-test-linux`: PASS
  - `gh act --workflows .github/workflows/test.yml --job community-node-tests`: PASS

## 7. 残る確認事項

- `https://api.kukuri.app` を使った実機再検証が未了である。
- relay 404 / auth warning は依然として別途切り分けが必要であり、profile 伝播修正後も残る可能性がある。
- 次の live-path 確認では、profile update アクションなしで受信側 timeline/thread の display name / avatar が反映されることを実機で再度確認する。
