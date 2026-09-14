# #1020 所有Domeの管理・削除

- Scope revision: 2026-09-14-r1
- リスク: C
- 基準commit: 64a1b4729a197e4131703bf724e574f657716e27
- 状態: 実装・Windows/Ubuntu実機・独立監査PASS。最終CIとmerge後判定はPR #1026 / Issue #1020を正本に追跡する。
- 承認範囲: 実装、Issue作業、コミット、PR、CI成功後のマージ。区分Cの独立監査PASSも維持する。

## 修正前の再現

`MetaverseRoomDiscovery.test.tsx`に停止ownerの管理操作を追加したところ、`Manage Dome`が存在せず失敗した。既存の参加操作は無効で、管理へ到達できなかった。2026-09-14のWindows実機報告のU01/U06/U11と一致するため、非再現Closeの条件には該当しない。

Windows実機では、他Domeのmanifest未取得が一覧/Connection取得全体を失敗させ、所有Domeがあっても一覧が空に見える状態も確認した。`missing_other_instance_keeps_owner_management_topology_available`を追加して失敗を確認した後、未取得だけを候補から外し、不正署名・binding違反は既存どおり拒否するよう修正した。

削除→同じslotで新generation作成→hosting開始のtestは、旧leaseのInstance不一致で失敗した。署名とContext/owner/idを検証した過去generationのleaseを履歴として扱い、authority候補から外す。epochは履歴を含めて単調増加する。

## 実装と受入条件

| 条件 | 実装 | 証跡 |
| --- | --- | --- |
| AC-1 | Discoveryの管理ボタンと独立した管理対象。停止中・再起動後もJoin不要 | Discovery/component test、Windows実機で停止カード→管理→開始→admission→sceneを確認 |
| AC-2 | `delete_dome`、署名付き削除journal、generation束縛、未完了操作の照会・retry、tombstone後の再作成 | app-api削除tests（停止/稼働、restart、旧retry、各docs書込み失敗）、runtime CN release contract、harness新scenario |
| AC-3 | 作成後は管理主操作。開始済みなら入室だけをretry。成功後にsceneへfocus/scroll | Panel/Management tests、Playwright flow、Windows実機 |
| AC-4 | 稼働/入室/外部ピア状態を分離、所有数制限の説明、一覧取得errorの伝達 | Discovery/Controls tests、ja/en/zh-CN、Story/browser確認 |
| INVAR-1 | owner/Context/expected generationをwrite前に検証。管理表示では署名操作なし。admission確認前はsceneなし | negative docs不変test、Context購読が増えないtest、既存admission/recovery/hosting tests |
| INVAR-2 | 同じIDの新世代に旧retryを適用しない。他ownerのInstanceを変更しない。retentionとremote-copy限界を明示 | 旧retry前後のcanonical Instance完全一致、署名済み旧CN closeのretry、既存move/retention回帰 |

テスト名の列挙は実行成功の代替にしない。下記の結果欄が未確認範囲を示す。

## Inventoryと状態遷移

IssueのINV-1（作成/一覧/entry）、INV-2（管理/hosting/delete）、INV-3（refresh/presence）を維持。追加は管理選択、削除確認、`delete_dome`、`list_pending_dome_deletions`と既存削除操作のretry。CLIのdeleteはDestructiveとして登録。

- 入口: Discovery/Panel/Management/PendingDeletions、初回auto entry/Return Home、Tauri command、CLI登録、startup後一覧、poll/refresh、delete retry。
- 保存sink: `persist_dome_instance_manifest`（create/update/import/move/delete）、`persist_game_room_manifest`、game cache、署名付きhosting/deletion record、Connection lifecycle。
- network sink: hosting assign/activate/release、SessionChanged hint、Join/input/presence/audio。CN releaseは既存auth/consent gateを通る。
- 同一AppService handles内のDome mutationは共有排他で直列化。layout commitからのupdate/start/prepareは内部関数を呼び、二重取得しない。
- TR-1: create→管理→明示start→Join確認→scene。start失敗とJoin失敗を分離。
- TR-2: 保存済み停止Dome→restart→管理、停止と削除の区別、削除→新generation作成。
- TR-3: 非owner/Context/generation不一致、cancel、遅着、途中書込み失敗、CN release失敗、旧operation retryを有限に検証する。

