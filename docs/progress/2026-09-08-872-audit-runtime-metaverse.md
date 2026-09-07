# Issue #872 Phase 0–2: runtime・IPC・Metaverse/Dome監査

## 対象と終了条件

- 親Issue: #872。担当groupは`INV-2`と`INV-4`。監査基準commitは`ac9946d66aaf197204f669a10701fac8a13877e9`、調査日は2026-09-08。
- `AGENTS.local.md`、`docs/README.md`、`docs/runbooks/dev.md`、`docs/runbooks/issue-lifecycle.md`、`REFACTORING.md`、前回責務計画を確認した。DomeについてADR 0038/0040/0042/0044/0045、CLIについてADR 0049、UI配置についてADR 0014、`DESIGN.md`、UI実装配置を参照した。
- 固定集合は以下のinventory。全groupについて入口とsinkの所有者を確認し、圧力のある6候補を分類した時点で終了する。個々のpublic API全挙動を再監査するcampaignではない。候補外の境界変更を発見しても同じ子へ取り込まない。
- 製品コード、test、schema、依存、GitHub、commitはこの担当では変更しない。実施候補2件と、各々の先行contract候補を親へ渡す。
- `.codegraph/`があり、`codegraph explore` / `codegraph node`を先に利用した。存在しない`runtime.rs`等は現行`runtime/mod.rs`へ訂正した。CodeGraphの汎用名による依存推定には偽一致があるため、採用候補のcallerは下記の正確なsymbol検索で照合した。「no indexed dependents」を未使用の根拠にはしていない。

## 変更前baselineと前回計画との差

件数は`git ls-files <path>`、履歴数は`git log --since=2026-05-29 --format=%H -- <path>`の件数である。mergeされた変更が各pathに現れたcommit数であり、PR数や不具合数とは異なる。行数は`[System.IO.File]::ReadAllLines(<absolute path>).Length`で測定した。CodeGraphの末尾空行を含む表示行数とは1行異なる。

| 範囲 | tracked file数 | 2026-05-29以降のcommit数 | 責務の現在判定 |
| --- | ---: | ---: | --- |
| `crates/app-api/src` | 82 | 63 | 公開domain API、`service/`内部helper、`ServiceHandles`は維持。Dome authority、access、hosting、layoutが追加された |
| `crates/desktop-runtime/src` | 82 | 95 | runtime orchestrationに加え、`host/`がGUI/CLI共通のprofile・同意・復元・runtime交換を所有 |
| `apps/desktop/src-tauri/src` | 26 | 53 | command adapter、Tauri lifecycle、OS notification、updater。startup/restoreのdomain判断は共有hostへ抽出済み |
| `crates/kukuri-cli/src` | 35 | 3 | 前回にはなかったCLI/daemon、protocol、command登録簿、要求単位実行。別profileから同じDesktopRuntimeを利用 |
| `crates/metaverse-host/src` | 4 | 7 | 共通Rust physics authority。owner-deviceとCNの双方が同じ`DomeSessionRuntime`を利用 |
| `apps/desktop/src/components/extended/metaverse` | 36 | 27 | 前回のpanel分割後、session/transition/admission/recovery/audio/budgetが追加された |

| 具体的path | 行数 | commit数 | 行数以外の変更圧力 |
| --- | ---: | ---: | --- |
| `crates/desktop-runtime/src/runtime/private_channels_game_api.rs` | 1100 | 15 | hosting delegationとlayout再起動が同じCN transfer副作用sequenceを二重所有 |
| `crates/app-api/src/dome_hosting.rs` | 1096 | 7 | docsのowner-signed recordとhost session lifecycleを合成。runtimeと共通化してよい境界ではない |
| `crates/metaverse-host/src/lib.rs` | 1191 | 6 | budget、safe spawn、snapshot、simulationが同じauthority stateを利用。現時点で移動自体の価値は未立証 |
| `apps/desktop/src/components/extended/metaverse/useMetaverseRoomSession.ts` | 1272 | 14 | transition attempt制御とscene/admission/chat/audioのstateが同居。#826のack喪失修正がhookとrecovery helperへ波及 |
| `crates/desktop-runtime/src/identity.rs` | 1004 | 7 | keyring/file/legacy読込は契約と回帰testがある。大型という理由で互換経路を消さない |

