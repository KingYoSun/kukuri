# #1025 接続方位と確認済みマップ

- Issue: [#1025](https://github.com/KingYoSun/kukuri/issues/1025)
- Scope revision: `2026-09-14-r1`、区分C。
- 基準commit: `ad5c1e9aa61e370570b3ca6a79edaa375b9d4521`。
- 2026-09-15に推奨案の実装、Issue更新、コミット、PR作成、CI後マージ、Windows／Ubuntu24のComputer Use確認を承認済み。
- PR: [#1035](https://github.com/KingYoSun/kukuri/pull/1035)。実装・検証中。独立監査、CI、マージ結果は確定後に追記する。

## 設計判断と変更

3D内の既存「接続」カテゴリに方位図、現在componentの相対マップ、選択方向の詳細を置く案を採用した。3Dの壁を直接選択する案はcamera／occlusion／pointer lockの追加依存があり、今回の目的に不要なため見送った。既存カテゴリ、入室外導線、4つの接続actionを使い、wire／physics／署名方式を変更しない。

マップのデータは取得済みContextだけに限定し、取得失敗と未取得を空きと扱わない。入室中はsessionとpaneのtopology取得を共有する。通行可否の取得失敗をclosedへ変換せず、draining／blocked等の既存理由を保つ。access拒否時は隣接assetを取得しない。

提案再試行のIDを保持し、write成功後のrefresh失敗を独立表示する。対象が変われば選択と古いresponseを破棄する。backendのlistにあった購読開始を除き、mutationもprivate Context検査後にだけ購読を開始する。許可されたreadによる端末内projection更新は維持する。

## 修正前の証拠

- 既存のPanel、TransitionModel、RoomPanel、RoomView: 4 files／49 tests成功。
- 追加した初回取得失敗／mutation後refresh失敗のtestは変更前に2件失敗、既存1件成功。
- backendのmap read購読開始禁止testは変更前に失敗。修正後の既存接続contractを含む7件成功。
- Windows Tauri/WebView、1283×871、ja、light、world version 7、既存owner Domeの[変更前フォーム](assets/2026-09-15-1025-connections/windows-before.png)。Issue起票時の旧画像とは別の現行HEAD観測。
- 取得controllerの追加で空の隣接array更新が初回イベントreadを中断する回帰を既存testが検出。空→空の不要更新を抑えて修正し、関連5 files／47 tests成功。
- browserで詳細paneの方位末尾が初期表示外になることを確認。paneの既存scrollを使う到達性を実操作で検証し、controllerを保持したまま狭幅で縦配置する。全方位の24px以上の幅・44px以上の高さ、keyboard、カテゴリ往復、禁止action 0回を確認。

## AC／INVARとinventory

| 条件・入口 | 実装 | test／証拠 |
| --- | --- | --- |
| AC-1、INV-2、TR-2 | Panel→MetaverseRoomActions→shell/actions/metaverse→runtimeApi→live_game→runtime/private_channels_game_api→app-api/dome_connections | Panelの提案／再試行／承諾／撤回／解除／非owner test、既存backend round tripとactor制限 |
| AC-2、INV-1、TR-1 | useDomeConnections、DomeConnectionModel／Map | 現在地・方位、別component除外、欠けた／他Context endpoint、未知slot、draining後の分裂 |
| AC-3、INV-1/3、TR-1/3 | 取得controller、neighbor hook、DomeTransitionModel | 初回失敗、更新失敗、access denied／error、draining理由、再試行 |
| AC-4、INV-1、TR-1/2 | HTML方位button、既存カテゴリpane | keyboard方向選択と候補保持、1283／390px browser、HUD／IME回帰 |
| INVAR-1/2、INV-1/2、TR-3 | Connection read replica、既存owner／Context guard、scope破棄 | map read購読0、未知channelの全5入口でdocs I/O・書込0／projectionなし、許可readで共有docs書込0、late response／混在Context |

入口からsink: UIの4actionとlistは `shell/actions/metaverse.ts` → `lib/api/commands/runtimeApi.ts` → Tauri `commands/live_game.rs` → runtime `private_channels_game_api.rs` → app-api。変更するreadは`dome_connection_read_replica` → private state検査 → docs open/query → projection upsert。writeはContext検査 → subscription →既存署名／persist_connection_envelope・proposal／selection／connection state → docs apply_doc_op、topology hint。

共有callerの範囲: Connection list／create／accept／withdraw／terminal／revoke、`service/dome_connection_support`のblock reconciliation、`dome_delete`、`dome_hosting`のtransition、runtimeとCLIのlive_metaverse、およびtests。UIの別入口は入室外Panel、入室中connections、session neighbor。変更前後でdomain action追加・削除は0。新規入口は端末内の方位選択とマップ表示だけ。

## 検証状況

実行ログはローカル `.codex/plans/issue-1025-*.log`。成功・未実行・途中失敗を区別し、最終結果を追記する。

- targeted frontend: 5 files／47 tests成功。追加neighbor testと全suiteは後続確認。
- browser: 接続2条件＋既存HUD2件、計4件成功。
- Storybook build: 成功。確認済み／partial／loading／失敗／候補なし／offline／draining／blocked／closed／非owner／狭幅を追加。
- check: 初回fmt、次回frontend typecheckの違反（ES target、Button variant、boundary union）を修正。typecheck単体は成功。全entrypointの再実行中。
- Rust本体: 942件成功、既存skip 4。harness 23件成功。未知channelの全入口I/O禁止testも成功。
- 接続／遷移scenario: `desktop_smoke_metaverse_dome_connections` 9 steps、`desktop_smoke_metaverse_dome_transition` 4 steps成功。in-processでpeer_count=0のため実P2P通信の証明ではない。
- 追加再現: draining recordより古いready表示を優先するtestが失敗。terminal理由の優先後にPanel全8件成功。host read中の離脱後にaccess previewを開始するtestが失敗。取消再確認後にneighbor全4件成功。
- oversized baseline: 初回は`dome_connections.rs`が1025→1029行となったが、監査修正でread専用helperを既存`service/dome_connection_support.rs`へ配置し、最終的に1025行を維持。生成時に既存のsession／noticeの減少も反映し、新規大型ファイル・許容上限増加は追加しない。
- Windowsでは実行中xtask.exeの再リンクが拒否されたため、同じソースのxtask.exeをローカル別名で実行して検証。製品・検証処理や必須項目は変更しない。
- ローカルdesktop-ui-check: lint／typecheck成功、frontend 1722件中1719成功・topics 3件失敗。失敗は変更前worktreeにも存在し、pipelineがtestで停止したため、後続Storybook／browser／visualを個別に確認した。
- 最終browser: 接続・HUD・camera・カラム幅の27ケース中26件が初回成功。ja/light/1 spanのheader位置assertion 1件は同じ条件の再実行で成功。assertionやtimeoutは変更していない。
- 最終Storybook build成功。Windows visual smoke 38件成功（pixel比較はskip）。修正後の接続／遷移scenarioを再実行し、9 steps／4 steps成功。
- Windows／Ubuntu24の初期実装に対するnative pointer・方位keyboard・基本描画を確認。[Windows map](assets/2026-09-15-1025-connections/windows-map-initial.png)は初期実装の証拠。監査修正後の両OSビルドは成功したが、新しいWindows executableのネットワーク許可画面が表示され、手動で閉じる対応待ち。最終native fullscreen／戻る確認・複数Dome実P2Pは未確認であり、browser／in-process成功で代替しない。
- CIは[PR checks](https://github.com/KingYoSun/kukuri/pull/1035/checks)で対象headを確認する。ローカル失敗をCI成功へ読み替えず、実行場所・対象commitを分ける。

## 監査・Close

初回head `629939c816103182b7e98bb12f12feeae587c36f` の独立監査はFAIL（F1/F2）。F1は加入済みprivateのwrite_state helperに購読・grant redemption・FriendOnly epoch rotationが含まれること、F2は旧generationのaccepted recordが現在slotの候補／解除を隠すこと。

F1はrestore相当のFriendOnly owner＋mutual解消済みparticipantで、変更前に共有書込5→10を再現した。readはmemory上の加入stateとread-onlyのrotation pending検査を使い、既存write helperの自動更新は明示mutation側へ残した。変更後は接続backend 9件成功（共有書込・secret登録・subscription・epoch不変を含む）。F2は旧generationのacceptedでmodel testが失敗し、current endpoint／Context一致で絞り、acceptedをslot占有と扱わない修正後に関連frontend 25件成功。方位提案への到達もcomponent testで固定する。

初回全体frontendは1721件中1717成功、topics／messagesの4件失敗。再実行と変更前commitの別worktreeでもtopicsのtimeout／後続Control Center不在を再現した。既存testのdeadlineやassertionは変更せず、CIの全suite結果と区別して記録する。

Storybookの11状態×light/dark、計22条件でaxeの対象WCAG tag違反0。色transitionが終了してから検査する。20条件のcolor-contrastはincompleteが残るため、自動判定の完了やAccessibility適合の証明とは扱わない。文字色を既存Buttonから継承し、theme tokenでmap線と現在地を描く。[明色の確認済みmap](assets/2026-09-15-1025-connections/story-confirmed-light.png)／[暗色](assets/2026-09-15-1025-connections/story-confirmed-dark.png)／[狭幅](assets/2026-09-15-1025-connections/story-narrow-light.png)を確認した。

固定PR headの独立監査を実装工程と分ける。IssueのAC／INVAR、上記入口と逆引きから再構築し、PASS、必須CI成功、merge tree整合またはdelta監査後にIssueをCloseする。未知の不具合の不存在や別Issueの新要件は条件へ追加しない。

`77db7d222626c3085b42ece38983cf5d0652caff` の[独立delta監査](2026-09-15-1025-independent-audit.md)はPASS。inventory 3件すべて適合、未分類0、不適合0、blocker0。許可ContextのIrohDocsSync内部同期は既存read機構として維持し、禁止するAppService購読Taskの新規起動・Connection／epoch等のdomain writeと区別する。後続差分はこの説明、comment、検証記録、画像のみで、domain動作は変更しない。

実機確認待ちを残したままComplete／マージ可能とは扱わない。必要な実機確認と最終CIが揃ってからPRをreadyにし、承認済みのマージとmerge tree確認を行う。
