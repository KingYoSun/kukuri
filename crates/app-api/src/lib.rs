//! デスクトップ向けアプリケーション API(`AppService`)。
//!
//! 配置規約(WP-B10):
//! - トップレベルのドメインファイル(`timeline.rs` / `private_channels.rs` 等)=
//!   IPC(desktop-runtime / src-tauri)から呼ばれる**公開ドメイン API**。
//!   新メソッドはまず該当ドメインファイルへ置く。
//! - `service/` 配下 = pub(crate) の内部ヘルパ(`*_support.rs`)と合成
//!   (`mod.rs` の `ServiceHandles` / `SubscriptionRegistry`)。複数ドメインから
//!   使う内部処理だけをここへ下ろす。
//! - `AppService` へフィールドを足す前に、依存なら `ServiceHandles`、購読タスク
//!   なら `SubscriptionRegistry` への収容を先に検討する(単一型への集中は既知の
//!   負債。2026-07-13 完了レビュー D12)。
//!
//! ※ `private_channels.rs` ↔ `service/private_channels_support.rs` は公開 / 内部の
//! 正当な分割であり、同名を理由に統合しない(REFACTORING.md 地雷リスト)。

mod direct_messages;
mod game;
mod live;
mod media;
mod notifications;
mod private_channels;
mod reactions;
mod service;
mod social;
mod sync;
mod timeline;
mod views;

pub use kukuri_store::NotificationKind;
pub use service::{AppService, ServiceHandles};
pub use views::*;