前回計画から維持するものはAppServiceの公開domain/内部helper分離、runtimeによるsecret永続化、thin Tauri adapter、docs/blobs canonical sourceである。WP-C2のcapability write-through callbackをwrapperの個別persistへ戻さない。`persist_private_channel_capabilities`の本番callerはruntime構築時に登録するcallback1箇所であり、AppService側の`persist_callback` testsが失敗伝播まで保護する。`#476`→`#479`→`#480`の修正履歴は再統合ではなく、この既存境界を維持する根拠になる。

`#896` (`01b65483`)で共通ClientHost、`#899` (`e4b37b59`)で同意/復元、`#901` (`a1c2696b`)でCLI全操作が実装されている。旧「desktop processだけのruntime」を前提に新しいhost層を重ねない。`Cargo.toml`に`crates/client-host` / `client-daemon` / `client-protocol`というcrateは存在しない。実体は`desktop-runtime/src/host/`と`kukuri-cli/src/{daemon,protocol,session,dispatcher}.rs`である。`crates/iroh-node`のEndpoint/Router/Docs/Blobs所有者は前回WP-H2で抽出済みであり、`INV-1`に属する共有基盤として維持する。

## 固定inventoryと全memberの列挙

各path以下の全tracked memberは`git ls-files <path...>`で再列挙できる。Rust入口は`codegraph node <file> --symbols-only`、補完は`rg -n 'pub(\([^)]*\))?\s+(async\s+)?fn\s+' <paths> -g '*.rs'`。Tauriは`lib.rs`の`tauri::generate_handler![...]`ブロックを正本に列挙し、基準時141登録。CLIは`commands/mod.rs::registrations`の8登録groupと`registry.rs::CommandRegistry::builtin`のbuilt-in登録を合わせる。CLIの完全な実行時集合は`CommandRegistry::builtin().schema_document()`と同じ登録簿から再現でき、個別callerの確認には`rg -n 'registrations\(|CommandRegistration|command\(' crates/kukuri-cli/src/commands crates/kukuri-cli/src/registry.rs`を使う。登録簿で登録された名前、handlerのmatch arm、schemaのmatch armを照合する。

