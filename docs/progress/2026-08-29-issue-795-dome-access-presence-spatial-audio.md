# Issue #795 Dome access / block / presence / spatial audio

- signed directional block/unblockをsocial graph、store、desktop IPC、Profile操作へ追加した。
- block時にowner間Connectionを`owners_blocked`で終了し、transition reservationと対象participantを失効する。Unblockでは再接続しない。
- topic subscription/private channel current epochを共通access decisionとし、owner hostとCommunity Node prepare、boundary preview、room event publish/listへ適用した。
- current + 最大4 active/access可能neighborのpresenceを既存ephemeral room eventから購読し、10秒TTLで除去する。
- 明示mic操作による16 kHz mono PCM16送信と、current/connection opening距離によるlocal Web Audio減衰、local muteを追加した。音声は永続化しない。
- resource budgetへaudio frame rate/bandwidth、同時stream、jitter frame上限を追加した。

Validationは`cargo xtask check`、`cargo xtask test`、`cargo xtask cn-check`、`cargo xtask cn-test`、`cargo xtask desktop-ui-check`、`cargo xtask tauri-check`、`cargo xtask ipc-types --check`を基準とする。
