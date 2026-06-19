//! community node の capability モデル。
//!
//! #352 の operator docs generator は、各機能を boolean トグルとしてだけでなく、
//! 説明責任・外部送信・保持影響を持つ capability として扱う。
//!
//! ここで重要なのは `Availability` による Phase A / Phase B の分離である。
//!
//! - `Availability::Available` (Phase A): 現行の community node 実装が実際に提供できる、
//!   またはデプロイ構成として確定できる capability。生成文書では「運用中」として開示してよい。
//! - `Availability::Planned` (Phase B): 現行実装に存在しない capability（index / moderation /
//!   trust signal / report endpoint など）。config 上は宣言できるが、生成文書では
//!   「計画中・この配布物では未提供」として扱い、運用中の外部送信・データ取扱い開示には載せない。
//!
//! これは `docs/architecture/p2p-first-community-node-responsibility-boundary.md` の
//! 「node manifest が宣言した capability / authority scope が責任範囲の上限」という方針に従い、
//! 「宣言」と「実際に実行可能」を分離して、実体のない開示を生成しないためのガードである。

use std::fmt;

/// capability が現行配布物で実行可能か（Phase A）、設計のみ（Phase B）か。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// Phase A: 現行実装またはデプロイ構成として提供可能。運用中として開示してよい。
    Available,
    /// Phase B: 設計・spec のみ。生成文書では「計画中・未提供」として扱う。
    Planned,
}

impl Availability {
    pub fn is_planned(self) -> bool {
        matches!(self, Availability::Planned)
    }

    /// 文書中の日本語ラベル。
    pub fn label_ja(self) -> &'static str {
        match self {
            Availability::Available => "提供中",
            Availability::Planned => "計画中（この配布物では未提供）",
        }
    }
}

/// community node が提供し得る capability。
///
/// `ALL` の並び順が生成文書・manifest の決定論的な出力順序になる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    // --- Phase A: 現行実装 / デプロイ構成 ---
    AuthConsent,
    BootstrapAssist,
    TopicRendezvous,
    IrohRelay,
    TrafficRelayFallback,
    BlobCache,
    PrivateMessageStorage,
    Analytics,
    CrashReport,
    CloudflareProxy,
    PushNotification,
    // --- Phase B: 未実装（spec のみ） ---
    CommunityIndex,
    Moderation,
    CommunityLocalTrust,
    ReportEndpoint,
}

impl Capability {
    /// 決定論的な出力順序を与える全 capability。
    pub const ALL: [Capability; 15] = [
        Capability::AuthConsent,
        Capability::BootstrapAssist,
        Capability::TopicRendezvous,
        Capability::IrohRelay,
        Capability::TrafficRelayFallback,
        Capability::BlobCache,
        Capability::PrivateMessageStorage,
        Capability::Analytics,
        Capability::CrashReport,
        Capability::CloudflareProxy,
        Capability::PushNotification,
        Capability::CommunityIndex,
        Capability::Moderation,
        Capability::CommunityLocalTrust,
        Capability::ReportEndpoint,
    ];

