# Issue #796 Spatial Context entry / safe spawn

- Spatial Contextの入場候補をown hosted Dome、confirmed last visit、channel設定entry Dome、安定順一覧の順で解決し、失敗候補を自動fallbackする。
- Private channel current policyへowner-onlyのentry Dome参照を追加し、設定、解除、epoch rotation、同一Context検証を実装した。Connection topologyは変更しない。
- Owner hostは共通access evaluator、Community Node hostはparticipant/context/target ownerへ束縛した短命access proofを`Join`直前に検証する。
- Desktopはthree-vrmの明示colliderまたはVRM bounding box由来capsuleを入場requestへ渡し、読込不能時はhostの既定capsuleへfallbackする。Authoritative hostがmanifest default spawnから決定的な安全候補を選び、avatar/prop colliderと余白が重なる候補を避ける。全候補占有時は原子的に拒否する。
- Desktopはselectionとadmissionを分離し、host snapshotでlocal avatarを確認するまでscene、presence、音声を開始しない。Confirmed admissionとtransition commitだけをlast visitへ記録する。
- `desktop_smoke_metaverse_dome_entry`でdefault spawnとDome境界に収まらないcolliderの決定的な退避を確認する。Prop/avatar重複と全候補占有はhost unit testで固定する。

Validationは`cargo xtask check`、`cargo xtask test`、`cargo xtask cn-check`、`cargo xtask cn-test`、`cargo xtask scenario desktop_smoke_metaverse_dome_entry`、`cargo xtask desktop-ui-check`、`cargo xtask tauri-check`、`cargo xtask ipc-types --check`を基準とする。
