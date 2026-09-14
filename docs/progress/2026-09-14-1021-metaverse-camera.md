# #1021 アバター追従カメラと入力所有

## 状態と範囲

- 状態: 実装済み、全体validation・実機確認の最終整理中。
- Scope revision: `2026-09-14-r2`。基準commit: `77d4aec1eaf98139e219e70e7d7737ca4383b38a`。
- リスクB。cameraとMetaverse専用input ownerに閉じ、Column共通guard、IPC、host physics、admission、shared transformは変更しない。独立監査の必須条件には該当しない。
- 承認済み要求: アバター追従、drag不要の回転、Tab／EnterでUIと固定を切替、3D表示中央のclickで開始。「アバター操作」と「視点の調整」を区別する。
- 所属先はトピック／チャンネル。日本語のDome管理文言も、対象のDome・一覧・描画データを具体的に表すよう修正した。
- #1024はサークルメニュー・カテゴリ構成、#1023はfullscreen黒画面、#1022はColumn幅／scrollを所有する。今回それらの全完了をClose条件に追加しない。

## 修正前の再現

Windowsの現行Tauriをローカルで起動し、所有Domeを管理から稼働・入室。日本語light、1283×871、標準3列。初期cameraでは下半身が見切れ、canvas左dragで視点が変わらず、wheelはColumn本文をscrollした。Ubuntu24でもRemote Desktopから所有Domeへ入室し、同じ固定cameraと見切れを確認した。

![Windows before](assets/2026-09-14-1021-camera/windows-before.png)

![Ubuntu24 before](assets/2026-09-14-1021-camera/linux-before.png)

修正前の`MetaverseRoomView.test.tsx`へ開始／reset導線の失敗testを追加し、1 failed / 6 passedを確認した。従来のScene mockだけでカメラ描画を保証しないよう、実Three cameraを使う投影・frame追従testと実Canvasのbrowser flowを追加した。

## 実装と固定inventory

| ID | 入口→helper→sink | guard／禁止副作用 | 条件・証拠 |
| --- | --- | --- | --- |
| INV-1 | mousemove／wheel／調整button／R→CameraModel、FollowCamera→Three camera | 自canvasの実固定＋enabled。camera-only、Ctrl／Meta wheelを奪わない | AC-1/2、CameraModel.test、FollowCamera.test |
| INV-2 | 中央click／再開／Tab／Enter／Escape／HUD callback→RoomView、useMetaverseSceneInput→Pointer LockとDOM focus | active／visible／suspend、IME・editable、UI markerで分離 | AC-1/3、RoomView.test、metaverse-camera.spec |
| INV-3 | blur／visibility／runtime／room切替／unmount／遅着→input hook→自canvas解除・request世代無効化 | 他Columnのlockを操作しない、自動再取得しない | INVAR-2、input hook.test |
| INV-4 | frame／spawn／handoff／VRMロード／resize→LocalAvatar group refとbounds→追従camera | 描画位置を読み取るだけ。assetロード時のprecise skinned boundsを利用 | AC-2、投影・frame test、実機画像 |
| INV-5 | 既存移動キー→LocalAvatar→session.handleLocalTransform→submitAuthoritativeInput→MetaverseRoomActions→DesktopApi | cameraからmove callbackへ接続しない。キー停止のみ補強し、physics／送信間隔を維持 | INVAR-1/2、Scene／session回帰、UI操作時のcallback spy |

caller確認はCodeGraphを先行し、未検出testをrgで補完した。変更対象sinkはcamera／Pointer Lock／focusだけ。Sceneの製品callerはRoomView、RoomViewはRoomPanelと直接story、RoomControlsはRoomViewのcallbackを使用する。ColumnSurfaceも読むColumnRuntimeContextは変更しない。sessionのprop interaction／transitionも使うhandleLocalTransformは変更しない。backend全sinkを監査済みとは扱わず、今回のcamera入力がDesktopApiへ達しない境界を検証した。inventory 5群、未分類0。

| 遷移 | 期待・検証 |
| --- | --- |
| TR-1 開始→回転→zoom→移動→reset | actual lock待ち、avatar追従と初期framing。model／frame／browser／実機 |
| TR-2 Tab／Enter→UI→閉じる | 自由pointer、UI focus、draft保持、IME無干渉。view／browser／両OS |
| TR-3 blur／非active／suspend→復帰 | 自owner解除、意図した再開だけ取得。hook／既存Column browser |
| TR-4 取得拒否／遅着 | unavailableと再試行、旧requestを解除。hook test |
| TR-5 resize／spawn／asset／handoff | bounds・aspect対応、現在の描画位置を追従。model／frame test |
| TR-6 キー押下→UI／別window | keys・jump要求を破棄し復帰時の固着を防ぐ。LocalAvatar cleanup、input lifecycle test |

## 実機証拠

Computer Useの`@oai/sky`でWindowsローカルとUbuntu24 Remote Desktopを操作した。Ubuntu24へのファイル反映・起動は`ssh local2`で行い、別ディレクトリ`/tmp/kukuri-1021`にfrontendを配置。既存backend executableを使用し、この変更ではbackend sourceを変更していない。

Windowsでは全身・足元、Pointer Lock取得、マウス移動による回転、wheel zoom、R reset、W移動、Tab解除／HUD、Escape復帰、Enterのchat focusを確認。Ubuntu24では変更前の見切れ、変更後の全身、中央clickで取得、Tab解除、Escapeのfocus復帰、Enterのchat focusを確認した。

![Windows after](assets/2026-09-14-1021-camera/windows-after.png)

![Ubuntu24 Pointer Lock](assets/2026-09-14-1021-camera/linux-locked.png)

![Ubuntu24 chat](assets/2026-09-14-1021-camera/linux-chat.png)

## Validation

- targeted camera／frame／input／view: 実行済み。数値比較の丸め差は近似比較へ修正し、該当testは成功。
- `metaverse-camera.spec.ts`: Chromium実Canvasで成功。中央click、実Pointer Lock、実wheel、外側scroll不変、Tab／Enter／Escape、draft保持、reset、再取得を確認。
- `cargo xtask check`: lint／typecheckとRust／Tauri checkを含め成功（後続差分は最終UI gateで再確認）。
- `cargo xtask test`: 実行中。
- `cargo xtask desktop-ui-check`: 初回はRust buildと並走中に無関係なAccountKeyPanelの5秒timeoutが発生したため中断。負荷を分離して再実行する。
- `cargo xtask oversized-files`: 初回は既存CSSが1000行を越えたため失敗。新規cameraスタイルを専用CSSへ配置して上限増加を回避し、再確認する。
- CI、Storybook／a11y、全体browser／visual、追加の表示条件は最終結果を追記する。実行中・未確認を成功扱いしない。

データ分類は[ADR 0050](../adr/0050-metaverse-camera-local-state.md)、UIの契約は[DESIGN](../../DESIGN.md)。カメラstateの保存・送信、カメラ衝突、新しい3D pickingは追加していない。