CodeGraphのcaller一覧にはmacro/文字列IPC/一部method呼出しの欠落があったため、対象symbolの`rg`逆引きとTauri/CLI登録を補完した。独立監査ではこの一覧を再生成する。

## 設計判断

- 削除journalは対象Contextの`metaverse/dome-deletions/<Context+Instance+generationのhash>/state`。owner署名付きで、request、Instanceメタデータの削除用投影、作成時刻、local完了、旧CN target/署名closeを保持する。private Contextの情報をauthor public replicaへ出さない。
- UIのoperation IDはInstance/generationから安定して作る。CLIもrequestを保持して同じoperation IDをretryする。異なるoperation IDで同世代journalを上書きしない。
- local削除完了とCN cleanup pendingを区別。CNに到達できなくてもlocal close/tombstoneを巻き戻さない。旧lease epochに束縛したreleaseだけをretryする。
- Instanceの公開停止はtombstone、topology失効、hosting closeで行う。PresetはContextから独立したowner資産（ADR 0036）であり、Dome Instance削除をPreset全体の破壊へ拡大しない。Current/rollback等の有効な参照と共有assetを保持する。blobの即時消去や他peerの取得済みcopy消去は保証しない。ADR 0040の参照0・24時間graceを変更しない。
- UI専用の管理対象・確認状態をnetwork sessionから分離。managed targetはaccount/Context/Instance/generationで識別し、旧async結果を現在の対象へ適用しない。

## 検証経過（当時の未確認事項を含む履歴）

Ubuntuでの既存Dome確認から、Preset未取得でownerまで一覧から消え、deleteもPreset/派生game blobを要求する残存経路を確認した。`owner_can_manage_and_delete_without_preset_bytes`と`owner_can_delete_without_derived_game_manifest`をred→greenにし、署名済みInstanceだけから管理メタデータ・削除投影を作るよう分離した。後者は空のprojection storeからの管理復元も確認する。入室準備とhostingの状態理由を混同せず、未取得の入室データはnullableなPreset情報とmetadata-only viewで表す。

古い管理画面のstart/stop/delegate要求が新generationを変更するケースもred→greenにした。GUIはgenerationを送信し、backend guardが拒否した場合はdocs全体と新runtimeが不変であることを確認する。対象を切り替えた際は高度なHosting panelもremountし、同意Dialogの委譲先Domeを持ち越さない。

同じroom IDでgenerationが変わると旧admissionが新Domeへ引き継がれるUI経路を`requires a new authoritative admission when the same room id has a new generation`で再現した。admissionをaccount/Context/ID/generationへ束縛し、snapshotもID/generationとlease epochで拒否・再基準化する。管理中に新generationを観測した場合は旧targetの高度な操作を閉じ、選び直す案内を表示する。focus移動もgenerationへ束縛する。

独立監査の初回commit `e5ced6b0` はCN releaseと新generation activationの競合B-1でFAIL。実際のDB close後・pin/runtime cleanup前にbarrierを置くtestで、新runtimeが消えることを再現した。assign/activate/releaseのprocess内排他とDB advisory transaction lockで修正する。旧FAIL記録は独立監査記録へ残し、修正commitをdelta監査する。

CI初回の失敗は新scenarioの台帳件数・lane登録、大型ファイルbaseline、browser mockでの停止ボタンと再作成時のaccordion状態だった。scenarioを21件へ更新してnightlyへ配線し、browserは失敗した同じflowで再試行成功。大型ファイル6件の増加は今回のlifecycle guard/内部呼出し・公開facade・一覧のcanonical照合・空topology・IPC・scenario追加に限定して承認対象差分へ記録する。削除本体は独立ファイルへ配置済みで、既存の大きなmoduleをこの修正で全面分割せず、変更6pathだけbaselineを更新する。

- 成功: frontend targeted 18 files / 127 tests。app-api Dome関連22 tests。IPC型生成、frontend typecheck/lint（途中修正後は最終headで再確認する）。
- Windows: 同じ保存データで停止カード→管理→明示稼働→admission後のscene表示・表示位置移動を確認。削除/再作成・最終headの再確認は継続中。
- 全frontend suite、全必須gate、CN失敗contract、Playwright flow、harness、Linux実機: 実行中または未実行。
- 中断: `cargo test -p kukuri-app-api dome_ --lib`のlayout commitが二重lockで停止したことをcall pathで確認し、該当test processだけ停止。内部関数へ修正後、Dome22 testsは完走した。
- 最終独立監査: 未実施。PASSと必須CI、merge tree整合を確認するまでCloseしない。

