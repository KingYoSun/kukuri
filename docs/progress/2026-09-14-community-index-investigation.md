# v0.2.3-preview.2 Community Index 調査

この文書は修正前の調査記録。後続の本番復旧・監視追加は[復旧記録](2026-09-14-community-index-recovery.md)を参照する。

## 範囲と判定

- 依頼: 検索・発見・おすすめが0件になる原因調査。VM・管理画面への接続、general / test / devへの検証投稿は許可済み。
- 調査日: 2026-09-14。主な観測は02:12–02:18 UTC（11:12–11:18 JST）。
- 本番: GCP `kukuri-cn` / `asia-northeast1-a` / `kukuri-cn-vm`、利用者向けAPI `https://api.kukuri.app`。
- API・indexerの稼働OCI revisionは `0aa3fe183006874eb004f18dceade9ca6125f992`。v0.2.3-preview.2の配布sourceと一致。
- ローカルHEADは `8fee1cbf`。以下に挙げる調査対象実装について、release tagとの差分がないことを確認。
- 本番への操作は参照のみ。設定変更、DB更新、再起動、検証投稿は実施していない。修正・復旧は未実施。

## 1. 主因: 既定トピックが索引対象に登録されていない

管理画面の「対応トピック」、Postgres `cn_index.supported_topics`、indexerのopened replicaログが一致した。

| 本番の索引対象 | 登録日（UTC） |
| --- | --- |
| `kukuri:topic:demo` | 2026-08-05 |
| `kukuri:topic:iroh` | 2026-08-05 |
| `kukuri:topic:nostr` | 2026-08-05 |
| `kukuri:topic:operators` | 2026-08-05 |

`kukuri:topic:general` / `kukuri:topic:test` / `kukuri:topic:dev` は0件。これらはrelease版の `apps/desktop/src/shell/slices/shared.ts:24` の `STARTER_TOPICS` と一致する利用者側の既定トピックだが、本番の索引対象とは同期されていない。

`crates/cn-indexer/src/participant.rs:150` の `desired_scopes` と `restore_scopes` は、DBの `list_supported_topics` を起点に同期対象を選ぶ。したがって、利用者のgeneralタイムラインに投稿が見えていても、このノードの検索対象には入らない。利用者のP2P購読とノードの索引対象登録は別の状態である。

画像の「このノードへの自分の索引登録申請はありません」は呼出し主の申請状態を示す。ノードが既定トピックを索引していることを示す表示ではない。

## 2. 別の観測: 既存対象の本文Blobも取得できず、索引全体が空

- `cn_index.index_entries`: 0行。
- 最新readinessによるArcadeDB件数: 0。`truth=0 projection=0`。
- indexer `/v1/status` の観測値:

```json
{"worker_running":true,"ingest_enabled":true,"opened_scopes":4,"last_sync_at":1789352027,"last_ingest_at":1789352027,"last_error":null,"last_error_scope":null,"scanned":432,"indexed":0,"skipped_non_allow":378,"scan_errors":0,"provider_unavailable":0,"deindexed":54,"media_fetch_success":0,"media_fetch_unavailable":0,"media_fetch_timeout":0,"media_fetch_oversize":0}
```

これらは起動後の累積処理回数であり、432件の異なる投稿があるという意味ではない。稼働ログには7つの異なるobjectについて、同じ本文取得失敗が各54回あった。

```text
failed to resolve post body; not indexing the post (fail-closed)
error=blob text body is not retrievable
ephemeral fetch remote transfer failed
error=io: stream reset by peer: error 3
```

取得先ピアへの接続成功ログの後に転送失敗があり、候補を使い切っている。特定のクライアントのUI検索語だけに依存する問題ではない。

例: object `303cf9c6dca389aa2f13a42f28828a5538c9f7aaec074d3a64b9c302de69c827` は2026-09-12 22:07:17 UTCの保存済み判定が `allow / no_known_match` だが、今回の走査では本文取得に失敗した。他の6対象も過去の `allow` 判定が存在する。

保存済み判定の集計はpost 8行・blob 3行で、すべて `allow / no_known_match`。今回の空結果を「有害判定で全件除外された」と説明する根拠はない。

処理箇所:

- `crates/cn-indexer/src/ingest/source.rs:160`: 署名等を確認した本文Blobを一時取得し、未取得なら失敗。
- `crates/cn-indexer/src/ingest.rs:297`: 本文取得失敗時は既存索引を除去し、`SkippedNonAllow` を返す。安全性スキャンより前に終了する。
- `crates/iroh-node/src/remote_fetch.rs:211`: ephemeral fetchの転送失敗後に他の候補を試し、全候補失敗なら `None`。

これにより、3機能に共通する検索用データが0件となる。`crates/cn-user-api/src/handlers/indexing.rs:370` 以降で、検索・発見・おすすめはいずれも同じ `IndexQuery` を利用する。

本文Blobが供給されないさらに下位の原因（所有端末の不在、Blob保持状態、配信側の問題等）は、このログだけでは確定していない。generalの投稿自体は索引対象外なので、この7対象の本文取得失敗をユーザー画像内の投稿に結びつけてはいない。

## 3. readinessで検出できない理由

02:12 UTCのreadinessは全項目passだった。`crates/cn-cli/src/commands/readiness_runtime.rs:90` の実装では次が成立する。

- opened scopeは1以上なら合格。既定3トピックの集合との一致は検査しない。
- freshnessはsync / ingestの時刻を見る。本文取得・索引登録の成功件数を必須にしない。
- scan coverageは不正な索引が公開されていないことを検査する。索引0件でも合格する。
- projectionへ接続でき、`projection <= truth` なら整合性確認に合格する。0対0でも合格する。
- 本文取得失敗は `SkippedNonAllow` に含まれ、上記の `scan_errors` / media fetchカウンタには表れない。

[今回のリリース記録](2026-09-14-v0.2.3-preview.2-release-rollout.md)でも、更新時に4 scopes / truth=projection=0で合格し、新しいlive投稿での検索成功は検証範囲外だった。これは本番の検索可能性を証明する検証ではなかった。リリースによって新しく壊れたかどうかは、今回の観測だけでは断定しない。

## 復旧・再発防止に必要な確認

1. default onboarding nodeの公開索引対象へ、既定3トピックを正式な運営操作で追加する。既存4件の削除は不要。
2. 既定トピックの所有端末を接続した状態で、既存投稿または許可済みの新規検証投稿がdocs同期→本文取得→allow判定→Postgres→ArcadeDBまで到達することを確認する。
3. 同じobject IDが日本語・ASCII検索、発見、おすすめで返ることを確認する。登録だけで復旧完了とはしない。
4. 既存7対象のBlob保有・配信状態を別途確認する。安全性判定の迂回、allowへの手動書換え、本文Blobの恒久保存は復旧策にしない。
5. default onboarding node固有の既定トピック確認と、本文取得失敗の計測・検索の実動確認を運用検証へ追加する。任意の空ノードを一律異常扱いする条件にはしない。

## 検証方法と制約

- GCP IAP経由SSH、管理listener `127.0.0.1:9090` のGET、Docker状態・revision、indexer status、journal/containerログ、SELECTによるDB確認を実施。
- 認証済みの利用者APIを新規identityで直接呼ぶ再現や、端末操作による投稿は実施していない。主因は本番の対象集合・実行ログ・release実装の一致から確定した。
- コード変更はなく、自動テストは未実施。本ファイルのみを追加した。
- 認証情報、管理画面のCSRF値、ピアのIPアドレス、生の投稿本文は本記録に含めない。