| ID | 全memberを定めるpath/入口 | helperと順方向のsink | sinkからの逆方向・保護 |
| --- | --- | --- | --- |
| INV-2A | `crates/app-api/src/lib.rs`が登録する公開domain module全件、`service/`、`views.rs` | Tauri/CLI→runtime request adapter→AppService→ServiceHandlesのstore/docs/blob/transport。private capabilityはcallback経由でruntime secret永続化 | `ServiceHandles`を構築するruntime、harness、testsを逆引き。`set_private_channel_capability_persist`の本番登録はruntime/harness。wire fixtureとprivate persist callback tests、domain別testsを維持 |
| INV-2B | `crates/desktop-runtime/src/runtime/`全8 moduleと`requests.rs`、`attachments.rs`、`discovery.rs`、`stack.rs` | request→AppService。またはcommunity_nodeの同意preflight→token/auth→HTTP。runtime構築→SqliteStore / SharedIrohStack / identity読込 | `new_with_config_and_identity_and_discovery`のconstructor chain、`ClientHost::build_detached_runtime`とharness/testsを逆引き。startup時CN relay/seedを未検証設定から適用しない |
| INV-2C | `apps/desktop/src-tauri/src`の全登録command、`lib.rs`setup/restart、`desktop_lifecycle.rs`、`restore_lifecycle.rs`、`state.rs` | invoke gate→Ready確認→AppHandle state→ClientHost/runtime。setupはprofile lease→restore journal回復→同意→runtime。update/exitはlifecycle guard | `with_desktop_startup_gate`の全登録対象、`orchestrate_restore_activation`のTauri起動/consent callerを確認。`invoke_gate`のnon-ready/exit tests、restore startup phase tests |
| INV-2D | `desktop-runtime/src/{host,accounts.rs,identity.rs,backup.rs,backup/}`、Tauri identity/backup command、CLI lifecycle/session | profile lock / account選択 / restore install→keys、optional secrets、journal、DB交換→runtime再構築 | `persist_keys`の本番callerはaccountsとbackup、`persist_optional_secret`はruntime capability/gossip、accounts移行、backup、CN token/consent/invite storage。秘密purpose・path・keyring account・legacy nsec・restore phaseを凍結。identity_restart、accounts_migration、device_backup/recovery tests |
| INV-2E | `crates/kukuri-cli/src/`全memberと`crates/kukuri-cli/tests/` | main/client→Unix socket→daemon peer UID/frame guard→Dispatcher→登録handler→ClientSession/ClientHost→同じruntime | `ClientSession::start`の本番callerはdaemon。`ClientHost::start_if_consented`はTauri stateとCLI session。`orchestrate_restore_activation`はTauri lib/app_consent、CLI session。profile分離、1入力1handler、timeout後自動再実行0、secret frameをADR 0049とdispatcher/daemon/session testsで固定 |
| INV-4A | `MetaverseRoomPanel.tsx`、`MetaverseScene.tsx`、`MetaverseSceneModel.ts`と`components/extended/metaverse/`全member | panel→session hook→MetaverseRoomActions→runtime API。scene transform→transition coordinator相当処理→prepare/commit/abort→host。snapshot→admitted scene / audio / local last-visited | session hookの本番呼出は`MetaverseRoomPanel.tsx`の1箇所。`recoverDomeTransitionCommit`の本番callerはsession hookのみ。戻り型参照/story/testと実呼出を区別。session10 tests、recovery4 tests、scene/budget/audio/entry tests |
| INV-4B | `app-api/src/{game.rs,dome_hosting.rs,dome_connections.rs,dome_move.rs,service/*dome*,service/*metaverse*,service/spatial_access_support.rs}`、runtimeのDome公開API | context replica guard→owner/access検証→署名record/doc mutation、host session入力、hint、blob stage。runtimeでowner-local/CN target分岐 | `persist_dome_hosting_record`のcallerはowner開始、CN準備、CN有効化、closeの4メソッド。`persist_dome_layout_commit`のcallerはlayout commit1メソッド。`hosting_context_replica`にはprivate channel entry setterも含む。AppService Dome testsとharness |
| INV-4C | `crates/metaverse-host/src/{lib.rs,transition.rs,support.rs,tests.rs}` | AppServiceまたはCN→`apply_signed_input_at`→signature/session/preflight→physics mutation→signed admission/stream snapshot。timer→advance/expire→participant/guest除去 | `apply_signed_input_at`の外部本番callerはAppServiceとCN input handler。`signed_snapshot_inner`はstream/admissionの2wrapper。transition admission/exit/revoke/evictはAppService access/topologyとCNから逆引き。host21 testsがcapacity、署名input、safe spawn atomicity、restart、ring、expiry、ownerを保護 |
| INV-4D | `desktop-runtime/src/community_node/dome_hosting_support.rs`と`cn-user-api/src/dome_hosting.rs` | CN request adapter→configured node/current consent→auth→HTTP。CNはlease validation→operational mirror/session、ownerはdurable authority | client側`send_dome_hosting_request`を使う9POST adapterと`send_dome_hosting_get`を使うstatus adapterを全列挙。CN側の詳細所有は`INV-5`。本監査のRT-1はclient2入口の順序だけを対象にする |

逆引きの再現コマンドは、次のsymbol集合に対して`rg -n '<symbol>\(' crates apps/desktop/src-tauri/src -g '*.rs'`、frontendは`rg -n '<symbol>' apps/desktop/src -g '*.ts' -g '*.tsx'`を使う。test参照とproduction callerを分け、public re-exportとmacro登録も確認した。

- Runtime: `assign_dome_hosting_to_community_node`、`activate_dome_hosting_on_community_node`、`build_dome_hosting_assignment_request`、`delegate_dome_hosting`、`commit_dome_layout`。
- Startup/secrets: `persist_keys`、`persist_optional_secret`、`persist_private_channel_capabilities`、`set_private_channel_capability_persist`、`orchestrate_restore_activation`、`recover_device_restore_before_startup`、`ClientHost::start_if_consented`、`ClientHost::build_detached_runtime`。
- Durable/physics: `persist_dome_hosting_record`、`persist_dome_layout_commit`、`hosting_context_replica`、`apply_signed_input_at`、`signed_snapshot_inner`、`prepare_transition_admission`、`commit_transition_admission`、`abort_transition_admission`、`evict_participant`。
- Frontend: `useMetaverseRoomSession`、`recoverDomeTransitionCommit`、`transitionAttemptRef`、`attempt.phase`、`attempt.cancelled`、`attempt.ticket`、`prepareTransition`、`commitTransition`、`abortTransition`。

