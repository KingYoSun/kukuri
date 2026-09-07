# #905 Linux Deb追加の独立監査

## 対象と判定

- 基準commit: `c4616fc706b94150ac6c2ac06aec68bc1c2b0f5a`
- 製品コード対象commit: `747a7440e9c35509f77948c2d73d11a0ce5eedc2`
- Scope revision: `2026-09-07-linux-deb-release-addition-v2`
- リスク区分: C
- 固定条件: Issue #905のAC-D1〜6、INVAR-D1〜5、INV-D1〜5、TR-D1〜5（TR-D4a〜d）
- 判定: **PASS**。製品コードのblocker 0、固定inventory 5群はすべて適合、不適合0、未分類0。
- 実装担当とは別の2担当が、固定条件・登録点・対象commitからコードを再構築し、後述の実機／CI証拠を独立確認した。初回の証拠待ち判定は今回のPASSで置き換える。

## 更新境界の独立監査

担当`issue905_boundary_audit`。初回`0085ee18`のコード監査を基に、`747a7440`のdesktop template差分と実機証拠を補完した。

INV-D3を6群に再構築し、すべて適合とした。

1. `DesktopShellPage`の初回／30分周期と`ReleasePanel`手動確認 → store → `check_app_update`。
2. 明示download → `download_app_update` → locked upstreamの署名検証済みbytes返却。
3. 更新適用の2ボタン → `install_app_update`／`install_checked` → `deb_update::install`。
4. `restart_after_update` → `require_installed` → `request_exit` → host停止 → 元GUI権限での再起動。
5. 更新3command＋restartのstartup／終了gate、upstream updater4commandのWebview ACL拒否。
6. CLIの更新3command＋restart除外と登録簿。

explicit Deb targetとraw manifest照合がAppImage fallbackを遮断する。署名検証済みbytesのみをprivate stateへ保存し、適用試行時に消費する。memfdのWRITE／GROW／SHRINK／SEALによって内容を固定し、同じ親processのFDをmetadata検査とroot dpkgが開く。権限境界に可変一時pathやpasswordを渡さない。

`pkexec --disable-internal-agent /usr/bin/dpkg --install`は1回だけで、取消後のsudo／zenity／kdialog fallbackはない。失敗後はrestart gateを閉じ、成功時も`dpkg-query`のinstalled＋version一致を要求する。再起動pathはTauriの起動時cacheを使い、dpkg置換後の削除済み旧pathを再取得しない。

独立実行したfrontend 5files／28testsはPASS。Windows backend testはcompile後のDLL入口解決エラーで未実行と記録し、Linux実機とCIの成功で補完した。

SSHで実機の対象7ソースのblob一致、検証Deb3本のhash、OS承認→旧host正常停止→UID1000の新GUI起動、取消時の`Request dismissed`と同じPIDの継続を確認。取消後から正常終了まで追加認証・再起動はなかった。実行中ELF一致とGUI下書き表示は主担当の実機観測を、installed ELF／実Deb内ELF一致と保存proofは独立確認した。AC-D4／INVAR-D3〜5はPASS。

## Package／releaseの独立監査

担当`issue905_package_audit`。INV-D1／D2／D4／D5の4群を再生成し、順方向とsinkからのcaller逆引きを確認した。すべて適合、不適合0、未分類0。

- 同一Tauri buildから両GUI形式を生成し、各本体・署名を検証。Debの実archiveでmetadata、ELF、許可path、desktop／icon／deep-link、依存、notice、script不在を確認する。
- Deb専用templateの`%U`を実archiveから確認。ユーザーデータpath、追加ELF、setuid、maintainer scriptsは拒否する。identifier／GUI保存先は不変。
- native collectorは実Debを再検査してreportと一致させ、Deb／payload report／sourceのhashをcomplianceと集約へ接続する。system依存の共有libraryをAppImage同梱runtimeの資料で証明したとは扱わない。
- 既存4targetを維持し、Windows／AppImage／Debの3entryを生成する。同source／version／公開鍵、distribution署名、checksum、実署名verifierを経てpublishへ進み、既存asset非置換・完全upload前draftを維持する。
- README en／ja、ADR0049、導入／回復／release手順と未公開境界を照合した。

独立実行のPython contractsは34件、PowerShell集約・3entry署名wrapperはPASS。PyYAML不足はCI同版6.0.3の隔離環境で補完した。stub wrapperを実cryptoの代替にはしていない。

実機Deb `99.0.1`を独立に再検査し、6file payload・control＋md5sumsのみ・required5depends・desktop entryを確認した。[Linux package CI](https://github.com/KingYoSun/kukuri/actions/runs/34079424398)の対象SHAと成功を直接確認し、ログ上の両形式署名、Deb配置／再導入／削除・sentinel、backend53tests、実updater2tests成功を照合した。CI DebのSHA-256は`f5df0f40117759701a3f61f07471bac733531e389be652ffb8cb1a9078070bec`。

検証package削除後の登録・binary・desktop entry不在とprofile保持をSSHで確認した。更新前／更新後／取消前／取消後／削除後の5proof JSONは、すべてSHA-256 `a21703decad751c0224d10b254587e54a25dda27635432565317cf99a670a56d`で一致。account registry、DB保存先、32tables、保存下書きを保持した。DBの業務tableは空であり、投稿保持の証拠として数えない。検証秘密鍵・user unitsも残っていない。

## 非blockerと引渡し

- Windowsローカルのtest起動DLLエラーは環境上の未実行として明示し、Windows CIとLinuxの実backend検証で補完した。
- 初回DebのURL引数欠落は実物検査で再現後、templateを修正して実物・CIを通過。検査の緩和は行っていない。
- 追加distribution／architecture GUI、全OS連携の再walkthrough、任意dpkg部分失敗の自動rollbackは固定Non-goal。
- 配布用署名・最終source資料集約・Debを含む公開URL確認は#890の公開工程。今回のPASSはRelease公開済みを意味しない。
- 最終記録commitは製品コードを変更しない。記録deltaの監査と最終headのCI結果はPR #906へ記録し、merge後に対象treeを照合してIssueをCloseする。

詳細な実行条件とAC／INVAR対応は[作業記録](2026-09-07-issue-905-linux-deb-updater.md)を参照する。

## v3承認追加の監査境界

2026-09-07に利用者が既存通知testの安定化を承認した。Scope revisionは`2026-09-07-linux-deb-release-addition-v3`。上記v2のDeb製品監査は当時のPASSとして維持する。追加差分は通知fixtureの背景Timeline縮小、Threadの全45件・2page／target維持contract、対応する作業記録のみであり、Deb製品入力・固定inventoryは不変。

v3では全既存assertions／Threadの新旧ページ／通知入口を維持したまま、背景側の重複描画を減らしたかを独立に確認する。timeout延長・retry・skipや製品コード変更による成功扱いはしない。追加差分の対象head・判定とCI結果はPR #906の最終監査コメントに記録する。