    /// config / manifest の snake_case キー。
    pub fn key(self) -> &'static str {
        match self {
            Capability::AuthConsent => "auth_consent",
            Capability::BootstrapAssist => "bootstrap_assist",
            Capability::TopicRendezvous => "topic_rendezvous",
            Capability::IrohRelay => "iroh_relay",
            Capability::TrafficRelayFallback => "traffic_relay_fallback",
            Capability::BlobCache => "blob_cache",
            Capability::PrivateMessageStorage => "private_message_storage",
            Capability::Analytics => "analytics",
            Capability::CrashReport => "crash_report",
            Capability::CloudflareProxy => "cloudflare_proxy",
            Capability::PushNotification => "push_notification",
            Capability::CommunityIndex => "community_index",
            Capability::Moderation => "moderation",
            Capability::CommunityLocalTrust => "community_local_trust",
            Capability::ReportEndpoint => "report_endpoint",
        }
    }

    pub fn availability(self) -> Availability {
        match self {
            // index / moderation / local trust は未実装（spec のみ）。
            Capability::CommunityIndex
            | Capability::Moderation
            | Capability::CommunityLocalTrust => Availability::Planned,
            // report endpoint は #370 で実装済み（POST /v1/report・保存・運営者確認導線）。
            _ => Availability::Available,
        }
    }

    pub fn display_name(self) -> &'static str {
        self.meta().display_name
    }

    /// 文書生成用の静的メタデータ。
    pub fn meta(self) -> CapabilityMeta {
        match self {
            Capability::AuthConsent => CapabilityMeta {
                capability: self,
                display_name: "認証・同意 (auth / consent)",
                handled_data: "ユーザー公開鍵、署名付き認証エンベロープ、同意ステータス、JWT 発行記録",
                purpose: "community node の補助機能を利用する client の認証と、利用規約・ポリシーへの同意取得",
                retention_impact: "認証チャレンジは短期 TTL、同意レコードは撤回まで保持",
                external_transmission: None,
                telecom_note: "認証はノード自身が処理する。回線設備の設置を伴わない。",
                privacy_note: "公開鍵と同意状態を扱う。IP アドレス・接続ログの扱いは接続ログ保持期間に従う。",
                terms_note: "本ノードの補助機能利用には認証と同意が必要である旨を記載する。",
            },
            Capability::BootstrapAssist => CapabilityMeta {
                capability: self,
                display_name: "ブートストラップ補助 (bootstrap assist)",
                handled_data: "ピアの接続ヒント（node id / addr hint）、一時的な登録エントリ",
                purpose: "新規 client が P2P network へ最初に到達するための seed peer 情報の提供",
                retention_impact: "ピア登録は短期 TTL で失効する ephemeral state",
                external_transmission: None,
                telecom_note: "接続ヒントの中継のみ。実データの伝送経路を恒久的に保持しない。",
                privacy_note: "一時的な接続情報を扱う。長期保存はしない。",
                terms_note: "P2P 接続を補助する目的であり、通信内容を保持しない旨を記載する。",
            },
            Capability::TopicRendezvous => CapabilityMeta {
                capability: self,
                display_name: "トピックランデブー (topic rendezvous)",
                handled_data: "topic ごとの presence 情報（node id / 接続ヒント）、TTL 付き ephemeral state",
                purpose: "同一 topic を購読する client 同士の Relay Supported P2P 接続成立を補助する",
                retention_impact: "presence は KV 上の TTL 付き ephemeral state で短期失効する",
                external_transmission: None,
                telecom_note: "presence の一時的な突き合わせのみ。実データ伝送の恒久経路を持たない。",
                privacy_note: "どの topic に接続中かの一時情報を扱う。長期保存はしない。",
                terms_note: "topic 接続の補助であり、投稿内容を保持しない旨を記載する。",
            },
            Capability::IrohRelay => CapabilityMeta {
                capability: self,
                display_name: "iroh relay 補助 (iroh relay assist)",
                handled_data: "NAT traversal / hole punching のためのシグナリング、暗号化済みパケットの中継",
                purpose: "Direct P2P が成立しない場合の hole punching / endpoint assist",
                retention_impact: "中継は経路上の一時処理であり、内容を恒久保存しない",
                external_transmission: Some(ExternalDestination::DedicatedIrohRelay),
                telecom_note: "iroh relay は単なる signaling ではなく、NAT 越えのために暗号化済み traffic の中継が発生し得る。届出要否は構成と所在地に依存するため事前確認が必要。",
                privacy_note: "中継時に接続元の IP アドレスを観測し得る。接続ログ保持期間に従って扱う。",
                terms_note: "relay 経由の接続補助が行われ得る旨を記載する。",
            },
            Capability::TrafficRelayFallback => CapabilityMeta {
                capability: self,
                display_name: "トラフィック relay フォールバック (traffic relay fallback)",
                handled_data: "Direct P2P / Relay Supported P2P が成立しない場合の暗号化済み実データ通信",
                purpose: "他のすべての経路が成立しない場合に限り、暗号化済み traffic を relay 経由で疎通させる",
                retention_impact: "中継のみで内容は恒久保存しない。接続メタデータは接続ログ保持期間に従う",
                external_transmission: Some(ExternalDestination::PublicRelay),
                telecom_note: "暗号化済みであっても traffic relay fallback は実データの伝送経路となり得る。signaling only と混同せず、届出要否を事前確認する。",
                privacy_note: "fallback 時に接続元 IP アドレスやタイミングメタデータを観測し得る。",
                terms_note: "最終手段として暗号化済み通信が relay 経由になり得る旨を記載する。",
            },
            Capability::BlobCache => CapabilityMeta {
                capability: self,
                display_name: "blob / 添付キャッシュ (blob cache)",
                handled_data: "添付メディアの一時キャッシュ、blob の content address (CID)",
                purpose: "添付メディアの配信補助・可用性向上のための一時キャッシュ",
                retention_impact: "キャッシュは一時保持であり、community node は blob 本体を恒久保存しない方針",
                external_transmission: Some(ExternalDestination::ObjectStorage),
                telecom_note: "メディア配信補助。恒久保存を行わない方針を明記する。",
                privacy_note: "添付メディアを一時的に扱う。保持期間と削除方針を明記する。",
                terms_note: "添付メディアが一時的にキャッシュされ得る旨を記載する。",
            },
            Capability::PrivateMessageStorage => CapabilityMeta {
                capability: self,
                display_name: "プライベートメッセージ保管 (private message storage)",
                handled_data: "暗号化されたプライベートメッセージの保管データ",
                purpose: "オフライン配送のためのプライベートメッセージの一時保管",
                retention_impact: "保管期間はノードの設定に従う。既定では無効。",
                external_transmission: None,
                telecom_note: "メッセージ保管はノード内処理。回線設備の設置を伴わない。",
                privacy_note: "プライベートメッセージを扱うため、保持期間・暗号化・アクセス制御を明記する。",
                terms_note: "プライベートメッセージが一時保管され得る旨と暗号化方針を記載する。",
            },
            Capability::Analytics => CapabilityMeta {
                capability: self,
                display_name: "アナリティクス (analytics)",
                handled_data: "利用状況の統計データ、イベントログ",
                purpose: "サービス改善のための利用状況分析",
                retention_impact: "分析プロバイダのポリシーに従う。既定では無効。",
                external_transmission: Some(ExternalDestination::AnalyticsProvider),
                telecom_note: "分析目的のデータ送信が発生し得る。",
                privacy_note: "利用状況データを第三者の分析プロバイダへ送信し得る旨を明記する。",
                terms_note: "アナリティクス目的のデータ収集が行われ得る旨を記載する。",
            },
            Capability::CrashReport => CapabilityMeta {
                capability: self,
                display_name: "クラッシュレポート (crash reporting)",
                handled_data: "クラッシュ時の診断データ、スタックトレース",
                purpose: "不具合の検出と修正のためのクラッシュ情報収集",
                retention_impact: "クラッシュレポートプロバイダのポリシーに従う。既定では無効。",
                external_transmission: Some(ExternalDestination::CrashReportProvider),
                telecom_note: "診断目的のデータ送信が発生し得る。",
                privacy_note: "クラッシュ診断データを第三者プロバイダへ送信し得る旨を明記する。",
                terms_note: "クラッシュレポートが送信され得る旨を記載する。",
            },
            Capability::CloudflareProxy => CapabilityMeta {
                capability: self,
                display_name: "Cloudflare プロキシ / CDN / WAF",
                handled_data: "HTTP リクエスト・レスポンス、接続元 IP アドレス（Cloudflare 経由）",
                purpose: "リバースプロキシ・CDN・WAF による配信補助と保護",
                retention_impact: "Cloudflare のログ・キャッシュポリシーに従う",
                external_transmission: Some(ExternalDestination::Cloudflare),
                telecom_note: "通信が Cloudflare を経由する。所在地・データ越境の観点を確認する。",
                privacy_note: "リクエストと接続元 IP が Cloudflare を経由する旨を外部送信として明記する。",
                terms_note: "通信が Cloudflare を経由し得る旨を記載する。",
            },
            Capability::PushNotification => CapabilityMeta {
                capability: self,
                display_name: "プッシュ通知 (push notification)",
                handled_data: "デバイストークン、通知ペイロード",
                purpose: "OS プッシュ通知の配信",
                retention_impact: "デバイストークンは登録解除まで保持。通知ペイロードは配送後保持しない方針",
                external_transmission: Some(ExternalDestination::PushProvider),
                telecom_note: "プッシュ通知プロバイダ経由の送信が発生する。",
                privacy_note: "デバイストークンと通知内容をプッシュプロバイダへ送信する旨を明記する。",
                terms_note: "プッシュ通知のためにデバイストークンを扱う旨を記載する。",
            },
            Capability::CommunityIndex => CapabilityMeta {
                capability: self,
                display_name: "コミュニティインデックス (community index)",
                handled_data: "（計画）index 対象 content のメタデータ、検索インデックス",
                purpose: "（計画）community node が関与した content の検索・発見の補助",
                retention_impact: "（計画）index 保持方針はノード設定に従う",
                external_transmission: None,
                telecom_note: "（計画）index はノードの authority scope 内に限定される。",
                privacy_note: "（計画）index 対象と保持方針を実装時に明記する。",
                terms_note: "（計画）本ノードが index した content の範囲についてのみ責任を負う旨を記載する。",
            },
            Capability::Moderation => CapabilityMeta {
                capability: self,
                display_name: "モデレーション (moderation)",
                handled_data: "（計画）moderation verdict、署名付き moderation event",
                purpose: "（計画）ノードの index / discovery / recommendation からの critical safety risk の排除",
                retention_impact: "（計画）moderation ログ保持期間に従う",
                external_transmission: None,
                telecom_note: "（計画）moderation はノードの authority scope 内に限定される。",
                privacy_note: "（計画）moderation 対象データの扱いを実装時に明記する。",
                terms_note: "（計画）moderation event は本ノードの authority scope 内でのみ意味を持つ旨を記載する。",
            },
            Capability::CommunityLocalTrust => CapabilityMeta {
                capability: self,
                display_name: "コミュニティローカル trust signal (community-local trust)",
                handled_data: "（計画）根拠付き risk signal / trust signal",
                purpose: "（計画）ノードの authority scope 内での trust / relation signal の発行",
                retention_impact: "（計画）signal の失効ポリシーに従う",
                external_transmission: None,
                telecom_note: "（計画）trust signal はノードの authority scope 内に限定される。",
                privacy_note: "（計画）signal 対象データの扱いを実装時に明記する。",
                terms_note: "（計画）trust signal は network-wide command ではなく optional trust input である旨を記載する。",
            },
            Capability::ReportEndpoint => CapabilityMeta {
                capability: self,
                display_name: "通報エンドポイント (report endpoint)",
                handled_data: "通報内容（対象・理由・任意の補足）、通報者の連絡先（任意）",
                purpose: "本ノードが関与した対象に対する通報の受付（POST /v1/report）",
                retention_impact: "通報データは通報保持ポリシーに従って保持される",
                external_transmission: None,
                telecom_note: "通報受付はノードの authority scope 内に限定される。",
                privacy_note: "reporter の identity / social graph は保持せず、明示入力された連絡先のみ任意保存する。",
                terms_note: "通報は本ノードが関与した対象に限定され、中央通報窓口ではない。",
            },
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

/// 文書生成に使う capability の静的メタデータ。
#[derive(Clone, Copy, Debug)]
pub struct CapabilityMeta {
    pub capability: Capability,
    pub display_name: &'static str,
    pub handled_data: &'static str,
    pub purpose: &'static str,
    pub retention_impact: &'static str,
    pub external_transmission: Option<ExternalDestination>,
    pub telecom_note: &'static str,
    pub privacy_note: &'static str,
    pub terms_note: &'static str,
}

/// 外部送信表示で列挙し得る送信先。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExternalDestination {
    CommunityServer,
    DedicatedIrohRelay,
    PublicRelay,
    Cloudflare,
    ObjectStorage,
    PushProvider,
    AnalyticsProvider,
    CrashReportProvider,
}

impl ExternalDestination {
    pub fn display_name(self) -> &'static str {
        match self {
            ExternalDestination::CommunityServer => "コミュニティサーバー本体",
            ExternalDestination::DedicatedIrohRelay => "専用 iroh relay",
            ExternalDestination::PublicRelay => "n0.computer 等のパブリック relay",
            ExternalDestination::Cloudflare => "Cloudflare (プロキシ / CDN / WAF)",
            ExternalDestination::ObjectStorage => "オブジェクトストレージ",
            ExternalDestination::PushProvider => "プッシュ通知プロバイダ",
            ExternalDestination::AnalyticsProvider => "アナリティクスプロバイダ",
            ExternalDestination::CrashReportProvider => "クラッシュレポートプロバイダ",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ExternalDestination::CommunityServer => {
                "client からの接続を受け、補助機能を提供する本ノード。"
            }
            ExternalDestination::DedicatedIrohRelay => {
                "NAT traversal / hole punching を補助する専用 relay。暗号化済み traffic の中継が発生し得る。"
            }
            ExternalDestination::PublicRelay => {
                "他経路が成立しない場合の fallback として、暗号化済み traffic が経由し得るパブリック relay。"
            }
            ExternalDestination::Cloudflare => {
                "リバースプロキシ / CDN / WAF。HTTP リクエストと接続元 IP が経由する。"
            }
            ExternalDestination::ObjectStorage => "添付メディアの一時キャッシュ配信先。",
            ExternalDestination::PushProvider => "デバイストークンと通知内容の送信先。",
            ExternalDestination::AnalyticsProvider => "利用状況データの送信先。",
            ExternalDestination::CrashReportProvider => "クラッシュ診断データの送信先。",
        }
    }
}