## 候補分類

| 候補 | 観測事実 | 目標成果 | 優先順位・分類 | 理由と子Issue案 |
| --- | --- | --- | --- | --- |
| RT-1 CN hosting transferの副作用sequence | `private_channels_game_api.rs:348–387`と`:718–758`にassignment→owner activation永続→CN activateが重複。`92ffe9c3`でhosting、`d3d292fe`でlayout再起動が追加。前後の業務処理は異なるがこの3段は同じ契約を所有 | 同じtransfer完了sequenceの所有者2→1。public入口は2のまま、同意guardとowner署名の層は維持 | P1・実施 | C-RT先行→R-RT。失敗境界を固定してから同一runtime内のprivate helperへ抽出 |
| RT-2 GUI/CLI startupをさらに一つの巨大facadeへ集約 | 9/5に`#896/#899`でClientHost/consent/restoreを抽出し、9/7にGUI exit/updateを追加。Tauri固有side effectとCLI socketが残る | 新たな正本削減の必要性は未立証 | P3・延期 | 新たな共通判断の重複や変更漏れが観測された時に再評価。今回の古い責務図だけで既実施抽出をやり直さない |
| RT-3 identityのlegacy/keyring/file経路撤去 | 1004行だが`legacy_nsec_file_still_loads`、keyring障害/原子的file保存testsが実在。REFACTORINGのsunsetはlegacy検知時fail-loudと一体 | 無条件の撤去は鍵喪失・別identity生成を引き起こし得る | 却下 | 行数・古さだけの削除は契約違反。仕様変更を伴う撤去は別のmigration/fix判断 |
| MV-1 frontend transition attempt所有者 | hook内でphase/cancelled/ticket/refを直接更新する箇所が9、abort/begin/commitとtransform分岐に分散。`#826` (`6692629f`)のack喪失fixがhook・recovery helper・両testへ波及(6 files、426追加/66削除、ADR含む) | transitionのattempt、prepare/abort/commit再試行、source cleanupを単一の専用coordinatorへ。scene/admission/chat/audio hookから直接protocol状態更新9→0 | P1・実施 | C-MV先行→R-MV。現存recovery helperの判断を再利用し、timeout/取消挙動変更を混ぜない |
| MV-2 physics/budget/snapshotを行数で分割 | `DomeSessionRuntime`1191行に同一authority stateとbudgetが集まる。transitionは既に別module。21 testがbefore-mutation rejection等を保護 | 分割だけではcoupling減少を証明できない | P3・延期 | 次の具体的変更がstate依存のため阻害される証拠が必要。抽象physics engine層、新crate、budget挙動変更は今回対象外 |
| MV-3 runtime/clientとAppService/hostのtopology・access検証を共通化 | runtime prepare(:503)とAppService prepare(:310)に同様のtopology検証。preview(:373)はtyped denialを返しprepareはerror。client preflightとhost authority再検証という異なる利用者を持つ | 境界をまたぐ共通化の価値より、guardの責務混同リスクが大きい | 却下 | 同じ見た目だけで共通guardを削らない。誤判定を見つけた場合は再現してfixへ分離 |

`実施`はPhase 3に向けた選定であり、先行contract未完のrefactorを今実装してよいという判定ではない。各子は区分Cとして独立監査を要求する。

## RT-1の現在の挙動・固定境界

対象sourceは`crates/desktop-runtime/src/runtime/private_channels_game_api.rs`。本番callerはTauri `commands/live_game.rs::{delegate_dome_hosting,commit_dome_layout}`、CLI `commands/live_metaverse.rs`の同名match armであり、各登録簿に両commandが存在する。

1. DelegationはAppServiceで新leaseを署名・docsへ保存して`Transferring`にする。
2. Layoutはhost candidate取得、owner検証、revision更新/operation保存をAppServiceで行う。CN targetかつ`Transferring`の場合だけ後続transferへ進む。owner target、no-op、既にactiveな再試行は同じ扱いにまとめない。
3. 両入口がsigned leaseとmanifestを読んでassignmentを送信する。acceptanceをAppServiceへ渡し、署名検証→HostAccepted/LeaseActivated保存→hintの後、CN activateを送信する。
4. `assign_dome_hosting_to_community_node` / `activate_dome_hosting_on_community_node` / `build_dome_hosting_assignment_request`の本番callerはそれぞれこの2入口だけ。CN HTTPは既存`require_dome_hosting_community_node_consent`を経由し、configured node/current local consent/401再認証の規則を維持する。

