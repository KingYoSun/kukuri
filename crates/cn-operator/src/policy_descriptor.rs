//! capability が法務文書へ与える構造化された事実。
//!
//! prose は renderer の責務とし、データ分類・処理・利用条件・保持参照・外部送信・
//! 請求経路・safety action・効果範囲はこの descriptor を単一の入力元にする。

use serde::Serialize;

use crate::{Capability, ExternalDestination};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDataClass {
    AuthenticationAndConsent,
    ConnectivityMetadata,
    TopicPresence,
    EncryptedTransitTraffic,
    CachedAttachment,
    EncryptedPrivateMessage,
    UsageAnalytics,
    CrashDiagnostics,
    HttpTraffic,
    PushDelivery,
    PublicContentAndMetadata,
    SafetyAssessment,
    CommunityRelation,
    UserReport,
    RightsClaim,
    TesterFeedback,
    DomeSession,
}

impl PolicyDataClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AuthenticationAndConsent => "公開鍵・認証情報・同意記録",
            Self::ConnectivityMetadata => "接続先・接続元・到達性メタデータ",
            Self::TopicPresence => "topic presence と一時的な接続ヒント",
            Self::EncryptedTransitTraffic => "暗号化済みの中継 traffic",
            Self::CachedAttachment => "一時キャッシュされた添付メディアと CID",
            Self::EncryptedPrivateMessage => "暗号化されたプライベートメッセージ",
            Self::UsageAnalytics => "利用状況の統計・イベント",
            Self::CrashDiagnostics => "クラッシュ診断・スタックトレース",
            Self::HttpTraffic => "HTTP request/response と接続元 IP",
            Self::PushDelivery => "device token と通知 payload",
            Self::PublicContentAndMetadata => "公開 topic の本文・metadata・再構築可能な索引",
            Self::SafetyAssessment => "scan verdict・moderation event・risk signal・異議状態",
            Self::CommunityRelation => "公開 topic 共参加由来の関係・node-local trust",
            Self::UserReport => "通報対象・理由・補足・任意の連絡先",
            Self::RightsClaim => "申出人情報・権利根拠・対象・証拠参照",
            Self::TesterFeedback => "自由記述 feedback・client version・OS",
            Self::DomeSession => "Hosting Lease・manifest・input・ephemeral state",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyProcessing {
    Authenticate,
    MatchPeers,
    Relay,
    Cache,
    TemporarilyStore,
    Analyze,
    DeliverNotification,
    IndexAndRecommend,
    SafetyScanAndModerate,
    ComputeAdvisory,
    IntakeAndReview,
    HostSession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyUsageCondition {
    ExplicitNodeConsent,
    ConnectionAttempt,
    EnabledFeature,
    SupportedPublicTopic,
    UserSubmission,
    OwnerSignedLease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRetentionRef {
    ShortTtl,
    UntilConsentWithdrawal,
    ConnectionLogsDays,
    TemporaryCache,
    ProviderPolicy,
    UntilUnregistered,
    UntilDelivered,
    IndexEligibility,
    ModerationLogsDays,
    ModerationEventDays,
    RiskSignalDays,
    ReportDays,
    RightsRequestLifecycle,
    TesterFeedbackDays,
    LeaseOrSessionEnd,
}

impl PolicyRetentionRef {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ShortTtl => "短期 TTL で失効",
            Self::UntilConsentWithdrawal => {
                "同意撤回まで（公開済み文書・同意履歴は監査履歴として保持）"
            }
            Self::ConnectionLogsDays => "retention.connection_logs_days",
            Self::TemporaryCache => "一時キャッシュ（恒久保存なし）",
            Self::ProviderPolicy => "operator が契約する外部 provider の保持方針",
            Self::UntilUnregistered => "登録解除まで",
            Self::UntilDelivered => "配送完了まで",
            Self::IndexEligibility => "対象 topic・許可判定の条件を満たす間",
            Self::ModerationLogsDays => "retention.moderation_logs_days",
            Self::ModerationEventDays => "retention.moderation_event_days",
            Self::RiskSignalDays => "retention.risk_signal_days または明示された失効まで",
            Self::ReportDays => "retention.report_days / report_contact_days",
            Self::RightsRequestLifecycle => "retention.rights_request_*_days",
            Self::TesterFeedbackDays => "retention.tester_feedback_days",
            Self::LeaseOrSessionEnd => "lease close・期限または session 終了まで",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyBillingPath {
    None,
    OperatorProviderContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySafetyAction {
    None,
    FailClosedHold,
    ExcludeFromNodeIndex,
    SignedNodeLocalEvent,
    Appeal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffectScope {
    ThisNode,
    ConnectionOnly,
    SupportedPublicTopics,
    NodeLocalAdvisory,
    SubmittedRequest,
    OwnerSignedLease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CapabilityPolicyDescriptor {
    pub data_classes: &'static [PolicyDataClass],
    pub processing: &'static [PolicyProcessing],
    pub usage_condition: PolicyUsageCondition,
    pub retention: &'static [PolicyRetentionRef],
    pub external_destinations: &'static [ExternalDestination],
    pub billing_path: PolicyBillingPath,
    pub safety_actions: &'static [PolicySafetyAction],
    pub effect_scope: PolicyEffectScope,
}

impl CapabilityPolicyDescriptor {
    pub fn data_classes_text(self) -> String {
        self.data_classes
            .iter()
            .map(|value| value.label())
            .collect::<Vec<_>>()
            .join("、")
    }

    pub fn retention_text(self) -> String {
        self.retention
            .iter()
            .map(|value| value.label())
            .collect::<Vec<_>>()
            .join("、")
    }
}

impl Capability {
    pub fn policy_descriptor(self) -> CapabilityPolicyDescriptor {
        use PolicyBillingPath::{None as NoBilling, OperatorProviderContract};
        use PolicyDataClass::*;
        use PolicyEffectScope::*;
        use PolicyProcessing::*;
        use PolicyRetentionRef::*;
        use PolicySafetyAction::*;
        use PolicyUsageCondition::*;

        let common = |data_classes, processing, usage_condition, retention, effect_scope| {
            CapabilityPolicyDescriptor {
                data_classes,
                processing,
                usage_condition,
                retention,
                external_destinations: &[],
                billing_path: NoBilling,
                safety_actions: &[PolicySafetyAction::None],
                effect_scope,
            }
        };
        match self {
            Self::AuthConsent => common(
                &[AuthenticationAndConsent],
                &[Authenticate],
                ExplicitNodeConsent,
                &[ShortTtl, UntilConsentWithdrawal],
                ThisNode,
            ),
            Self::BootstrapAssist => common(
                &[ConnectivityMetadata],
                &[MatchPeers],
                ConnectionAttempt,
                &[ShortTtl],
                ConnectionOnly,
            ),
            Self::TopicRendezvous => common(
                &[TopicPresence],
                &[MatchPeers],
                ConnectionAttempt,
                &[ShortTtl],
                ConnectionOnly,
            ),
            Self::IrohRelay => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::DedicatedIrohRelay],
                billing_path: OperatorProviderContract,
                ..common(
                    &[ConnectivityMetadata, EncryptedTransitTraffic],
                    &[Relay],
                    ConnectionAttempt,
                    &[ConnectionLogsDays],
                    ConnectionOnly,
                )
            },
            Self::TrafficRelayFallback => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::PublicRelay],
                billing_path: OperatorProviderContract,
                ..common(
                    &[ConnectivityMetadata, EncryptedTransitTraffic],
                    &[Relay],
                    ConnectionAttempt,
                    &[ConnectionLogsDays],
                    ConnectionOnly,
                )
            },
            Self::BlobCache => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::ObjectStorage],
                billing_path: OperatorProviderContract,
                ..common(
                    &[CachedAttachment],
                    &[Cache],
                    EnabledFeature,
                    &[TemporaryCache],
                    ThisNode,
                )
            },
            Self::PrivateMessageStorage => common(
                &[EncryptedPrivateMessage],
                &[TemporarilyStore],
                EnabledFeature,
                &[TemporaryCache],
                ThisNode,
            ),
            Self::Analytics => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::AnalyticsProvider],
                billing_path: OperatorProviderContract,
                ..common(
                    &[UsageAnalytics],
                    &[Analyze],
                    EnabledFeature,
                    &[ProviderPolicy],
                    ThisNode,
                )
            },
            Self::CrashReport => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::CrashReportProvider],
                billing_path: OperatorProviderContract,
                ..common(
                    &[CrashDiagnostics],
                    &[Analyze],
                    EnabledFeature,
                    &[ProviderPolicy],
                    ThisNode,
                )
            },
            Self::CloudflareProxy => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::Cloudflare],
                billing_path: OperatorProviderContract,
                ..common(
                    &[HttpTraffic],
                    &[Relay, Cache],
                    ConnectionAttempt,
                    &[ProviderPolicy],
                    ConnectionOnly,
                )
            },
            Self::PushNotification => CapabilityPolicyDescriptor {
                external_destinations: &[ExternalDestination::PushProvider],
                billing_path: OperatorProviderContract,
                ..common(
                    &[PushDelivery],
                    &[DeliverNotification],
                    EnabledFeature,
                    &[UntilUnregistered, UntilDelivered],
                    ThisNode,
                )
            },
            Self::CommunityIndex => common(
                &[PublicContentAndMetadata],
                &[IndexAndRecommend],
                SupportedPublicTopic,
                &[IndexEligibility],
                SupportedPublicTopics,
            ),
            Self::Moderation => CapabilityPolicyDescriptor {
                safety_actions: &[
                    FailClosedHold,
                    ExcludeFromNodeIndex,
                    SignedNodeLocalEvent,
                    Appeal,
                ],
                ..common(
                    &[SafetyAssessment],
                    &[SafetyScanAndModerate],
                    SupportedPublicTopic,
                    &[ModerationLogsDays, ModerationEventDays, RiskSignalDays],
                    SupportedPublicTopics,
                )
            },
            Self::CommunityLocalTrust => CapabilityPolicyDescriptor {
                safety_actions: &[Appeal],
                ..common(
                    &[CommunityRelation, SafetyAssessment],
                    &[ComputeAdvisory],
                    SupportedPublicTopic,
                    &[RiskSignalDays, IndexEligibility],
                    NodeLocalAdvisory,
                )
            },
            Self::ReportEndpoint => common(
                &[UserReport],
                &[IntakeAndReview],
                UserSubmission,
                &[ReportDays],
                SubmittedRequest,
            ),
            Self::RightsRequestEndpoint => common(
                &[RightsClaim],
                &[IntakeAndReview],
                UserSubmission,
                &[RightsRequestLifecycle],
                SubmittedRequest,
            ),
            Self::TesterFeedback => common(
                &[TesterFeedback],
                &[IntakeAndReview],
                UserSubmission,
                &[TesterFeedbackDays],
                SubmittedRequest,
            ),
            Self::DomeHosting => common(
                &[DomeSession],
                &[HostSession],
                PolicyUsageCondition::OwnerSignedLease,
                &[LeaseOrSessionEnd],
                PolicyEffectScope::OwnerSignedLease,
            ),
        }
    }
}
