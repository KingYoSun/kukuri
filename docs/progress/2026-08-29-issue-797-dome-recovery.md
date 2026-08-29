# Issue #797 Dome recovery 実装記録

## 実装した契約

- 5秒host heartbeat、15秒offline grace、30秒participant timeoutをcore定数と決定的resolverへ集約した。
- Owner hostは署名済みP2P heartbeat、Community Nodeは署名済みstatus heartbeatを返し、desktopは直前の検証済みheartbeatから通信失敗中のstateを導出する。
- `KeepAlive`をsession inputへ追加し、Community Nodeではaccess proofを再検証する。timeout時はhost runtimeがparticipant関連stateを原子的に除去する。
- 通常Connection解除は3秒のdurable `draining`を公開してからrevokeし、block等は即時terminal化する。draining中のreservationはcancelし、既存participantは維持する。
- Desktopはoffline countdown中にsceneを保持して入力をfenceし、期限後はready隣接Domeからentry候補へ退避する。Return Homeはtarget avatarをhost snapshotで確認するまでsource sceneを保持する。
- 境界はoffline、draining、blocked、closedを別stateとして描画し、HUD文言とReturn Home操作を追加した。

## 検証

- Core liveness/candidate ordering、metaverse-host keepalive/timeout、app-api draining/block、Desktop boundary/scene retention/keyboard操作を自動testで固定した。
- Owner/CNは同じheartbeat署名検証、session input、safe-spawn primitiveを利用する。

## Out of scope

- Community Node failover、自動lease再割当、prop/seat/装着assetの退避。
