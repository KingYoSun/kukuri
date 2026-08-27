# Issue #789: Spatial Context単位のDomeと引っ越し

## 実装した境界

- Metaverse wire contractを後方互換なしの`world_version = 3`へ更新し、topic / channelを区別する`SpatialContextV1`、owner-owned `DomePresetManifestV1`、Context-owned `DomeInstanceManifestV1`、generation付きlifecycle、move recordを追加した。
- Preset current pointerはowner author replica、Preset manifest / texture / modelはcontent-addressed blobへ保存する。customizationとpersistent prop初期配置はPresetだけが所有する。
- Instance owner slotは対象topic / private-channel replicaの`metaverse/dome-instances/{owner_pubkey}/state`へ保存する。Instance manifestはContext、owner、Preset ref、title / description、max peers、spawn、generation、status、session境界だけを所有する。
- instance idとowner slotを`Spatial Context canonical id + owner pubkey`から決定論的に導出し、同一Contextのactive / staging重複を拒否する。tombstone slotの再作成時はgenerationを増やす。
- 引っ越しを`preparing -> target_staged -> source_detached -> target_active -> source_tombstoned -> completed`の再開可能な処理として実装した。同じmove idのretryは保存済みphaseから続行し、異なる操作へのid再利用を拒否する。
- target公開前にPreset manifestと全asset blobを検証する。asset欠落ではsourceをactive / attachedのまま維持し、復旧後に同じmove idで再開できる。
- targetはsourceと同じPreset manifest hash / asset hashを参照し、chat / sessionは新Instanceへ持ち越さない。sourceにはgeneration-bound detach markerとreplacement instance idを残し、tombstoneを削除しない。
- Connection / proposal候補を同一Context、active、attachedのInstance間だけに制限するcore contractを追加した。保存形式とproposal処理はIssue #792がこのcontractを利用する。
- room eventへSpatial Context、instance generation、session idを署名対象として追加し、旧generation、Context不一致、detached / tombstoned Instanceのeventを拒否する。
- desktopではowner slotが存在するContextでcreate controlsを隠し、ownerだけにMove Dome formを表示する。target topicと任意channel idを指定して移動し、完了後にdiscoveryをrefreshする。
- harnessのdesktop restartを同一identity / docs / blobs / private-channel capabilityで再構成するようにし、public topicからprivate channelへのmove後もtargetが復元されるscenarioを追加した。

## Failure contract

- Presetまたはasset検証失敗: source active、relationship attached、target非表示。
- target staging後の失敗: stagingは一覧非表示、source active。同じoperationで再開。
- source detach後の失敗: sourceはtombstone化せずrelationshipだけ無効。同じoperationでtarget公開から再開。
- target公開後の失敗: target activeを維持し、retry / restartでsource tombstoneを完了。
- 完了後: sourceは一覧、session、publish / event一覧へ復活しない。

## 後続Issueとの境界

- Connection / proposal recordとtopology更新はIssue #792。
- Hosting Lease、active host、authoritative physicsはIssue #788。
- guest prop、layout commit、snapshotはIssue #793。
- entry、prefetch、access / block、offline recoveryはIssue #790 / #795 / #796 / #797。

## 検証

- `cargo xtask test`: workspace 616件成功（3件skip）、harness 20件成功、doc tests成功、frontend 883件成功。
- `cargo xtask desktop-ui-check`: lint / typecheck / frontend 883件 / Storybook build / browser E2E 44件 / visual regression 14件成功。
- `cargo xtask scenario desktop_smoke_metaverse_dome_move`: 14 steps成功。
- `cargo xtask e2e-smoke`: `desktop_smoke_post_persist` 6 steps成功。
- `cargo xtask check`、`cargo xtask oversized-files`、`cargo xtask ipc-types --check`: 成功（oversized filesは既存baseline 4件のみ）。
