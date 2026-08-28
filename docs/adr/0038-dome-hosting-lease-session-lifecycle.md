# ADR-0038: Dome Hosting Lease と session lifecycle

- Status: Accepted
- Date: 2026-08-28
- Issue: #788

## Context

Dome の Durable world definition は owner が管理する。一方、実行中の avatar / prop physics は低遅延な単一 authority が必要である。owner 不在時も Dome を公開できるように、owner device または owner が選んだ Community Node の一方だけを active host とする必要がある。

## Decision

### Authority

- Durable world authority は owner であり、Preset、Instance、Hosting Lease、activation、renew、close を owner が署名する。
- Ephemeral physics authority は active Hosting Lease が指す host である。
- Community Node は lease と manifest bundle の operational mirror を持てるが、canonical owner または Durable world source にはならない。
- canonical Hosting record は Instance が所属する SpatialContext replica の append-only object とする。gossip と WebSocket は通知・転送、SQLite と Postgres は再構築可能な projection / operational mirror とする。

### Lease と一意性

`DomeHostingLeaseV1` は lease id、SpatialContext、Instance id / generation、owner、host target、manifest blob hash / version、issued / expiry、単調増加する epoch を含む。lease は owner-signed envelope として検証する。

host 切替は次の二段階で行う。

1. owner が前回より大きい epoch の lease を発行する。この時点で旧 host を fence し、状態を `Transferring` とする。
2. target host が lease digest、epoch、新しい session id を署名して accept し、owner がその acceptance を明示的に activate する。

activation 前の target と、新 epoch 発行後の旧 host は authoritative output を発行できない。owner が online に戻っただけでは Community Node の lease を変更しない。owner device への切替も同じ transfer を使う。

lower epoch、期限切れ、改ざん、Instance generation / manifest hash 不一致を拒否する。同一 Dome / epoch に異なる有効 lease がある場合は split-brain として fail closed にし、どちらも active にしない。owner がさらに大きい epoch を発行した場合だけ復旧する。

### 状態

- `Closed`: active lease がない、close 済み、または期限切れ。
- `Transferring`: 新 epoch の lease はあるが、host acceptance と owner activation が揃っていない。
- `Owner Hosted`: owner device target が activate 済み。
- `Community Node Hosted`: Community Node target が activate 済み。
- `Grace Period`: activate 済み lease に対する heartbeat が失われた、または split-brain を検出した fail-closed view。新 host 権限は発生しない。同一 host の再開か owner の新 epoch 操作でのみ解消する。

### Session lifecycle

- host は activation ごと、または process restart ごとに新しい session id を発行する。
- session は active lease epoch、Instance generation、manifest hashへ束縛する。
- participant が 0 人なら physics step を sleep する。ただし wall-clock sweeper は継続し、guest prop TTL を失効させる。
- Community Node hosted session は participant が 0 人でも lease expiry または owner close まで存続する。
- host restart は exact manifest bundle の initial state から開始し、velocity、grab / seat、transient guest prop、過去 snapshot は復元しない。
- manifest 変更を実行中 session に暗黙適用しない。owner の apply-and-restart が新 epoch と新 session を開始する。

### Physics protocol

participant は movement、grab、throw、push、sit を lease epoch / session / sequence に束縛した署名済み input として host に送る。peer が送った transform / object state は authoritative state として採用しない。

host は検証済み input を共通 Rust runtime へ適用し、host-signed snapshot を配信する。client prediction / interpolation は表示上の補助であり、active host signature、lease epoch、session id、sequence の検証に失敗した snapshot を破棄する。

## Feature Data Classification

| Data | Authority | Canonical store | Sync / transport | Local / Node cache | Retention / delete |
| --- | --- | --- | --- | --- | --- |
| Hosting Lease / activation / close | owner signature | SpatialContext replica | docs sync、gossip hint | SQLite projection、Postgres operational mirror | append-only。Instance tombstone / move後は無効として保持し、通常GC規則に従う |
| host acceptance | target host signature | SpatialContext replica | docs sync、gossip hint | SQLite / Postgres | 対応するlease recordと同じ |
| heartbeat | active host signature | なし | gossip / WebSocket | memory latest only | grace判定後に破棄 |
| participant input | participant signature | なし | host session stream | host memory queue | 適用またはreject後に破棄。raw inputをlogへ出さない |
| physics snapshot | active host signature | なし | host session stream | client / host memory latest only | session終了または置換で破棄。ring bufferは#793 |
| guest prop expiry metadata | active host | なし | snapshot | host memory | wall-clock expiryまたはsession終了で破棄 |
| Community Node assignment mirror | owner / host署名済みrecord | なし | HTTPS | Postgres | lease expiry / close後にoperational retention規則で削除 |

秘密鍵、bearer token、participant raw input は永続化・診断出力しない。Dome id、epoch、lease digest、host id、session id、heartbeat timestamp、reject reason は診断可能とする。

## Community Node boundary

Community Node hosting は `dome_hosting` capability を明示的に有効化したNodeだけが提供する。既定は無効とする。Node signing key の公開鍵は manifest の `node_id` と一致しなければならない。authenticated / consent 済み owner だけが assign、renew、releaseできる。

Postgres は Dome ごとに active assignment を最大一件に制限する。Node restart は有効な lease と exact manifest bundle を再検証して新 session を開始するが、ephemeral simulation state は復元しない。

## Consequences

- ownerの明示操作なしにavailabilityを優先する自動failoverは行わない。
- transfer中またはsplit-brain検出時は一時的にDomeへ入れなくても、二重authorityより安全側を選ぶ。
- metaverseは実験機能のため、旧peer-authoritative wire contractとの互換decodeやmigrationは提供しない。
- snapshot retentionとresource budgetはADR-0041、transitionはADR-0042、access / block / presence / audioはADR-0043がこのcontractを利用する。
