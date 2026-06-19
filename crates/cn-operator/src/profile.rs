//! operator profile。profile は features の既定値を与える。
//!
//! 個々の `features` キーが profile の既定値を上書きできる。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::capability::Capability;

/// 初期実装で定義する operator profile。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// community index / moderation / community-local trust（いずれも計画中）。
    /// private message storage / blob cache / analytics / crash report は無効。
    Minimal,
    /// minimal + 専用 iroh relay + 暗号化済み traffic fallback 開示。
    RelayEnabled,
    /// relay-enabled + blob cache + 通知 + 任意の analytics / crash report。
    FullService,
}

impl Profile {
    pub fn key(self) -> &'static str {
        match self {
            Profile::Minimal => "minimal",
            Profile::RelayEnabled => "relay-enabled",
            Profile::FullService => "full-service",
        }
    }

    /// この profile における capability の既定有効値。
    ///
    /// 明示されない capability は false（auth_consent は config 側で baseline 有効）。
    pub fn feature_defaults(self) -> BTreeMap<Capability, bool> {
        let mut map = BTreeMap::new();
        let mut set = |cap: Capability, on: bool| {
            map.insert(cap, on);
        };

        // 全 profile 共通: 接続補助の基盤。
        set(Capability::AuthConsent, true);
        set(Capability::BootstrapAssist, true);
        set(Capability::TopicRendezvous, true);

        match self {
            Profile::Minimal => {
                set(Capability::CommunityIndex, true);
                set(Capability::Moderation, true);
                set(Capability::CommunityLocalTrust, true);
                set(Capability::PrivateMessageStorage, false);
                set(Capability::BlobCache, false);
                set(Capability::Analytics, false);
                set(Capability::CrashReport, false);
            }
            Profile::RelayEnabled => {
                set(Capability::CommunityIndex, true);
                set(Capability::Moderation, true);
                set(Capability::CommunityLocalTrust, true);
                set(Capability::IrohRelay, true);
                set(Capability::TrafficRelayFallback, true);
                set(Capability::PrivateMessageStorage, false);
                set(Capability::BlobCache, false);
                set(Capability::Analytics, false);
                set(Capability::CrashReport, false);
            }
            Profile::FullService => {
                set(Capability::CommunityIndex, true);
                set(Capability::Moderation, true);
                set(Capability::CommunityLocalTrust, true);
                set(Capability::IrohRelay, true);
                set(Capability::TrafficRelayFallback, true);
                set(Capability::BlobCache, true);
                set(Capability::PushNotification, true);
                set(Capability::ReportEndpoint, true);
                // analytics / crash report は任意。既定は無効のまま運用者が選ぶ。
                set(Capability::Analytics, false);
                set(Capability::CrashReport, false);
            }
        }

        map
    }
}
