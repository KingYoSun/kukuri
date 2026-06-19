use super::*;

/// 分散通報ルーティング（#310）の通報送信リクエスト。
///
/// 通報先は中央窓口ではなく、対象を実際に表示・索引・moderation・cache 等した
/// community node の manifest（#310 で追加した `report_endpoint`）から解決される。
/// client 側（provenance + manifest）で通報先を解決し、その `report_endpoint` を
/// この request に載せて渡す。endpoint が無い node は client 側で `abuse_contact`
/// 案内に切り替えるため、この command には到達しない。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubmitCommunityNodeReportRequest {
    /// 通報先 node の base url（記録・表示用）。
    pub node_base_url: String,
    /// node manifest が公開する通報受付 endpoint（絶対 http(s) URL）。
    pub report_endpoint: String,
    /// 通報対象の種別（post / profile / media / search_result / recommendation 等）。
    pub subject_kind: String,
    /// 通報対象の識別子（post id / pubkey / object id 等）。
    pub subject_id: String,
    /// 通報先となった node capability（community_index / moderation / media_cache 等）。
    pub capability: String,
    /// 通報理由カテゴリ。
    pub reason: String,
    /// 任意の補足説明。
    #[serde(default)]
    pub details: Option<String>,
    /// 任意の通報者連絡先（node が follow-up に使える）。
    #[serde(default)]
    pub reporter_contact: Option<String>,
}

/// 通報送信結果の状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitCommunityNodeReportStatus {
    /// node が通報を受理した（2xx）。
    Submitted,
}

/// 通報送信結果。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitCommunityNodeReportResult {
    pub status: SubmitCommunityNodeReportStatus,
    /// node が返した受付参照 ID（任意）。
    #[serde(default)]
    pub reference_id: Option<String>,
}

/// node の report endpoint へ送る payload。reporter の identity / social graph は
/// node-independent であり、通報のために node へ預けない。明示的に入力された連絡先のみ送る。
#[derive(Serialize)]
struct ReportPayload<'a> {
    subject_kind: &'a str,
    subject_id: &'a str,
    capability: &'a str,
    reason: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reporter_contact: Option<&'a str>,
}

/// node が返し得る受付応答（任意フィールド）。
#[derive(Deserialize, Default)]
struct ReportAck {
    #[serde(default)]
    reference_id: Option<String>,
}

/// report endpoint が POST 可能な http(s) 絶対 URL か検証する。
///
/// 空文字や非 http(s) は拒否する。client は endpoint が無い node を `abuse_contact`
/// 案内へ切り替えるため、ここに到達する endpoint は manifest 由来の絶対 URL である。
pub(crate) fn validate_report_endpoint(endpoint: &str) -> Result<&str> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("report endpoint is empty"));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(anyhow!("report endpoint must be an http(s) URL"));
    }
    Ok(trimmed)
}

impl DesktopRuntime {
    /// 解決済みの通報先 node の report endpoint へ通報を POST する（#310）。
    ///
    /// この経路は中央集約ではなく、対象に実際に関与した node の authority scope 内へ
    /// 通報を届けるためのものである。endpoint が無い node 宛ては呼び出し側で
    /// `abuse_contact` 案内に切り替えるため、この method には渡らない。
    pub(crate) async fn request_community_node_report_submit(
        &self,
        request: &SubmitCommunityNodeReportRequest,
    ) -> Result<SubmitCommunityNodeReportResult> {
        let endpoint = validate_report_endpoint(&request.report_endpoint)?;
        let client = community_node_http_client()?;
        let payload = ReportPayload {
            subject_kind: request.subject_kind.as_str(),
            subject_id: request.subject_id.as_str(),
            capability: request.capability.as_str(),
            reason: request.reason.as_str(),
            details: request.details.as_deref(),
            reporter_contact: request.reporter_contact.as_deref(),
        };
        let response = client
            .post(endpoint)
            .json(&payload)
            .send()
            .await
            .context("failed to submit community node report")?;
        let response = response
            .error_for_status()
            .context("community node report submission failed")?;
        let reference_id = response
            .json::<ReportAck>()
            .await
            .ok()
            .and_then(|ack| ack.reference_id);
        Ok(SubmitCommunityNodeReportResult {
            status: SubmitCommunityNodeReportStatus::Submitted,
            reference_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_http_endpoints() {
        assert!(validate_report_endpoint("https://node.example/v1/report").is_ok());
        assert!(validate_report_endpoint("http://node.example/v1/report").is_ok());
        assert_eq!(
            validate_report_endpoint("  https://node.example/v1/report  ").unwrap(),
            "https://node.example/v1/report"
        );
    }

    #[test]
    fn rejects_empty_or_non_http_endpoints() {
        assert!(validate_report_endpoint("").is_err());
        assert!(validate_report_endpoint("   ").is_err());
        // mailto / file などは POST 経路に乗せない（client が contact 案内へ切り替える）。
        assert!(validate_report_endpoint("mailto:abuse@node.example").is_err());
        assert!(validate_report_endpoint("ftp://node.example/report").is_err());
    }

    #[test]
    fn report_payload_omits_optional_fields_when_absent() {
        let payload = ReportPayload {
            subject_kind: "post",
            subject_id: "abc",
            capability: "community_index",
            reason: "spam",
            details: None,
            reporter_contact: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["subject_kind"], "post");
        assert_eq!(json["capability"], "community_index");
        assert!(json.get("details").is_none());
        assert!(json.get("reporter_contact").is_none());
    }

    #[test]
    fn request_deserializes_from_snake_case() {
        let request: SubmitCommunityNodeReportRequest = serde_json::from_str(
            r#"{
                "node_base_url": "https://node.example",
                "report_endpoint": "https://node.example/v1/report",
                "subject_kind": "post",
                "subject_id": "abc",
                "capability": "community_index",
                "reason": "spam"
            }"#,
        )
        .unwrap();
        assert_eq!(request.node_base_url, "https://node.example");
        assert_eq!(request.capability, "community_index");
        assert!(request.details.is_none());
    }

    #[test]
    fn result_serializes_status_snake_case() {
        let result = SubmitCommunityNodeReportResult {
            status: SubmitCommunityNodeReportStatus::Submitted,
            reference_id: Some("ref-1".to_string()),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["status"], "submitted");
        assert_eq!(json["reference_id"], "ref-1");
    }
}
