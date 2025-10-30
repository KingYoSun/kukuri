use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Nostr イベント検証やアプリケーションレベルのバリデーション失敗理由。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ValidationFailureKind {
    /// 汎用的なバリデーションエラー。
    Generic,
    /// NIP-01 の基本整合性（ID 再計算・署名・タイムスタンプなど）違反。
    Nip01Integrity,
    /// NIP-10 のタグ構造（marker・relay URL 等）違反。
    Nip10TagStructure,
    /// NIP-19 の bech32 エンコードが不正な場合。
    Nip19Encoding,
    /// NIP-19 の TLV セクションが仕様外な場合。
    Nip19Tlv,
    /// サポート外の `kind` が指定された場合。
    UnsupportedKind,
    /// kind:30078 の必須タグ欠如や識別子不正。
    Kind30078TagMissing,
    /// kind:30078 のタグ値に不一致がある場合。
    Kind30078TagMismatch,
    /// kind:30078 の content スキーマが仕様外。
    Kind30078ContentSchema,
    /// kind:30078 の content サイズが許容範囲を超えた場合。
    Kind30078ContentSize,
    /// content のサイズが制限を超過。
    ContentTooLarge,
    /// tags の件数が制限を超過、またはタグ内容が UTF-8 ではない。
    TagLimitExceeded,
    /// 非 UTF-8 文字列が含まれている場合。
    Utf8Invalid,
    /// 許容できないタイムスタンプやプレシデンス違反。
    TimestampOutOfRange,
    /// PRE（Parameterized Replaceable Event）の古いリビジョンを拒否した場合。
    PrecedenceRejected,
}

impl ValidationFailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationFailureKind::Generic => "generic",
            ValidationFailureKind::Nip01Integrity => "nip01_integrity",
            ValidationFailureKind::Nip10TagStructure => "nip10_tag_structure",
            ValidationFailureKind::Nip19Encoding => "nip19_encoding",
            ValidationFailureKind::Nip19Tlv => "nip19_tlv",
            ValidationFailureKind::UnsupportedKind => "unsupported_kind",
            ValidationFailureKind::Kind30078TagMissing => "kind30078_tag_missing",
            ValidationFailureKind::Kind30078TagMismatch => "kind30078_tag_mismatch",
            ValidationFailureKind::Kind30078ContentSchema => "kind30078_content_schema",
            ValidationFailureKind::Kind30078ContentSize => "kind30078_content_size",
            ValidationFailureKind::ContentTooLarge => "content_too_large",
            ValidationFailureKind::TagLimitExceeded => "tag_limit_exceeded",
            ValidationFailureKind::Utf8Invalid => "utf8_invalid",
            ValidationFailureKind::TimestampOutOfRange => "timestamp_out_of_range",
            ValidationFailureKind::PrecedenceRejected => "precedence_rejected",
        }
    }
}

impl fmt::Display for ValidationFailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for ValidationFailureKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "generic" => Ok(ValidationFailureKind::Generic),
            "nip01_integrity" => Ok(ValidationFailureKind::Nip01Integrity),
            "nip10_tag_structure" => Ok(ValidationFailureKind::Nip10TagStructure),
            "nip19_encoding" => Ok(ValidationFailureKind::Nip19Encoding),
            "nip19_tlv" => Ok(ValidationFailureKind::Nip19Tlv),
            "unsupported_kind" => Ok(ValidationFailureKind::UnsupportedKind),
            "kind30078_tag_missing" => Ok(ValidationFailureKind::Kind30078TagMissing),
            "kind30078_tag_mismatch" => Ok(ValidationFailureKind::Kind30078TagMismatch),
            "kind30078_content_schema" => Ok(ValidationFailureKind::Kind30078ContentSchema),
            "kind30078_content_size" => Ok(ValidationFailureKind::Kind30078ContentSize),
            "content_too_large" => Ok(ValidationFailureKind::ContentTooLarge),
            "tag_limit_exceeded" => Ok(ValidationFailureKind::TagLimitExceeded),
            "utf8_invalid" => Ok(ValidationFailureKind::Utf8Invalid),
            "timestamp_out_of_range" => Ok(ValidationFailureKind::TimestampOutOfRange),
            "precedence_rejected" => Ok(ValidationFailureKind::PrecedenceRejected),
            _ => Err(()),
        }
    }
}
