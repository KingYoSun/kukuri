# Issue #1025 独立監査

## 初回監査の判定

- 判定: **FAIL**（対象commitにblocker 2件）。後続修正はこの判定へ混ぜず、固定commitでdeltaを追記する。
- 対象commit: `629939c816103182b7e98bb12f12feeae587c36f`
- 基準commit: `ad5c1e9aa61e370570b3ca6a79edaa375b9d4521`
- Scope revision: `2026-09-14-r1`
- リスク区分: C
- 対象: [Issue #1025](https://github.com/KingYoSun/kukuri/issues/1025)、[PR #1035](https://github.com/KingYoSun/kukuri/pull/1035)
- 担当: 実装と別コンテキストの監査agent。実装時の結論／progressの自己評価を採用せず、Issue本文、ADR、対象source、tests、実行出力から再構築した。
- 固定inventory: **合計3 / 適合0 / 不適合3 / 未分類0**。各行はIssueのINV-1〜3であり、個別操作数ではない。INV-3にもprivate previewからF1が伝播する。部分的に適合する境界は下表へ別記する。
- コード変更: なし。本監査記録だけを作成した。
- 必須CI、Windows実機の最終証拠、merge tree一致: 親担当が確認する未完了の別ゲート。成功とみなさない。

## 固定inventoryと入口からsinkへの照合

| ID | 全member／入口と経路 | sink／guardの確認 | 判定 |
| --- | --- | --- | --- |
| INV-1 | 入室中connectionsカテゴリ、入室外接続管理、初回mount、5秒timer、明示refresh → `DomeConnectionPanel` / `useDomeConnections` → `MetaverseRoomActions.listConnections` → shell action → runtimeApi → Tauri → DesktopRuntime → AppService list。方位button／候補draft → model/map | Context/author/Instance/generation scope、epochとflightによる遅着排除・重複read抑止、失敗snapshotのscope内保持は確認。private readが購読・epoch更新へ進むF1と旧世代slotのF2が残る | 不適合 |
| INV-2 | create／accept／withdraw／revoke、結果不明retry → `ConnectionControls.run` → 同名MetaverseRoomActions → shell action → runtimeApi → Tauri → DesktopRuntime → AppService。CLIにも同じ5 commandが登録 | 明示クリック、owner/current endpoint、同Context候補、pending lockを確認。backendの署名・payload・Context/generation・slot/queue・cycle拒否は維持。旧世代のacceptedによって提案／管理へ到達できないF2 | 不適合 |
| INV-3 | 入室session、topology更新、host更新、接続失効 → `useDomeTransitionNeighbors` → `resolveActiveDomeNeighbors` → getHosting／previewTransitionAccess／許可後blob preview → session boundary / scene。preview AppServiceもlistを呼ぶ | 同Contextと両endpoint generation、cancel後の後続access/asset停止、denied/error区別、draining/blocked/closed優先は確認。private preview→listにF1が伝播 | 不適合 |

### API登録と中継

- UI共通経路: `apps/desktop/src/components/extended/MetaverseRoomPanel.tsx:280` が入室中snapshot/boundariesを渡し、`:346` と `:359` に入室外／入室中を配置する。
- action定義: `.../metaverse/MetaverseRoomActions.ts:113`、shell中継 `apps/desktop/src/shell/actions/metaverse.ts:68`。
- IPC payload: `apps/desktop/src/lib/api/commands/runtimeApi.ts:728`、Tauri handler `apps/desktop/src-tauri/src/commands/live_game.rs:276`、登録 `apps/desktop/src-tauri/src/lib.rs:444`。
- runtime中継: `crates/desktop-runtime/src/runtime/private_channels_game_api.rs:895`。入力Context/ID/directionをそのままAppServiceへ渡し、別のmutation implementationは追加しない。
- CLI登録: `crates/kukuri-cli/src/commands/live_metaverse.rs:150` / `:246`。list / create / accept / withdraw / revokeすべて同じruntimeへ接続する。CLI自体のUX再設計はNon-goalだがshared helperの影響先として確認した。

## sensitive sinkの逆引き

CodeGraph explore/node/callersを先に使用した。Rustの同名method跨ぎやTauri macro登録がcallers出力に完全には含まれなかったため、当該識別子と対象sourceに限った`rg -n`で補完した。indexの作成・更新は行っていない。

| sink / shared helper | 逆引きで確認した全production caller | 確認結果 |
| --- | --- | --- |
| `list_dome_connection_topology` | DesktopRuntime/Tauri/CLIのlist、runtime prepare、AppService `prepare_dome_transition` / `preview_dome_transition_access`、`delete_dome`（前後2回）、`reconcile_blocked_dome_connections` | source Contextに限定するprojectionとread。private副作用F1がすべてへ伝播し得る |
| `dome_connection_context_replica` | create、accept、withdraw、proposal terminal、connection terminal | unknown channelはprivate state検査でsubscription前に拒否。既加入private helperはF1の副作用を持つ。明示mutationの正当な更新契約まで禁止してはならない |
| `dome_connection_read_replica` | list、context replica | publicではtopic subscriptionを開始しない。privateではwrite_stateが実行されるF1 |
| `upsert_dome_connection_projection` | AppService listのみ（MemoryStore / SqliteStoreのtrait実装） | 許可readからの端末内派生cache。共有署名recordのwriteと区別。unknown privateでは到達しない |
| `persist_connection_envelope` → docs `apply_doc_op` | create、accept、proposal terminal、`persist_connection_lifecycle` | 署名record生成・検証後。通常閲覧から直接呼ばれない |
| `persist_dome_proposal_state` → docs `apply_doc_op` | create、proposal terminal | ID再利用時のowner/payload一致、terminal actorの署名を確認 |
| `persist_dome_selection` → docs `apply_doc_op` | acceptのみ | receiver owner、現endpoint、双方署名、prospective topology検証後 |
| `persist_dome_connection_state` → docs `apply_doc_op` | accept、lifecycle | 上記guardとlifecycle actor検証後 |
| `persist_connection_lifecycle` | `terminate_dome_connection_with_reason` のdraining／terminalの2箇所 | endpoint ownerのみ。通常3秒draining、deadline後再読取、既にrevokedなら早期return。block/deleteは即時terminal |
| `publish_dome_topology_hint` | create、accept、proposal terminal、connection terminal（drainingと完了） | Contextのhint topicへ通知。retryしたrevoked操作のhint再送は既存契約。新規合意writeや無権限mutationと混同しない |
| private subscription / rotation / handoff | list→read_replica→write_state→owner_action→private subscription / auto rotate / handoff redemption | F1。接続専用docs sinkの逆引きだけでは見つからない副作用 |

Background／aggregate経路として、block reconciliationは既存subscriptionのTopicとjoined private Contextを列挙し、各Contextのread成功後にlocal ownerとtarget ownerが一致するproposal／Connectionだけをterminal化する。deleteは要求Context/Instance/generationに合うConnectionを対象とする。新しいglobal apply、scheduler、永続schema、protocolは追加されていない。

## Blocker

### F1 — privateのマップ閲覧が購読・epoch更新・共有writeを起こす

- 分類: **Existing-gap**（固定INVAR-1の「マップ閲覧／方向選択だけではnetwork writeを行わない」に対応）。優先度: P1。
- 到達入口: INV-1の既加入private Contextでの初回表示／refresh／timer。INV-3のpreview→listからも到達する。
- 根拠: `crates/app-api/src/dome_connections.rs:15` → `:531 private_channel_write_state` → `service/private_channels_support.rs:398` → `:366 private_channel_state_for_owner_action`。
- このhelperは名前だけの権限検査ではない。
  1. `:376` / `:384` のensure_private_channel_subscription → `:584 spawn_private_channel_subscription` → `:608` のsecret登録、`:611` のsubscription task起動。restore後など、joined stateはあるがsubscriptionがない場合に開始する。
  2. FriendOnly ownerにactive participantが残りrelationship.mutual=falseなら `private_channel_diagnostics:198–225` がrotation_requiredを返す。`maybe_auto_rotate_private_channel_for_owner:355–360` → `private_channels.rs:645 rotate_private_channel` → freeze、next epoch seed、grant配布、joined state確定という署名・共有・永続更新に進む。
  3. 利用可能なepoch handoff grantを持つmemberでは `maybe_redeem_epoch_handoff_grants_for_channel:138–169` がparticipant docを書き、joined registryを更新する。
- 利用者影響: マップを読む操作から、購読とprivate channelの共有状態／epoch／participantが変更される。取得先の新epochへの移行も発生し得る。秘密漏洩が実測されたとの主張ではない。
- `unknown_connection_context_rejects_all_actions_before_io_and_projection` と `connection_map_reads_do_not_start_subscriptions_even_for_unknown_channels` は未加入channelとpublicの分岐を検証しており、既加入private／FriendOnly rotation／handoffは通らない。19件のfrontend成功もこのbackend境界の証拠にならない。
- 再現条件: 既加入private state＋未起動subscription、またはFriendOnly owner＋stale参加者、または未redeem handoff grantを固定し、list前後のsubscription／docs writes／epoch／registryを比較する。監査では到達sourceを確定し、親へ再現test追加を依頼した。監査側でRust testの追記はしていない。
- 最小修正方向: 閲覧には副作用のないjoined state確認とreadの検査を使い、既存private write helperの自動更新は明示mutation側で維持する。`dome_deletion_replica`もwrite_stateを呼ぶため代替にはならない。

### F2 — 旧世代accepted recordで方向slotの操作が消える

- 分類: **Regression**、AC-1 / AC-3 / TR-3。優先度: P2。
- 到達入口: 同じInstance IDの再作成後、旧Connectionのterminal更新がまだ届かない部分同期状態で接続カテゴリを開く。
- backendの根拠: Instance IDはContext/ownerで決定し、再作成時には同じIDでgenerationを増やす（`crates/app-api/src/game.rs:305–321`）。旧generationのrecordはcore topologyから拒否される（`core/src/dome_connections.rs:562`）。listはactive_ids外のactive recordをacceptedへ降格して返す（`app-api/src/dome_connections.rs:57–60`）。
- UIの根拠: `DomeConnectionModel.ts:19–24` はcurrent instance_idだけで旧generationのrecordを選ぶ。`DomeConnectionPanel.tsx:51` はacceptedをoccupiedとし、`:112` のrevokeはcurrent endpointが必要、`:115` の候補フォームは!occupiedが必要。したがって旧世代ではどちらも表示されない。
- 監査側の実行再現: ViteのssrLoadModuleで実sourceのconnectionSlot / endpointIsCurrentを読み、current generation=2、旧endpoint generation=1、record status=accepted、current component孤立を入力した。結果は `{"status":"accepted","occupied":true,"current":false}`。これは無権限writeではなく、空いている現在世代slotから管理操作へ到達できない利用者影響である。
- before/after: 基準commitのPanelのactiveConnectionはactive/drainingだけを占有対象にした。対象commitがacceptedの占有を追加したことで発生する。
- 最小修正方向: current endpoint generation/ownerを表示・占有の照合に含め、topologyで有効化されていないacceptedを現在slotの占有へ変換しない。親へ回帰test付き修正を依頼した。

## AC / INVAR evidence

| 条件 | source / test / 実行証拠 | 初回の結論 |
| --- | --- | --- |
| AC-1 | `DomeConnectionPanel.test.tsx`: east payload、correct endpoint accept/withdraw、explicit revoke、non-owner拒否。`MetaverseRoomPanel.tsx`で3Dカテゴリへ実接続 | 通常経路は到達する。旧世代F2で不適合 |
| AC-2 | model/mapは現在component、current相対x/z、north、edgeの方向を表示。`DomeConnectionModel.test.ts`で別component・foreign Context名除外、missing current unknown、draining geometryとsplit。backend成功をglobal completeとみなさないscope note | topology／catalog境界の表示は適合。F1によりread無副作用を総合PASSとできない |
| AC-3 | hookのloading/ready/refreshing/error、last successful snapshotとstale note、refresh button。Panelのproposal/lifecycle reasonとdeadline、neighborのhosting/error/access結果を別扱い。drainingが古いreadyを上書きするtest | 通常状態は適合。旧世代F2とprivate read F1で総合不適合 |
| AC-4 | button/selectの標準keyboard、方位Arrow/Home/End、IMEを無視。`MetaverseCategories.tsx:72`は各paneをmountedのままhiddenで切替。既存RoomViewのEscapeでstage focusへ復帰。browser testで1283/390pxのpointer、keyboard、メニュー・カテゴリ往復とwrite0 | componentとbrowser証拠は適合。nativeの最終同条件検証は未確認 |
| INVAR-1 | app-apiでowner、ID payload、current endpoints、32/4/32 queue cap、frequency、双方署名、prospective topology検証をwrite前に実施。coreのslot/cycle/component merge/coordinate拒否testsとログ、block tests、draining tests | 署名／合意／drainingは維持。read由来private writeのF1で不適合 |
| INVAR-2 | 同Context候補だけ、mapに未知名を補わない、隣接は同Context＋両generation、access allowed後だけasset取得。unknown channelでdocs I/O/write0、topic subscriptionなし、projectionなし。双方合意APIは同じ | 未加入Context拒否・追加探索なしは確認。既加入privateのread境界はF1のため適合と断定しない |

## 状態遷移と拒否I/O

- TR-1 fresh / missing: roomなしではreadなし。初回失敗をopen/no candidatesへ変換しない。unknown Context responseはerror。同期snapshotはscope不一致のrenderで直ちに隠す。
- TR-1 timer/refresh: 同scope flightを共有し、失敗は前回snapshotとerrorを保持。5秒timerのcleanupとepoch増分でaccount/Context/Instance/generation変更後の遅着を不採用にする。
- TR-2 create/retry: IDをdirection/target/target generationに束縛し、結果不明retryは同ID/payload。候補消失で操作不可。成功後refresh失敗はaction成功と区別して表示し、read failureで再送しない。
- TR-2 accept/withdraw/revoke: pending lockとownerガード、endpointごとの操作表示。サーバではownerと双方署名がwriteを支配する。unknown private拒否はdocs I/O、docs write、topic subscription、projectionを0/不変で検証。
- TR-3 blocked: accept時のblock再検査はproposalの正当なowners_blocked terminal記録を許可し、Connectionを作成しない。これを「拒否時は全DB無変更」と誤って禁止しない。unblockで自動復元しない。
- TR-3 draining: durable drainingと3秒deadline、geometry維持、deadline後terminal、reservation drainを確認。UIはtopologyのdraining/blocked/closedを古いready passageより優先する。
- TR-3 cancel/late read: neighborのhosting後／access後にcancelを確認し、退出後の後続previewとasset取得を止める。送信済み明示mutationをrollback済みとは表示しない。
- TR-3 restart/cache: scenarioは保存された2本のConnectionをrestart後に確認し、revoke後にcomponentが分裂する。privateのrestore後subscription開始はF1で残る。

## 実行したvalidationと参照した実行証拠

監査側で実行:

1. `git rev-parse HEAD`、基準との差分とstatus確認。開始時HEADは指定629939cと一致。開始時の既存未追跡調査成果物は変更しなかった。
2. CodeGraph explore / node / callers、登録・同名method跨ぎだけ限定rg補完。
3. `npx pnpm@10.16.1 exec vitest run src/components/extended/metaverse/DomeConnectionModel.test.ts src/components/extended/metaverse/DomeConnectionPanel.test.tsx src/components/extended/metaverse/useDomeConnections.test.tsx src/components/extended/metaverse/useDomeTransitionNeighbors.test.tsx` → **4 files / 19 tests PASS**（5.45秒）。
4. 実source SSRによる旧世代slot再現 → occupied=true/current=falseを確認（exit 0）。source/testファイルは追記しなかった。

自己評価ではなく、sourceと対応させて読んだ既存実行出力:

- `.codex/plans/issue-1025-test.log`: coreのdome_connections 9 tests、app-apiの接続8 tests、harnessの接続scenarioにPASS出力。全suiteは進行中／別test timeoutがあり、全体成功は主張しない。
- `.codex/plans/issue-1025-rust-targeted.log`: 当時の接続7 tests PASS。後で追加されたunknown-context projection testの実行は上のfull logにあり、7 testsで8件全体を代表させない。
- `.codex/plans/issue-1025-scenario-connections.log`: 9 steps PASS。YAMLのcreate→connect→restart→revoke→splitと照合。
- `.codex/plans/issue-1025-scenario-transition.log`: 4 steps PASS。複数実機のP2P通行とは区別。
- `.codex/plans/issue-1025-browser-3.log`: connections 1283/390pxと既存HUDの計4 tests PASS。test sourceのfocus／direction保持／write0と照合。

## Non-blocker／未確認

- 許可readのlocal projection更新、署名したownerによるblock terminal、draining deadline後のterminal、retry hintは既存の正当な副作用でありblockerにしない。
- 未知Contextの全世界探索、global map、権限拡張、新しいphysics／protocol、一般的な悪意clientへの追加対策はNon-goal。
- テスト名「account or generation changes」は現testではauthor変更を実行している。generationもscope構成とkeyで処理されるsource根拠はあるが、名前だけでgeneration test実行済みとは扱わない。
- Windows実機の最終before/after、fullscreen、200%、High Contrast、screen reader、複数Domeの実P2P通行、必須CI、merge tree確認は本監査では未確認。親が別に追記する。未確認を成功に置き換えない。
- 対象commit後のCSS整理・その他の変更は本初回判定の対象外。修正後のcommitを固定し、F1/F2とその影響先をdelta監査する。

## Delta監査 — 2026-09-15 / 77db7d2

- 対象commit: `77db7d222626c3085b42ece38983cf5d0652caff`
- delta基準: 初回FAILの `629939c816103182b7e98bb12f12feeae587c36f`
- Scope revision: `2026-09-14-r1`（変更なし）
- リスク区分: C
- 判定: **PASS（コード・contract・固定surfaceの独立監査）**
- 固定inventory: **合計3 / 適合3 / 不適合0 / 未分類0**
- Blocker: **0件**。初回F1/F2は解消。初回FAILの本文・当時の根拠は履歴として保持する。
- 新規入口／sink: 0。Connection read helperの配置と副作用境界を変更した。公開command・payload・schemaは変更していない。
- 適用範囲: F1/F2、その全callerへの影響、同commitのCSS差分。初回で適合した未変更の署名／topology／draining／keyboard等の監査結果は継続利用した。
- 必須CI、desktop-ui-check、Windows実機最終証拠、merge commit整合: 親担当の別ゲートとして未完了。**このPASSはmerge/Close条件がすべて完了したという判定ではない。**
- コード変更: なし。監査記録への追記のみ。

### F1の解消と影響先

`AppService::list_dome_connection_topology`は引き続きread専用helperを呼び、そのhelperを既存`service/dome_connection_support.rs:87`へ配置した。

1. publicは既存topic replica IDを解決する。
2. privateは`joined_private_channel_state`でmemory上の加入状態を取得し、存在しなければreplicaアクセス前に拒否する。
3. `private_channel_rotation_is_pending`は同じ既知epochのpolicy/grantを読み、decryptしてpendingを判定するだけである。pendingならlistをerrorにし、grant redemption・次epochのsecret登録・participant書込へ進まない。
4. read側から`private_channel_write_state`、`maybe_auto_rotate_private_channel_for_owner`、`maybe_redeem_epoch_handoff_grants_for_channel`、`ensure_private_channel_subscription`、`ensure_topic_subscription`へ至るcall pathがなくなった。
5. 許可Contextでのdocs readと既存local projection更新は維持する。

明示mutation用`dome_connection_context_replica`は`dome_connections.rs:512`に残り、privateでは`ensure_private_channel_access`を**write_stateより前**に実施した後、既存のprivate更新・topic subscription・replica openへ進む。これにより未加入channelは共有副作用前に拒否し、提案／承諾／撤回／解除が従来必要としたepoch更新を一律禁止しない。owner署名、Context/Instance generation、queue/slot/cycle、block terminal、drainingの処理はdeltaで変更されていない。

CodeGraph callersと限定識別子検索を再実行し、read helperのcallerはlistのみ、mutation helperのcallerはcreate / accept / withdraw / proposal terminal / connection terminalの5件と確認した。初回の逆引き表にあるruntime / CLI / Tauri、preview / prepare、delete、block reconciliationのlistはすべて修正されたread経路へ接続する。したがってINV-3のpreview→listにあったF1の伝播も解消する。

#### F1の修正前後の証拠

- 新test `restored_friend_only_map_read_does_not_rotate_or_subscribe`は、FriendOnly ownerの既存capability、active参加者、mutual=falseのrelationshipを作り、rotation_required=trueを先に検証する。別AppServiceへcapabilityだけを復元し、listを呼ぶ。
- `.codex/plans/issue-1025-private-read-red.log`を実読: 同testが修正前にFAILし、`viewing must not rotate the epoch or publish records`、writesの期待5に対して実際10と記録されている。初回のsource指摘だけでなく、共有writeの増加が再現された。
- 修正testのassertionは、docs writes、secret登録回数、private subscription registry、topic subscription、current_epoch_idを比較する。許可readに伴うdocs query回数やlocal projectionの更新まで0と誤指定していない。
- `.codex/plans/issue-1025-audit-fixes-rust.log`を実読: このtestとunknown channelのI/O/projection拒否を含む接続9 testsがPASS。正常proposal→accept→revoke、owner制限、双方向block、unblock非復元も含む。

### F2の解消と影響先

- `DomeConnectionModel.ts:20`はrecordのagreement Contextと、現在endpointのInstance ID / generation / owner / directionをすべて照合する。proposalにも同じContext/current endpointフィルタを適用する。
- `DomeConnectionPanel.tsx:51`で占有はactive/drainingだけになり、acceptedを現在slotの占有へ変換しない。
- acceptedを含む現在endpointの非revoked recordには明示revokeを残す（`:112`）。これにより有効化されなかった合意のcleanupを失わず、旧世代recordへの操作も表示しない。
- current slotへの候補選択・明示createは既存owner、candidate、ready、pending lockで制御し、scope key／retry ID／署名backendは変更しない。

#### F2の修正前後の証拠

- 新model testはold-generation acceptedがconnectionとして選ばれず、current slotがopenになることを検証する。
- 新component test `a recreated Dome can propose into a slot with an old accepted record`は、generation=2のDomeでEast→候補d→Proposeを実行し、既存create actionへContext/a/d/eastが渡ることを検証する。表示上のopenだけで実用途到達の代替にしていない。
- 監査側で初回と同じ形の入力を実source SSRで再実行した。旧accepted、current generation=2、current component孤立に対して `{"status":"open","occupied":false,"current":false}` となった。初回のaccepted/occupied=trueからの修正を確認した。

### inventory・AC / INVARの現在判定

| 固定行 | deltaで確認したこと | 現在の分類 |
| --- | --- | --- |
| INV-1 | private readからdomain write／AppService購読開始を除去。旧世代slotを除外。readのscope/late response/error/refresh実装は未変更で限定testsもPASS | 適合 |
| INV-2 | 旧世代recordが現在候補を隠さず、実提案へ到達する。acceptedの明示解除を維持。既存mutationのowner・署名・generation・block・draining契約を維持 | 適合 |
| INV-3 | preview / prepareから使うlistにもread修正が適用される。neighbor cancel / access / assets / boundary優先処理は未変更で限定testsもPASS | 適合 |

| 条件 | 更新したevidence | コード・contractの判定 |
| --- | --- | --- |
| AC-1 | 新component testで再作成後の候補→提案到達。従来accept / withdraw / revoke testsの成功を再確認 | PASS |
| AC-2 | mapのcomponent/current相対座標・未知名扱いは未変更。modelの5 tests成功 | PASS |
| AC-3 | F2の旧世代占有を除去。read failure / stale snapshot、draining優先、neighbor error区別を再確認 | PASS |
| AC-4 | 方位keyboard、draft保持、カテゴリpaneの状態は未変更。今回の21 testsにkeyboard選択を含む。最終nativeゲートは別途未完了 | PASS（実機最終ゲートとは区別） |
| INVAR-1 | F1のread起因のrotation/handoff/domain writeを除去し、禁止write不変test成功。正当な明示mutation、block terminal、drainingを維持 | PASS |
| INVAR-2 | memoryで加入済Contextだけを解決し、pending grantで新epochを取得・登録しない。unknown channel I/O/projection拒否test成功。同Context候補／アクセス許可後asset境界は未変更 | PASS |

### その他の差分

- 旧`.metaverse-connection-slots` CSSは現sourceで参照0であることを限定検索した。旧2列フォーム用スタイルの削除は現UIの動作を変更しない。
- 方位button内smallの`color: inherit`は既存Buttonの配色を継承する。map legendは`ol`へdecimalを明示する。新しいkeyboard／pointer／network動作を追加しない。
- 追加のconfirmed dark / narrow light PNGを目視し、現在地・方向・edge・詳細が判別できることを確認した。これらはStorybook fixture画像であり、複数Dome実通信や最終Windows実機結果へ読み替えない。凡例CSS追加後の最終browser gateは親担当の実行結果で確定する。
- oversized baselineはread helperを既存supportへ移して`dome_connections.rs`を1025行へ戻した差分であり、新しい上限増加ではない。

### Deltaで実行・確認したvalidation

監査側:

1. HEADが`77db7d222626c3085b42ece38983cf5d0652caff`であることを開始時と終了前に確認。対象コードに未コミット差分なし。
2. CodeGraph explore/node/callers、private pending判定のcallee、read／mutation全callerと旧CSS参照を限定検索。
3. 初回と同じ限定Vitest 4 filesを再実行 → **21 tests PASS**、5.68秒。
4. F2の実source SSR再現 → open / occupied=false、exit 0。
5. 修正前private readのFAILログ、修正後backend接続9件PASSログ、関連frontend5 files/25件PASSログをtest sourceのassertionと照合した。重いcargo／frontend全suiteは重複実行しなかった。

### Non-blockerと監査の限界

- **許可docs readに内在する同期**: `IrohDocsSync::open_replica` / `query_replica_with_policy`は`ensure_replica`を通り、必要に応じて`doc_start_sync`や内部`doc.subscribe`を使用する（`crates/docs-sync/src/iroh_sync.rs:135`, `:252`, `:323`）。今回の境界は、AppService topic/private subscriptionの新規起動とdomain record／epoch／participantの共有writeを閲覧から起こさないこと。既知・許可Contextのdocs取得まで「通信0」「すべてlocal-only」「内部同期購読0」とは主張しない。readが内部同期を使う事実は既存の許可readの実装であり、Connectionやprivate epochを自動変更するF1とは区別する。
- 新規blocker、未知Contextの追加探索、禁止されたConnection mutationは発見しなかった。一般的な将来hardeningをClose条件に追加していない。
- 必須CI、desktop-ui-check、Windows実機の最終証拠、merge tree一致は本delta判定に含めていない。親担当が確定結果を追記するまでmerge/Close完了とは扱わない。