末尾のHTTP送信失敗は、すでに保存されたowner activationを巻き戻すコードではない。エラー文脈もdelegationの`prepared Dome hosting view...`とlayoutの`prepared layout commit...`で異なる。この差を「同じだから」と消す抽出は不可。署名bytesのgolden化、docs key/prefix、manifest/schema、public request/response、API URL、同意層、token storage、owner authority、layout operation冪等性は凍結する。

既存保護はAppServiceの`owner_explicitly_transfers_hosting_to_one_community_node`と`owner_layout_commit_is_explicit_idempotent_and_restarts_from_new_revision`、runtime CN dome tests6件、`dome_hosting_lifecycle` scenario。ただしscenarioはMemoryStore/FakeTransport/AppServiceを直接使用し、実runtimeの2入口からCN HTTPへの失敗sequenceは通らない。全Rust caller検索でruntimeの2入口を直接呼ぶtestは確認できなかった。これはその区間の保護不足であり、製品不具合の断定ではない。

## 子Issue草案 C-RT: CN Dome transferの途中失敗をcharacterizationで固定

- Current status: Planned。Scope revision: `2026-09-08-872-C-RT-v1`。基準commit: 上記監査commit(実行時にbefore headを追加)。リスクC。親#872、R-RTの先行条件。
- 種別: contract。Goal: CNへのDome委譲・layout再起動の成功/失敗後に観測できるauthorityと結果を変更前実装で固定し、同じ操作の再試行で誤って旧hostを再有効化しないことを確認できるようにする。
- In scope: `crates/desktop-runtime/src/tests/community_node/dome_hosting.rs`と必要最小の同配下fixture。既存public runtimeメソッドを呼び、local mock CNの応答と保存済みhosting viewを観測する。Non-goals: transfer実装修正、汎用test framework、HTTP/auth/domain policy共通化。
- `AC-1`: delegationとCN layout restartの両入口で、success、assignment失敗、invalid acceptance、activation HTTP失敗を変更前実装に対して実行し、エラーと保存済みlease/activationをassertする。
- `AC-2`: layoutのno-op/owner-hosted/既にactiveなoperation retryではCN再assignment/activationを新規に増やさないことを固定する。既存のcandidate取得など前段I/Oは別に計測し、禁止範囲を広げない。
- `AC-3`: 未同意/失効/未登録nodeの禁止HTTPと既存署名・manifest binding検証を維持し、test-only commitで成功ログを残す。現行実装とADRの不一致が再現された場合はfixへ分離し、refactor開始条件を更新する。
- `INVAR-1`: owner-signed docsがcanonical、CNはmirror。epoch/session/manifest/operation id、HTTP endpoint、IPC DTO、エラー文脈を変えない。
- `INVAR-2`: production control flow/test assertion/既存guardを弱めない。追加testはhelperの呼出順そのものではなく、受信requestと保存後状態・禁止副作用を観測する。

| ID | 入口→helper→sink | transition | 証拠 |
| --- | --- | --- | --- |
| INV-1 | runtime delegation→prepare lease→assignment HTTP→owner activation docs→activate HTTP | TR-1/2/3 | runtime public APIを使う追加contract |
| INV-2 | runtime layout commit→candidate/operation→CN Transferring分岐→同sequence | TR-1/2/3/4 | 同じfailure matrixをlayout入口でも実行 |
| INV-3 | shared CN send→configured node/current consent→token/auth/HTTP | TR-5 | 既存dome_hosting testsとHTTP counter |

| ID | sequence | 期待する観測・禁止副作用 |
| --- | --- | --- |
| TR-1 | prepared→valid assignment/acceptance→activate成功 | owner activationと返却viewが同じlease/session。新旧hostのauthority条件を維持 |
| TR-2 | prepared→assignment失敗/invalid acceptance | エラーを返し、activation HTTPは0。旧epochを再有効化しない。準備済みrecordを削除しない |
| TR-3 | owner activation保存済み→activation HTTP失敗 | 失敗を返し、保存済みactivationを暗黙rollbackしない。後続再試行の観測を固定 |
| TR-4 | no-op/owner-host/active operation retry | CN reassignment/activation不要分岐を維持。no-op/既存active operation retryはrevision/operation数を増やさない。owner-hostの新規layout変更は既存のrevision更新・operation保存・owner再開始を許可する |
| TR-5 | current consent無し/撤回/未登録→両HTTP adapter | 禁止対象HTTP 0、既存エラー。通常の公開policy preflightの許否は既存contractどおり |

