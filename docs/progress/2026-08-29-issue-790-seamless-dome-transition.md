# Issue #790 Seamless Dome transition 実装記録

## 完了した範囲

- active Dome Connectionとcomponent座標を使い、current Domeと最大4方向のneighborを同一sceneへ配置した。未接続方向は従来の半球壁、接続済み方向だけがconnection zoneになる。
- neighborのHosting、capacity、assetを先読みし、`closed/loading/ready/denied/full/unhosted/error/stale`の境界状態を導入した。`ready`以外は中心線手前でavatarを停止する。
- transition ID、Connection、topology digest、source/target Instance generation、participant、target lease epoch/sessionを固定する15秒のreservation ticketを追加した。owner-device hostとCommunity Node hostは同じhost runtimeでprepare/commit/abortする。
- 中心線通過前はrollbackでき、通過時は宛先commitを先に確定する。宛先確定後はcurrent Domeを戻さず、送信元の退出だけを再試行して二重presenceを解消する。
- avatarのcomponent座標、環境光、ambient、fog、gravityをconnection zone内で補間する。propは移送せず、host physicsで半球内に拘束する。
- Rust unit test、React hook/model test、異なるowner-device host間のdeterministic harness scenarioを追加した。

## 後続Issueとの境界

signed blockによる`denied`判定は#795、offline時のpresence/editor整合は#796、entry/evacuation UXは#797が本admission hookとboundary stateを利用して実装する。
