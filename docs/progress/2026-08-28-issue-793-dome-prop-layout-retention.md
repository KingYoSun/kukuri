# Issue #793 Dome prop/layout retention 実装記録

## 完了した範囲

- world schemaを5へ更新し、preset/instance/leaseに単調増加するmanifest revisionを追加した。preset revisionはappend-onlyで保持し、古い参照からもexact manifestを取得できる。
- shared host runtimeにowner-only persistent prop mutation、participant guest prop、wall-clock TTL、persistent-only layout candidate、10 Hz・最大100件のmemory-only snapshot ringとresyncを追加した。
- ownerの明示操作によるlayout commitを追加した。host署名候補とowner署名commit、operation IDによる冪等性、正規化no-op、30秒rate limit、同一host targetでの新epoch/session再始動を実装した。
- desktopとCommunity Nodeにmanifest/asset専用cacheを追加した。content hash dedupe、current/active/staging/直近3 rollback pin、24時間grace、desktop 1 GiB/CN 10 GiB上限を適用した。
- CN assignmentでmanifestと全assetのhashを検証してstageし、active化後にpinを切り替える。layout candidate/resync endpointとdesktop IPC/UIを接続した。
- UIでpersistent/guest propを区別し、owner用persistent mutation/layout保存、参加者用guest spawn、snapshot resync、manifest revision/cache上限を表示する。

## 後続Issueとの境界

容量の設定可能化、metrics、段階的degrade、悪意あるassetの包括的制限は#794、Dome間prop transition/prefetch/持ち込み規則は#790、access policyは#795が本contractを利用して実装する。