- Validation: before/after targeted `cargo test -p kukuri-desktop-runtime tests::community_node::dome_hosting`、`cargo test -p kukuri-app-api tests::dome_hosting`。完了時`cargo xtask rust-test`。追加scenarioを編集する場合だけそのscenario実行。重い全suiteの未実行は理由/補完CIを記録する。
- Rollback: test-only PRを単独revertできる。R-RTが開始済みなら弱体化するrollbackをせず先にR-RTを戻す。

## 子Issue草案 R-RT: CN Dome transfer完了sequenceの所有者を一つにする

- Current status: Planned。Scope revision: `2026-09-08-872-R-RT-v1`。リスクC。親#872、依存C-RT。C-RTのbefore成功証拠が得られるまで製品変更を開始しない。
- 種別: `refactor:extract`。Goal: Dome委譲・layout更新の成功/失敗・再試行結果を保ちながら、CN transfer完了処理の変更箇所を一つにする。
- In scope: `private_channels_game_api.rs`の上記2sequenceと同責務のprivate helper(必要なら`runtime/dome_hosting_api.rs`として配置)。`runtime/mod.rs`はmodule登録/import整理の最小差分だけ。Non-goals: ファイル全体のdomain再分割、public API変更、CN server、app-api authority、HTTPリトライ、no-op判定、トランザクション追加。
- `AC-1`: assignment bundle構築→CN assignment→AppService activation→CN activationの所有者を2から1へ集約し、両入口から利用する。
- `AC-2`: 前段のdelegation/layout判断と後段のview構築をそれぞれの入口に維持し、C-RTの全transition・エラー文脈を同じ条件で通す。
- `AC-3`: caller再列挙、before/after構造指標、required validationと独立監査を記録する。新public API/依存/冗長wrapperを増やさない。
- `INVAR-1/2`と`INV-1`〜`INV-3`、`TR-1`〜`TR-5`はC-RTと同じ契約・集合を子本文にも固定する。C-RTは保護網、この子は構造変更だけを所有する。
- Before/after: `assign_dome_hosting_to_community_node`、`activate_dome_hosting_on_community_node`、`build_dome_hosting_assignment_request`の本番呼出siteを各2→各1、public入口2→2。抽出後helperのcaller2を逆引きし、残る同sequence0を確認する。行数低下は補助指標。
- Validation: C-RT targeted + `cargo xtask rust-test`。起動/永続往復変更時は`cargo xtask e2e-smoke`、CN session/connectivityの振る舞いが変わるならrefactorを停止し別種別へ分け、必要に応じ`community_node_public_connectivity`。IPC/Tauri/TSを変更した場合は対象matrix追加。通常はpublic callerを変えずコンパイルされることを確認する。
- Rollback: このextract PRのみrevertし、C-RTは維持する。schema/DB rollback手順を発生させる変更はscope外。

## MV-1の現在の挙動・固定境界

対象は`useMetaverseRoomSession.ts`の`TransitionAttempt`、`transitionAttemptRef`、`setTransitionPreparing`、`abortTransitionAttempt`、`beginTransitionAttempt`、`commitTransitionAttempt`、`handleLocalTransform`のtransition分岐。`joinRoom`/`leaveRoom`は取消要求のcallerとして含めるが、admission/evacuation/scene stateの再設計はしない。

`abortTransitionAttempt`はcommit送信前だけ取消し、`committing/target_committed`では戻る。prepare後に取消済みなら遅着ticketをabortする。commit応答喪失は既存`recoverDomeTransitionCommit`へ委譲し、同じticket/transformを再送する。authoritativeな拒否またはtarget lease/session置換が確認された時だけrollbackする。ack後に選択Dome、last-visited、handoff transformを更新し、source completeを0/250/1000msで最大3回試す。source cleanup失敗でdestinationを巻き戻さない。

production sinkとcallerは次のとおり。

