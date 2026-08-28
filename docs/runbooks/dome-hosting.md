# Dome Hosting 運用

## 有効化

Community Node の `features.dome_hosting` は既定で無効。利用する Node だけ `true` にし、`COMMUNITY_NODE_DOME_HOST_SIGNING_KEY` に Node 専用の kukuri secret key を設定する。起動時に公開鍵が manifest の `node_id` と一致しない場合、user-api は fail closed で起動を拒否する。

Hosting API は既存の bearer authentication と consent gate の内側にある。lease、acceptance、activation、close はログへ本文を出さず、bearer token と participant raw input も記録しない。

## 割当と切替

1. Dome owner が desktop の Hosting panel で owner device または Community Node を明示選択する。
2. CN 委譲では client が owner-signed lease と exact Instance/Preset manifest bundle を `/v1/dome-hosting/assignments` へ送る。
3. Node の署名済み acceptance を owner が Context replica に追記し、owner activation を `/v1/dome-hosting/activate` へ返す。
4. `CommunityNodeHosted` になった後だけ Node が input を受理し、host-signed snapshot を返す。

owner が online に戻っても自動 reclaim はしない。「この端末でHostingを開始」は epoch を進める明示 switch-back/renew であり、旧 Node の lower epoch output は client 検証で拒否される。

## 確認と終了

- `GET /v1/dome-hosting/status/{instance_id}` で state、epoch、session、participants、sleep、expiry を確認する。
- participant が 0 の間も assignment は lease expiry/close まで残り、physics だけ sleep する。
- desktop の「Hostingを終了」は owner-signed close を canonical replica に追記してから Node の `/release` を呼ぶ。Node に到達できなくても canonical close と expiry が authority を失効させる。

## Prop とレイアウト保存

- persistent prop の追加・削除は owner だけが active session に送信できる。session 中の位置・回転は一時状態であり、「現在のレイアウトを保存」を実行するまで manifest は変わらない。
- guest prop は参加者が追加でき、5分の wall-clock TTL または session 終了で消える。layout candidate と durable manifest には含めない。
- 保存時は active host が physics tick 境界で persistent prop だけを抽出して署名し、owner が候補を検証・署名した後に新しい manifest revision をpublishする。同一レイアウトはno-op、変更を伴う保存は30秒に1回まで。
- 保存成功後は同じhost targetでlease epoch/sessionを更新する。新sessionは保存されたtransformから速度0、grab/seatなし、guestなしで開始する。
- late joinまたは欠落検知時はdesktopの「Physics snapshotを再同期」で最大100件のmemory-only ringから再取得する。ringより古いsequenceの場合は最新baselineを返す。

## Manifest/asset cache

- 管理対象はmetaverse manifestとそこから参照されるasset blobだけ。physics snapshot、session state、DB metadata、GPU resource、metaverse以外のblobは容量計算に含めない。
- 既定上限はdesktop 1 GiB、Community Node 10 GiB。content hashで重複排除し、`staging`、`current`、`active_lease`、直近3版の`rollback`参照をpinする。
- pinが一つでもあるblobは削除しない。全参照解除から24時間のgrace後にだけlocal GC対象とする。容量不足時もcurrent/active/stagingを退避せず、新規stagingを失敗させる。
- P2Pで既に取得された他peer上のコピーは強制消去できない。保証範囲は新規配布の停止と各nodeのlocal unpin/GCまで。

## Resource budget

- owner desktopは`KUKURI_METAVERSE_RESOURCE_BUDGET_JSON`、Community Nodeは`COMMUNITY_NODE_METAVERSE_RESOURCE_BUDGET_JSON`へ[ADR-0041](../adr/0041-metaverse-resource-budget.md)の完全なJSON objectを設定する。部分object、不正値、安全上限超過は起動失敗になる。
- `GET /v1/dome-hosting/status/{instance_id}`の`resource_budget`で適用値、`resource_metrics`で拒否総数/code別件数、participant/rigid-body high-water、snapshot bytes/throttleを確認する。metricにplayerやassetの識別子は含まれない。
- `METAVERSE_*_RATE_EXCEEDED`はwindow経過後に再試行できる。`LIMIT_EXCEEDED`または`UNVERIFIED_ASSET`はasset/config/scene構成を修正してから再割当する。上限引上げはADRのhard ceiling内に限定する。
- Clientの`reduced` / `fallback` / `minimal`表示はlocal描画budgetによる段階的劣化で、host session停止を意味しない。current Domeの基本操作を確認し、optional texture/avatar/propを減らすかlocal budgetを安全範囲内で調整する。

## 再起動と障害復旧

Node 再起動時は有効な lease と保存済み manifest bundle を再検証し、新しい session id、manifest initial transform、velocity 0、grab/seatなし、guest propなしで開始する。

`GracePeriod` または split-brain を観測した場合は、秘密情報を採取せず Context/Instance、lease epoch/digest、target host、session、last heartbeat、rejection reason を確認する。owner が同一 host を再開するか、より高い epoch で明示切替する。DB 行や replica record の手動書換えは行わない。
