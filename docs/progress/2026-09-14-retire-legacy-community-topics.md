# 旧Community Nodeトピックの廃止

## 範囲・承認・完了条件

- 本番反映・検証済み。ユーザーの旧topic購読破棄・DB削除依頼に基づく。対象は本番nodeの `kukuri:topic:demo` / `iroh` / `nostr` / `operators` の4件。
- 区分C（永続データ削除）。一般topicや利用者端末へ削除命令は送らない。監査ログ・復旧backupは保持する。
- AC-1: 管理preview/applyで4topicを対象外にし、既定general/test/devだけを購読する。
- AC-2: 4topicの索引・申請・旧投稿にだけ属する派生判定をDBから削除する。旧docsレプリカも削除し、restartで開かれないことを確認する。
- AC-3: 既定3topicのDB/レプリカ、endpoint identity、既存運営監査を保持し、検索と監視が正常。
- INVAR-1: 対象を削除前に列挙・固定し、backupを取得。公式ストアAPIで停止中DBのコピーを編集し、対象外entryとauthor保持を照合してから反映。
- INVAR-2: 共有subject・blobや保全/通報に関連する状態は参照確認なしに削除しない。全DB/volumeの初期化をしない。

## 作業・状態遷移

| ID | 入口→sink | 必要な確認 |
| --- | --- | --- |
| T1 | IAP管理→supported_topics/operator_actions | previewで4件固定、applyと監査ID、既定3件保持 |
| T2 | 停止済みindexer docs.redbのコピー→公式remove_replica→検証済みコピー | 原本不変、対象4namespaceだけ消失、非対象entries/authors一致、再open検証 |
| T3 | 旧レプリカのobject集合→Postgres/ArcadeDB | 依存・共有参照を確認、対象限定transaction、前後件数 |
| T4 | indexer再開→検索/monitor | 3scopeだけopen、旧topic走査なし、既定索引保持、独立監査 |

停止中の反映失敗は元docsコピーへ戻せる。DB削除はtransactionで確定し、件数不一致ならrollbackする。同名topic IDは同じP2P名前空間のため、将来再購読すると他peerの旧履歴が再流入し得る。完全な新履歴には新しいIDが必要であることをユーザーへ説明済み。

## 削除前の対象とbackup

- 管理設定7topic中の旧4topicを解除。既定 `general/test/dev` の設定・既存監査・indexer identityは保持。
- 対応する旧docs namespaceは、現行の `blake3("kukuri-docs:topic::<topic ID>")` → NamespaceSecret → NamespaceIdから算出。公式Store APIで原本のコピーを列挙し、demo 52 records（8 object state）、他旧3topicは0 recordsだった。
- 8 object IDsは先行調査の7取得失敗対象に、撤回済み `208faf488d12ef185278c119b2eb091996c2863144580e99a93e5b884b75f750` を加えた集合。削除SQLはその8 IDを固定。
- 直前に `kukuri-backup.service` を実行しResult=success / ExecMainStatus=0。
- indexer停止後、`/var/lib/kukuri/maintenance-legacy-topics-20260914/`（root、0700）にdocs.redbとblobs.dbの復旧コピーを保持。
- docs原本SHA256: `59e447c8023105bfb4981127e8a15c4322ef5d4bdefdede7ad0665cc6393aabe`。

## 運営操作（AC-1）

IAPのpreviewでactor=`ops@kukuri.app`、対象4件・非同期索引解除の影響を確認しapply。各行はpresent=true→false。

| topic | 監査ID |
| --- | --- |
| demo | `9b41f9f3-9cea-4270-845e-fdd1f83bd931` |
| iroh | `4788d641-0d70-4314-868f-91b537cd36aa` |
| nostr | `677f0ee1-1e49-4faf-9d80-60e5e5b09bce` |
| operators | `0861f924-2aba-49a7-b647-41f78c739776` |

## データ削除（AC-2 / AC-3）

1. `scripts/community-node/retire-legacy-docs`を使って停止済みdocsの新規コピーだけを編集。旧4namespace削除後は8→4 namespaces（既定3topicと対象外の1namespace）。非対象SignedEntryの全serialize bytes・tombstoneとauthor鍵bytesが一致し、再openでも一致、原本hash不変。対象は4namespace内の52 records。
2. SQLは同一transaction内で11関連テーブルをSHARE ROW EXCLUSIVEロックした後、共有索引・通報・保全参照がないことを検査。旧4scope索引0、旧scope申請0、旧post単位のsigned event/risk signal/subject author0、旧post scan verdict8行を削除。結果はscan verdict8→0。他scopeの索引8行・非対象verdict15行・運営監査件数を前後比較し一致してCOMMIT。
3. ArcadeDBはscopeごとのbind parameter DELETE。旧scope投影はもともと0行で、処理後も0、非対象8行の全列JSONが一致。途中失敗時は再実行できるが一括transactionではない。
4. 検証済みdocsコピーをroot:root / 0600で配置。適用直前に停止状態、原本hash、出力hashを照合した。出力SHA256は `ac33daa8f02dd9ba1c8ef75c8c0462bdeeecfdfeb41813b94606a8de3416341e`。
5. endpoint-secret/default-authorのSHA256が削除前後で一致。共有blob storeとblob hash単位のsafety記録はtopicを再登録する状態ではないため保持した。バックアップ・監査の消去や物理的な全bytes抹消を行ったものではない。

## 再開・検証

- 2026-09-14 03:13:29 UTCにindexer再開。ログ上のopened scopeはdev/test/generalだけで、status `opened_scopes=3`。
- 起動直後のreadinessは、最初のgeneral再走査が未完了でfreshnessが一時fail。走査完了を待ち、03:17:31 UTCに再実行して全項目pass、`truth=8 projection=8`、activation更新を確認した。
- statusはworker/ingest=true、last_sync/last_ingest更新済み、scanned=11/indexed=8/last_error=null。先行復旧の検証投稿には供給元CLI終了による取得不能があるが、旧4topicの再走査ではない。
- API・relay・Postgres・ArcadeDB・Valkey等のcontainerは維持。停止・再開したのはcn-indexerのみ。
- monitor serviceはResult=success / ExecMainStatus=0。
- Cloud Monitoringでも03:17:31 UTCの期待topic集合=1、索引=8を確認。本文取得失敗2は先行検証のtest/dev投稿で、旧topicの処理ではない。VM転送用一時file6本とIAP tunnelを片付け、保護済みbackupは保持した。

## 独立監査・検証

- helperは署名entry+tombstone入り7namespacesからの限定削除、非対象保持、二度適用、既存output/source上書き拒否、missing source拒否の2 tests PASS。独立担当によるWindows `cargo test --locked`とfmt確認もPASS。
- SQLの初回監査は、参照SELECT後の並行書込みを防げない競合によりFAIL。guard前の関連テーブルlockを追加し、delta監査PASS後に本番実行した。
- Arcade scriptの固定scope/bind parameter/非対象全列比較/credential非出力もPASS。実行前にenv名・database既定を現行configと一致させた2文字列deltaも追加監査PASS。
- 実行SQL SHA256: `9a19cce03d54037107b2d2750d91f8800d6cc1c0bcdc1d8f89533062b5ea4ebd`。Arcade script SHA256: `a3fc3ce3646467146403ada72ef8e38deefe2b3fe9a7267789db96f6f591ab4c`。
- repositoryの追加物は保守補助と本記録。製品の同期・索引・安全性コードは変更していない。後続のユーザー依頼によりPR作成とCI/独立監査成功後のmergeも承認された。最終headとCI/監査結果はPRへ記録する。
