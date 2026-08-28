# ADR-0039: Dome prop、layout commit、manifest/asset保持

- Status: Accepted
- Date: 2026-08-28
- Issue: #793

## Context

Dome Hosting Lease が選ぶactive hostはsession中のphysics authorityだが、Domeのdurable world authorityはownerである。実行中のprop transformを自動保存すると、guestの持ち込み、速度、grab、collision途中状態までowner資産へ混入し得る。一方、late join/resyncには短期snapshotが必要であり、manifestとassetは版をまたいで再利用しながら、参照がなくなったlocal copyを安全に解放する必要がある。

## Decision

### Prop lifecycle

- `persistent prop`はowner管理のDome Presetに属する。ownerだけがsession中の追加、削除、初期定義変更を要求できる。
- Manifestはpersistent propのasset/primitive、position、rotation、scale、visual/interaction/colliderを初期physics定義として保持する。session開始時のlinear velocityは常に0、grab/seat/collision途中状態は常に空とする。
- `guest prop`は参加者がactive hostへ送るsession inputで生成する。絶対wall-clock expiryを必須とし、TTL到達またはsession終了で破棄する。docs、SQLite/Postgresのcanonical/derived record、manifest blobへ保存しない。
- host process restart、新lease activation、layout commit後のapply-and-restartはexact current manifestから新sessionを作り、guest、snapshot、velocity、grab/seatを復元しない。

### Layout commit

1. ownerが一意なoperation idで明示的にcommitを要求する。
2. active hostがphysics tick境界まで進め、active lease epoch/session/manifest revisionへ束縛したlayout candidateへ署名する。
3. candidateはpersistent propだけを抽出する。position、rotation、既存scale、session中にownerが追加/削除したpersistent definitionを含め、guest/avatar、velocity、grab/seat/collision途中状態を含めない。
4. ownerはhost署名とactive leaseを検証し、正規化したdurable contentを現在版と比較する。同一ならno-opとし、新blob/revisionを作らない。
5. 直前の成功commitから30秒以内の新規commitは拒否する。retryは同じoperation idで冪等に再開できる。
6. 変更時はownerがrevisionを1増やしたDome Presetへ署名し、manifest/blobをstageしてcurrent pointerをpublishする。Community Nodeがcandidateを生成してもowner署名なしではdurableにならない。
7. Active Domeは同じhost targetに対してlease epochとsessionを更新し、新manifestの初期状態から再開する。stage前の失敗は旧版を継続し、publish後の失敗は旧hostをfenceした`Transferring`としてretryする。

Metaverseは実験機能のためworld schemaを5へ上げ、旧experimental schemaとのmigration/互換decodeは提供しない。`manifest_version`はschema versionではなくPresetの単調増加revisionを表す。

### Snapshot retention

- active hostは10Hzで署名snapshotを生成し、最大100件の`VecDeque`だけに保持する。
- resyncは同じlease epoch/sessionの`after_sequence`より新しいsnapshotを返す。履歴外の場合はring内の最新snapshotを返し、clientはそれを新baselineとする。
- snapshot ringはmemory-onlyで、owner device、Community Nodeとも再起動・session終了時に全件破棄する。

### Manifest/asset retention

- Dome Preset manifestと参照assetはBLAKE3 content hashで保存し、同一bytesを版ごとに複製しない。
- pin理由は`current`、`active lease`、`staging`、`rollback`とし、manifestからassetへの参照グラフ単位で保持する。rollbackはcurrentを除く直近3revisionとする。
- 参照が0になったmetaverse管理blobへ`unreferenced_at`を記録し、24時間のgrace後にだけunpin/local GC候補にする。
- desktopのmetaverse manifest/asset blob cacheは1GiB、Community Nodeは10GiBを固定上限とする。physics snapshot、session state、DB metadata、GPU resource、metaverse以外のblobはこの容量に含めない。上限超過時はunpinned候補をlast-accessed順に削除する。
- Iroh GCは明示的なmetaverse候補だけを削除対象にし、それ以外の全hashをprotectedとしてone-shot実行する。一般blobへglobal automatic GCを適用しない。
- P2P peerが既に取得したcopyの強制消去は保証しない。kukuriが制御するpin/再配布を停止し、各nodeでlocal GCする。

固定上限の設定可能化、運用metrics、描画/physicsの段階的degrade、悪意あるassetに対する包括的budgetはIssue #794で扱う。

## Feature Data Classification

| Data | Authority | Canonical store | Sync / transport | Local / Node cache | Retention / delete |
| --- | --- | --- | --- | --- | --- |
| Preset revision/current pointer | owner signature | owner author replica | docs sync | SQLite projection | append-only revision。current/rollback参照に従う |
| Preset manifest/asset blob | owner署名manifest + content hash | content-addressed blob | blob P2P / authenticated CN staging | desktop/CN metaverse blob cache | pin reason、24h grace、unpinned LRU/local GC |
| layout candidate | active host signature | なし | owner-host session API | operation中のmemory | commit/no-op/reject後に破棄 |
| layout commit operation | owner | owner device local store | なし | SQLite operational state | 完了/失敗診断と冪等retryに必要な期間保持 |
| guest prop | active host | なし | session input/snapshot | host/client memory | wall-clock TTLまたはsession終了 |
| physics snapshot ring | active host signature | なし | session stream/resync | host/client memory最大100件 | ring上書きまたはsession終了 |
| CN pin/cache ledger | owner/lease/manifestから導出 | なし | authenticated assignment/staging | Postgres + local blob store | assignment/revision参照とgrace/LRUに従う |

秘密鍵、bearer token、raw participant inputは保存・診断出力しない。診断可能な値はoperation id、instance/revision/epoch/session、content hash、pin reason、byte size、last access、GC結果とreject reasonに限定する。

## Consequences

- 通常のinteractionはdurable layoutを暗黙更新しない。
- Owner不在のCommunity Nodeはphysicsを継続できるが、新しいdurable revisionを確定できない。
- Current/active/stagingは容量超過時にも保護され、容量不足は新規stageの明示的な失敗として扱う。
- `cn-iroh-relay`は引き続き純粋なiroh relayであり、manifest/asset cacheは`cn-user-api`側の責務とする。
