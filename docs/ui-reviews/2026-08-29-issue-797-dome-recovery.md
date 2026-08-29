# Issue #797 Dome recovery UI review

Preview: Storybook `Extended/MetaverseRoomPanel` の `OfflineGraceBoundary`、`DrainingBoundary`、`BlockedBoundary`、`ClosedBoundary`。

## Shneiderman eight golden rules

- Consistency: 既存HUD toolbar、Button、warning/danger token、connection zone barrierを再利用した。
- Shortcuts: host grace期限後はready隣接Domeを自動評価し、Return Homeは常時toolbarから実行できる。
- Feedback: offline残り秒、evacuation target、closed/no-candidateを`aria-live` bannerで表示する。
- Closure: target snapshotでlocal avatarを確認した時点だけsceneを切り替える。
- Error prevention: offline/draining/blocked/closedではbarrierを閉じ、inputと新規transitionをfenceする。
- Reversal: grace内の同一session復帰では再Joinせずcurrent sceneへ戻る。通常退出も従来通り残す。
- User control: 自動退避とは別にReturn Homeと通常退出を選べる。
- Memory load: 候補順序、last visit、safe spawn確認をsystemが扱い、利用者にInstance IDを要求しない。

境界は色だけでなく、offlineのwireframe、draining/blockedの横bar、HUD文言と残り時間で区別する。Return Homeはaccessible nameとtooltipを持ち、keyboard focus順に含まれる。
