//! 正規化・封筒検証は kukuri-cn-protocol へ移動した(WP-H3)。
//! サーバ側の import(`kukuri_cn_core::normalize` / `crate::normalize`)互換のための re-export。

pub use kukuri_cn_protocol::normalize::*;