### 追加監査B-3の修正

`0c4ff02b`への独立監査で、旧GUIのJoin/Prop要求が現在generationで再署名されるExisting-gapを確認した。要求に任意の`expected_generation`を追加し、GUIの共通input・Prop入口から必ず渡す。runtimeはlease照合後・署名/HTTP前、app-apiはcanonical Instance照合後・runtime mutation前に拒否する。runtimeからlocal app-apiへも取得済みleaseの世代を固定して渡す。省略する既存CLI/Rust callerは現対象を明示的に操作する互換方針を維持する。

- `stale_session_inputs_cannot_mutate_recreated_dome`: G1のJoin/Propを保持→削除→G2作成/稼働→送信。修正前は成功してしまいFAIL、修正後は拒否してparticipantsとphysics bodiesが不変。Dome関連26 tests PASS。
- `stale_session_input_never_reaches_recreated_community_node_host`: 同sequenceでCNへのHTTP hitが修正前2、修正後0。PASS。
- CIで一覧読込み完了前にcreateボタンを同期取得していたshell testを、実際の読込み完了待機へ同期。Linux専用CLI registry件数も追加2command分を139→141へ同期。
- CI視覚差分のprofile-connectionsは変更対象外の読込み中表示の一時capture。expected/actualを確認し、baselineを変更せず最終headで再実行する。

## 最終確認の対応

製品コードの検証対象は `ade5a5ea`。以後の差分は本作業記録・独立監査記録・UI review・証跡のみ。

| 対象 | 結果・根拠 |
| --- | --- |
| AC-1 / TR-2 | 両OSのprocess終了・再起動後にowner管理→再開→authoritative入室を確認。Ubuntuの既存owner一覧欠落も解消 |
| AC-2 / TR-2/3 | 両OSで削除取消→対象保持、確認削除→空一覧/作成枠復帰→同Context再作成→入室成功。docs各writeの部分失敗/restart/retryはapp-api test |
| AC-3 / TR-1 | 作成直後に主操作提示、成功時にsceneへfocus/scroll。start失敗とadmission失敗のretry分離はcomponent test |
| AC-4 | 両OSで「入室済み・この端末で稼働中・外部ピア未接続」を同時表示。所有枠説明/停止/開始中/復帰状態はcomponent fixture |
| INVAR-1/2 | 独立監査8群適合・未分類0・blocker0、`ade5a5ea` PASS。stale docs/runtime不変、CN入力HTTP0、旧release/status競合、owner/Context境界、safe spawn回帰 |
| Rust | 最終targeted Dome26 tests / CN input禁止HTTP contract PASS。CN check/testは実DB競合3tests込みでPASS。全non-CN regressionはPRのlinux-rust-tests、compile/staticはlinux-rust-static/windows-fast |
| Frontend | 最終delta4 files/78 tests、typecheck/lint PASS。全VitestはPR linux-desktop-ui。Storybook/browser/visualはlinux-desktop-browser。旧visual失敗はprofile読込み中captureで、baseline不変 |
| Scenario | `desktop_smoke_metaverse_dome_delete`の7 steps（create/delete/restart/recreate/entry）PASS。e2e-smokeとCN connectivityはPR linux-smoke/linux-community-node |
| 実機 | [UI review](../ui-reviews/2026-09-14-1020-dome-management.md)にviewport/OS/WebView/locale、操作と画像、未確認事項を記録 |

local全suiteの途中失敗は消していない。旧CLI台帳/件数とreadiness待機は修正後の対象testおよび最終CIへ対応。負荷中に発生した無関係なshell timeoutは単体rerunとCIで確認する。単体成功を全suite成功へ読み替えない。PR最終headの必須CI成功、監査対象とmerge tree一致、Issue本文更新を満たしてからCompleteとする。

最終CIのRust testで963成功・1失敗となり、追加したCN delete/input testの共有資源lock分類が5→7へ未更新と判明した。取得箇所を保ったまま分類表と総数134→136を同期し、同じcontractで検証する。製品コード・lock取得・既存assertは変更しない。