- `actions.prepareTransition` / `commitTransition` / `abortTransition`: 上記3callback内各1site。既存`MetaverseRoomActions`→shell/runtime API→Tauri/CLIと同じRust domain APIの契約へ達する。
- `recoverDomeTransitionCommit`: production callerは`commitTransitionAttempt`のみ。CodeGraphが示す同名汎用callback由来の他file依存を実callerへ数えない。
- `submitInputForRoom`: prepare、abort、completeに加えadmission/keepalive/move等も利用。sequence正本は既存`sessionSequenceByInstanceRef`であり、抽出先へ複製しない。
- `writeLastVisitedDome`: authoritative Join成功とtransition commit成功のみ。描画選択/previewで書く仕様へ変更しない。

既存10 session testsには中心線通過とlost ackが各1件、recovery helperにはtransient retry/session置換/lookup失敗/invalid ticketの4件がある。precommit取消とsource cleanup失敗をhookの観測で固定する網が不足するため別contractを先行する。`desktop_smoke_metaverse_dome_transition`はRust harnessのscenarioであり、browserのhook非同期競合の代替にはしない。`desktop_smoke_metaverse_dome_recovery`も現行YAMLではconnection解除/topology確認が中心で、audio/scene/ack競合まで成功扱いにしない。

## 子Issue草案 C-MV: Dome transitionの取消と確定後cleanupをcharacterizationで固定

- Current status: Planned。Scope revision: `2026-09-08-872-C-MV-v1`。リスクC。親#872、R-MVの先行条件。
- 種別: contract。Goal: Dome移動の取消・通信断でsource/destinationと画面選択がどう残るかを、現行hookの結果として固定する。
- In scope: `useMetaverseRoomSession.test.tsx`、必要最小の同責務test fixture、`DomeTransitionCommitRecovery.test.ts`。Non-goals: production hook修正、retry上限追加、unmount/取消の新仕様、UI redesign、巨大mock全体整理。
- `AC-1`: prepare中に中心線方向から戻り、ticketが遅着した場合のsource/destination abort・選択維持を固定する。
- `AC-2`: commit応答喪失中は同一ticket/transformを再送し、authoritativeな拒否/target session置換が確認されるまでsource/destinationをabortしないことをhookの結果で確認する。
- `AC-3`: destination ack後のsource cleanup再試行と全試行失敗を固定し、destination selection/last-visitedがsourceへ戻らず、失敗表示が出ることを確認する。
- `AC-4`: 変更前実装で追加testと既存10+4 testsを実行したログを残す。canonical契約との矛盾が再現されたらfixへ分離し、構造変更を始めない。
- `INVAR-1`: ADR 0042/0044/0045のauthority、geometry、safe admission、同一ticket再試行、ack後handoff、audio停止の境界を保つ。signature/wire/IPC/domain requestを変えない。
- `INVAR-2`: scene/chat/audio/admission/entry候補順は既存のまま。非同期source cleanupやinput sequenceを都合よくmockしてassertionを省略しない。

| ID | 入口→helper→sink | transition・証拠 |
| --- | --- | --- |
| INV-1 | transform→zone判定→begin/abort→source input + target prepare/abort | TR-1。deferred responseを使うhook contract |
| INV-2 | 中心線通過→commit→recoverDomeTransitionCommit→target HTTP/IPC | TR-2/3。hook+既存helper tests |
| INV-3 | ack→handoff/selected room/last-visited→source complete/presence leave | TR-4。cleanup failureを使うhook contract |
| INV-4 | joinRoom/leaveRoomからの取消要求→同じattempt guard | TR-1/2。既存挙動の観測、commit後の新取消仕様は追加しない |

| ID | sequence | 許可するI/Oと禁止副作用 |
| --- | --- | --- |
| TR-1 | preparing/provisional→中心線前へ戻る/入場変更要求 | 現行precommit abortを維持。取消対象transition attemptからのtarget commit/handoff/last-visited更新は禁止。独立したjoinRoom/leaveRoom本来のselection/admission/presence/audio副作用は許可し維持する |
| TR-2 | committing→ack喪失/hosting lookup失敗→同じsession | 同一ticket/transformのcommit再送。適用不明のままsource/destination abort禁止 |
| TR-3 | commit失敗→確定拒否/target session置換確認 | 現行rollback/abortとerror表示。失敗を成功handoff扱いしない |
| TR-4 | target ack→source complete失敗/成功 | 最大3回のsource cleanup。成功後presence leave。全失敗でもdestination currentを維持 |

