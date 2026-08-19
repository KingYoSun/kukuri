# 設計判断記録 0030: 内容の観測元記録

## 状態
承認済み

## 日付
2026-08-14

## 関連
- #310
- #612
- #666
- #684
- #692
- `docs/adr/0002-feature-data-classification-template.md`
- `docs/adr/0027-deterministic-moderation-critical-safety.md`
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`

## 機能のデータ分類
- `Feature 名`: 内容の観測元記録
- `Durable / Transient`: 端末内で永続。索引結果だけで端末内に内容がない場合は一時扱いとし、保存しない
- `Canonical Source`: 端末内の `content_observations`
- `Replicated?`: しない
- `Rebuildable From`: 再観測したコミュニティノードの応答。共有文書だけからは再構築できない
- `Public Replica / Private Replica / Local Only`: 端末内限定
- `Gossip Hint 必要有無`: 不要
- `Blob 必要有無`: 不要
- `SQLite projection 必要有無`: 必要
- `必須 contract`:
  - `content_observation_requires_local_subject`
  - `content_observation_upsert_refreshes_timestamp`
  - `content_observation_restores_after_restart`
  - `content_observation_expires_after_time_passes_without_another_write`
  - `post_view_exposes_content_provenance`
  - `expired_content_observations_do_not_reach_post_profile_or_attachment_views`
  - `unknown_provenance_has_no_report_candidate`
  - `report_manifest_loads_without_settings_visit`
- `必須 scenario`:
  - `community_node_report_routing`

## 判断
- 保存するのは対象種別、対象識別子、観測元ノードの基底アドレス、観測能力、最終観測時刻だけとする。
- `source_peer`、接続元アドレス、認証情報、投稿本文、添付本体、`CommunityNodeManifest`、解決済み通報先は保存しない。
- `community_index` は、応答元ノードと対象識別子の対応を応答そのものから確認できるため、観測事実として扱える。
- `bootstrap_assist` と `relay_assist` は、対象内容まで一意に対応付けられる情報が得られた場合だけ記録する。現行経路では対応を証明できないため記録しない。
- 手動接続、`DHT`、直接接続、単なる待ち合わせ参加からコミュニティノード由来を推測しない。
- 索引応答を受けても、対象の投稿またはプロフィールが端末内に存在しない場合は永続化しない。
- 同じ対象、ノード、能力を再観測した場合は最終観測時刻だけを更新する。
- 保持期間は最終観測から90日、総数は2048件を上限とする。ちょうど90日の記録は残し、90日を超えた記録を期限切れとする。
- 期限切れ記録は書き込み時の整理に加え、観測記録の読み取り時にも現在時刻で判定する。読み取りでは期限切れの物理削除と対象記録の取得を同じ取引または排他範囲で行い、削除対象を返さない。これにより、新しい観測がない端末でも保持期間を強制する。
- 2048件の上限を超えた場合は、引き続き最終観測時刻が古い記録から削除する。
- 投稿の端末内射影が削除された場合、その投稿の観測記録も削除する。プロフィールは端末内プロフィールが存在する間だけ記録できる。
- 添付は親投稿の観測元を表示時に引き継ぎ、正本を `blob` とする。添付単位の観測記録は保存しない。
- 通報先は保存せず、通報を開いた時に観測元の基底アドレスから最新の `CommunityNodeManifest` を取得して求める。
- 観測元不明、ノード情報の取得失敗、能力または責任範囲の不一致では候補を作らず、既定ノードへ代替しない。
- 能力と責任範囲の一致は次の対応表で判定する(#702)。通報能力ごとに、公開ノード情報の `capability_scope.available_enabled` に提供中能力キーが含まれ、かつ `authority_scope.applies_to` に責任範囲の語彙が含まれる場合だけ候補にする。`planned_enabled` だけの能力、失効した能力、`this_node` だけの責任範囲は候補にしない。`does_not_apply_to` に通報能力名または責任範囲語彙が含まれれば明示的否認として除外し、`network_wide_authority` を僭称するノードも除外する。通常通報と異議申し立て(`trust_signal`)は同じ判定を使う。

  | 通報能力 | 提供中能力キー(いずれか) | 責任範囲の語彙 |
  |---|---|---|
  | `community_index` / `recommendation` | `community_index` | `communities_indexed_by_this_node` |
  | `moderation` | `moderation` | `moderation_events_issued_by_this_node` |
  | `trust_signal` | `community_local_trust` | `trust_signals_issued_by_this_node` |
  | `media_cache` | `blob_cache` | `media_cached_by_this_node` |
  | `bootstrap_assist` | `bootstrap_assist` | `this_node`(観測記録の生成経路が整うまで) |
  | `relay_assist` | `iroh_relay` または `traffic_relay_fallback` | `this_node`(同上) |

## 影響
- 観測記録は共有文書や他端末へ流れず、利用者の受信経路を新たに公開しない。
- コミュニティノード経由を証明できない内容は通報先候補を持たないため、端末内ミュートだけを案内する。
- 新しい受信経路で観測元を記録する場合は、対象内容との一意な対応を契約試験で先に固定する必要がある。