- Validation: `npx pnpm@10.16.1 --dir apps/desktop test src/components/extended/metaverse/useMetaverseRoomSession.test.tsx src/components/extended/metaverse/DomeTransitionCommitRecovery.test.ts`をbefore/afterで実行し、完了時`cargo xtask desktop-ui-check`。test-onlyのため描画変更なし。browser成功をOS/WebView確認へ置き換えない。
- Rollback: test-only PRを単独revert可能。R-MVに着手後は先にR-MVを戻し、保護網だけを外さない。

## 子Issue草案 R-MV: Dome transition attemptを専用coordinatorへ集約

- Current status: Planned。Scope revision: `2026-09-08-872-R-MV-v1`。リスクC。親#872、依存C-MV。
- 種別: `refactor:extract`。Goal: Dome参加者の移動・取消・復帰結果を維持したまま、transitionの非同期状態と副作用の所有者を一箇所にする。
- In scope: `useMetaverseRoomSession.ts`の上記attempt関連処理と、新しい同directoryの専用hook/coordinator1件。`DomeTransitionCommitRecovery.ts`の既存判断を利用する。型import/テストの最小同期を含む。Non-goals: scene/budget/physics/audio/room admissionの再構成、入力sequenceの別正本、routing/global state、HTML/CSS、IPC変更。
- `AC-1`: attemptのphase/cancelled/ticket/refへの直接更新9siteを専用ownerへ移し、元session hookの直接更新を0にする。hookは移動・取消要求と確定handoff通知を通じて利用する。
- `AC-2`: prepare/abort/commit/source cleanupの副作用実行を同ownerへ集約し、既存retry helperの意味を維持する。別state copyや汎用workflow frameworkを追加しない。
- `AC-3`: C-MVの全transitionと既存session/recovery testsが変更前後で同じ結果を示し、public戻り値/利用componentのAPI・描画を維持する。
- `AC-4`: caller/sink再列挙、構造before/after、path validation、独立監査を記録する。
- `INVAR-1/2`、`INV-1`〜`INV-4`、`TR-1`〜`TR-4`はC-MVと同じ契約・集合を子本文へ固定する。
- Before/after測定: `rg -n '^\s*(attempt\.(phase|cancelled|ticket)|transitionAttemptRef\.current)\s*=(?!=)' --pcre2 <session hook>`で9→0。`prepareTransition/commitTransition/abortTransition`の直接呼出を各1→0(session hook)、各1→1(coordinator)と確認する。owner外からmutable attempt objectを露出させない。
- Validation: C-MV targeted、`DomeTransitionModel.test.ts`、`DomeEntryModel.test.ts`、`MetaverseRoomPanel.test.tsx`、`cargo xtask desktop-ui-check`。Rust boundary不変は既存transition/entry/recovery scenariosとの対応を確認し、scenario編集時は該当scenarioを実行する。Metaverse操作への影響があるためADR 0014に沿う対象OS/WebViewの移動/退出/input ownership確認を記録し、未確認なら明示する。
- Rollback: このextract PRのみrevertしC-MVを残す。retry/取消/authorityの変更が必要になった場合は停止して別fix/featureへ分離する。

## 検証結果と監査終了判定

- 実行した確認: CodeGraphのsource/symbol/caller探索、全tracked path列挙、2026-05-29以降のgit履歴、candidateの全caller/sink再検索、ADR/test/scenario内容の照合、line/参照siteの測定。重いbuild/testはこの担当では実行していない。親のPhase 0 validation記録へ対応付ける。
- 現時点の既知fail: この担当の実行結果から製品testのPASS/FAILは判定していない。親担当が同一commitのWindowsで`cargo test -p kukuri-app-api --features iroh-integration-tests metaverse_room_events_replicate_between_iroh_peers -- --nocapture`を実行し、`tests/game.rs:141`の`list rooms: Dome preset manifest is unavailable`を再現した(1 failed、192 filtered、0.30秒)。原因は未確定であり、親のH-04別fix候補へ分離する。RT-1/MV-1へ混ぜず、前回の成功記録を現headの成功へ流用しない。
- 分類: 6候補中、実施2、延期2、却下2、未分類0。先行contract2件。新たな製品不具合はこの静的監査だけでは断定しない。
- Phase 0–2として必要な根拠と子Issue草案を親へ引き渡した。C-RT/C-MVの保護実装、R-RT/R-MVの構造変更、child/parent独立監査、completion baselineはPhase 3以降に残る。
