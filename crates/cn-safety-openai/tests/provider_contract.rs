use async_trait::async_trait;
use kukuri_cn_safety::{
    FetchedMedia, MediaFetcher, ProviderScanRequest, SafetyCategory, SafetyPolicy, SafetyProvider,
    ScanError, ScanInputKind, SubjectKind,
    provider::{VideoFrameExtractor, VideoScanFrames},
    route,
};
use kukuri_cn_safety_openai::{
    API_KEY_ENV, MemoryModerationBudget, ModerationAssessment, ModerationClient, ModerationConfig,
    ModerationCredentials, OpenAiModerationProvider,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

fn response(image: bool, flagged: &[(&str, f64)]) -> serde_json::Value {
    let categories = [
        ("harassment", false),
        ("harassment/threatening", false),
        ("hate", false),
        ("hate/threatening", false),
        ("illicit", false),
        ("illicit/violent", false),
        ("self-harm", true),
        ("self-harm/intent", true),
        ("self-harm/instructions", true),
        ("sexual", true),
        ("sexual/minors", false),
        ("violence", true),
        ("violence/graphic", true),
    ];
    let mut flags = serde_json::Map::new();
    let mut scores = serde_json::Map::new();
    let mut applied = serde_json::Map::new();
    for (name, supported) in categories {
        let score = flagged
            .iter()
            .find(|(category, _)| *category == name)
            .map(|(_, score)| *score);
        flags.insert(name.into(), serde_json::json!(score.is_some()));
        scores.insert(name.into(), serde_json::json!(score.unwrap_or(0.0)));
        applied.insert(
            name.into(),
            if image && !supported {
                serde_json::json!([])
            } else {
                serde_json::json!([if image { "image" } else { "text" }])
            },
        );
    }
    serde_json::json!({"model":"omni-moderation-latest","results":[{
        "flagged":!flagged.is_empty(),"categories":flags,"category_scores":scores,"category_applied_input_types":applied
    }]})
}

fn config(url: &str) -> ModerationConfig {
    ModerationConfig {
        api_base_url: format!("{url}/v1"),
        image_token_reservation: 1,
        ..Default::default()
    }
}

fn provider(
    config: ModerationConfig,
    budget: Arc<MemoryModerationBudget>,
) -> OpenAiModerationProvider {
    OpenAiModerationProvider::new(Arc::new(
        ModerationClient::new(
            config,
            ModerationCredentials::new("test-secret").expect("test credentials"),
            budget,
        )
        .expect("client"),
    ))
}

fn text() -> ProviderScanRequest {
    ProviderScanRequest::for_subject(SubjectKind::Post, "post-a").with_text("benign text")
}

#[tokio::test]
async fn dedicated_endpoint_uses_boolean_without_cross_category_confidence() {
    let server = MockServer::start().await;
    let mut body = response(false, &[("sexual", 0.02)]);
    body["results"][0]["category_scores"]["violence"] = serde_json::json!(0.99);
    Mock::given(method("POST"))
        .and(path("/v1/moderations"))
        .and(header("authorization", "Bearer test-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;
    let p = provider(
        config(&server.uri()),
        Arc::new(MemoryModerationBudget::default()),
    );
    let result = p.scan(&text()).await.expect("scan");
    let mut policy = SafetyPolicy::public_node_default();
    policy.require_known_csam = false;
    let verdict = route(std::slice::from_ref(&result), &policy, "now");
    assert!(verdict.is_labeled_allow());
    assert_eq!(verdict.advisory_labels.len(), 1);
    assert_eq!(verdict.advisory_labels[0].confidence, Some(2));
    assert_eq!(
        result.coverage.expect("coverage").input,
        ScanInputKind::Text
    );
    let requests = server.received_requests().await.expect("requests");
    let input: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("JSON");
    assert_eq!(input["input"].as_array().expect("input").len(), 1);
    assert_eq!(input["input"][0]["type"], "text");
    assert!(input.get("messages").is_none());
    assert!(!format!("{p:?}").contains("test-secret"));
}

#[test]
fn frame_aggregation_keeps_each_category_and_unsupported_coverage() {
    let mut a = ModerationAssessment::parse(
        &serde_json::to_vec(&response(true, &[("sexual", 0.02)])).expect("json"),
        ScanInputKind::Image,
    )
    .expect("first");
    let b = ModerationAssessment::parse(
        &serde_json::to_vec(&response(true, &[("violence", 0.81)])).expect("json"),
        ScanInputKind::Image,
    )
    .expect("second");
    a.merge(b).expect("merge");
    assert_eq!(a.categories["sexual"].confidence, 200);
    assert_eq!(a.categories["violence"].confidence, 8100);
    let result = a.into_result(ScanInputKind::Video, 2, Some(10_000));
    assert_eq!(result.labels.len(), 2);
    assert_eq!(
        result
            .labels
            .iter()
            .find(|l| l.category == SafetyCategory::Nsfw)
            .expect("nsfw")
            .confidence,
        Some(2)
    );
    let coverage = result.coverage.expect("coverage");
    assert!(!coverage.audio_scanned);
    assert!(
        coverage
            .unsupported_categories
            .contains(&"sexual/minors".to_owned())
    );
    assert!(
        !coverage
            .evaluated_categories
            .contains(&"sexual/minors".to_owned())
    );
}

#[test]
fn malformed_or_incomplete_response_is_not_clean() {
    for kind in 0..5 {
        let mut body = response(true, &[]);
        match kind {
            0 => body["results"] = serde_json::json!([]),
            1 => body["results"][0]["categories"]["sexual"] = serde_json::Value::Null,
            2 => body["results"][0]["category_scores"]["violence"] = serde_json::json!(1.1),
            3 => {
                body["results"][0]["category_applied_input_types"]["sexual"] =
                    serde_json::json!(["text"])
            }
            _ => body["results"][0]["categories"]["sexual/minors"] = serde_json::json!(true),
        }
        assert!(
            ModerationAssessment::parse(
                &serde_json::to_vec(&body).expect("JSON"),
                ScanInputKind::Image
            )
            .is_err()
        );
    }
}

#[test]
fn credentials_are_required_redacted_and_configuration_invalidates_identity() {
    assert_eq!(API_KEY_ENV, "COMMUNITY_NODE_VLM_API_KEY");
    assert!(ModerationCredentials::new(" ").is_err());
    assert!(
        !format!("{:?}", ModerationCredentials::new("a-secret").expect("key")).contains("a-secret")
    );
    let a = ModerationConfig::default();
    let mut b = a.clone();
    b.configuration_version = "2".into();
    assert_ne!(a.fingerprint(), b.fingerprint());
}

struct ImageFetcher(Vec<u8>);

#[tokio::test]
async fn animated_png_is_rejected_before_http() {
    use image::AnimationDecoder;
    // Locally generated 8x8 red/blue fixture: no third-party or user content.
    let bytes = include_bytes!("fixtures/two-frame-apng.png").to_vec();
    let decoder =
        image::codecs::png::PngDecoder::new(std::io::Cursor::new(&bytes)).expect("PNG fixture");
    assert!(decoder.is_apng().expect("animation metadata"));
    let frames = decoder
        .apng()
        .expect("APNG decoder")
        .into_frames()
        .collect_frames()
        .expect("APNG frames");
    assert_eq!(frames.len(), 2);
    assert_ne!(
        frames[0].buffer().get_pixel(0, 0),
        frames[1].buffer().get_pixel(0, 0)
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response(true, &[])))
        .expect(0)
        .mount(&server)
        .await;
    let p = provider(
        config(&server.uri()),
        Arc::new(MemoryModerationBudget::default()),
    )
    .with_media_fetcher(Arc::new(ImageFetcher(bytes)));
    assert!(
        p.scan(
            &ProviderScanRequest::for_subject(SubjectKind::Blob, "apng").with_media_hint("apng")
        )
        .await
        .is_err()
    );
}
#[async_trait]
impl MediaFetcher for ImageFetcher {
    async fn fetch(&self, _: &str, _: Option<&str>) -> Result<FetchedMedia, ScanError> {
        Ok(FetchedMedia {
            bytes: self.0.clone(),
            content_type: "video/mp4".into(),
        })
    }
}

#[tokio::test]
async fn static_gif_is_normalized_but_animation_is_not_declared_scanned() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response(true, &[])))
        .expect(1)
        .mount(&server)
        .await;
    for frames in [1, 2] {
        let mut gif = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut gif);
            for _ in 0..frames {
                encoder
                    .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                        8,
                        8,
                        image::Rgba([10, 20, 30, 255]),
                    )))
                    .expect("GIF frame");
            }
        }
        let p = provider(
            config(&server.uri()),
            Arc::new(MemoryModerationBudget::default()),
        )
        .with_media_fetcher(Arc::new(ImageFetcher(gif)));
        let result = p
            .scan(
                &ProviderScanRequest::for_subject(SubjectKind::Blob, "gif").with_media_hint("gif"),
            )
            .await;
        assert_eq!(result.is_ok(), frames == 1);
    }
}

#[tokio::test]
async fn auth_failure_is_not_retried_or_exposed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("test-secret private input"))
        .expect(1)
        .mount(&server)
        .await;
    let p = provider(
        config(&server.uri()),
        Arc::new(MemoryModerationBudget::default()),
    );
    let error = p
        .scan(&text())
        .await
        .expect_err("auth must fail")
        .to_string();
    assert!(error.contains("401"));
    assert!(!error.contains("test-secret"));
    assert!(!error.contains("private input"));
}

#[tokio::test]
async fn separately_constructed_clients_share_budget_and_do_not_send_on_exhaustion() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response(false, &[])))
        .expect(1)
        .mount(&server)
        .await;
    let budget = Arc::new(MemoryModerationBudget::default());
    let mut cfg = config(&server.uri());
    cfg.budget.requests_per_minute = 1;
    cfg.scan_timeout = Duration::from_secs(1);
    cfg.http_timeout = Duration::from_millis(500);
    provider(cfg.clone(), budget.clone())
        .scan(&text())
        .await
        .expect("first admission");
    assert!(provider(cfg, budget).scan(&text()).await.is_err());
}

struct VideoFetcher;

struct CurrentReference(Arc<std::sync::atomic::AtomicBool>);
#[async_trait]
impl kukuri_cn_safety::provider::ScanReferenceGuard for CurrentReference {
    async fn check(&self) -> Result<(), ScanError> {
        if self.0.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(ScanError::Unavailable("reference revoked".into()))
        }
    }
}

#[tokio::test]
async fn revocation_stops_remaining_frames_and_retries() {
    for retry in [false, true] {
        let server = MockServer::start().await;
        let current = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let invalidated = current.clone();
        Mock::given(method("POST"))
            .respond_with(move |_: &wiremock::Request| {
                invalidated.store(false, Ordering::SeqCst);
                if retry {
                    ResponseTemplate::new(429).insert_header("retry-after", "0")
                } else {
                    ResponseTemplate::new(200).set_body_json(response(true, &[]))
                }
            })
            .expect(1)
            .mount(&server)
            .await;
        let p = provider(
            config(&server.uri()),
            Arc::new(MemoryModerationBudget::default()),
        )
        .with_media_fetcher(Arc::new(VideoFetcher))
        .with_video_extractor(Arc::new(Frames));
        let request = if retry {
            text()
        } else {
            ProviderScanRequest::for_subject(SubjectKind::Blob, "video").with_media_hint("video")
        };
        assert!(
            p.scan_guarded(&request, &CurrentReference(current))
                .await
                .is_err()
        );
    }
}
#[async_trait]
impl MediaFetcher for VideoFetcher {
    async fn fetch(&self, _: &str, _: Option<&str>) -> Result<FetchedMedia, ScanError> {
        Ok(FetchedMedia {
            bytes: b"\0\0\0\x18ftypisom".to_vec(),
            content_type: "image/png".into(),
        })
    }
}
struct Frames;
#[async_trait]
impl VideoFrameExtractor for Frames {
    fn config_fingerprint(&self) -> String {
        "fixture-frames-v1".into()
    }
    async fn extract(&self, _: &[u8]) -> Result<VideoScanFrames, ScanError> {
        Ok(VideoScanFrames {
            duration_ms: 12_000,
            frames: (0..3)
                .map(|i| FetchedMedia {
                    bytes: vec![0xff, 0xd8, 0xff, i],
                    content_type: "image/jpeg".into(),
                })
                .collect(),
        })
    }
}

#[tokio::test]
async fn partial_video_failure_returns_no_completed_result_and_never_sends_video() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(200).set_body_json(response(true, &[]))
            } else {
                ResponseTemplate::new(403)
            }
        })
        .expect(2)
        .mount(&server)
        .await;
    let p = provider(
        config(&server.uri()),
        Arc::new(MemoryModerationBudget::default()),
    )
    .with_media_fetcher(Arc::new(VideoFetcher))
    .with_video_extractor(Arc::new(Frames));
    let request =
        ProviderScanRequest::for_subject(SubjectKind::Blob, "video-a").with_media_hint("video-a");
    assert!(p.scan(&request).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    for request in server.received_requests().await.expect("requests") {
        let json: serde_json::Value = serde_json::from_slice(&request.body).expect("JSON");
        assert_eq!(json["input"].as_array().expect("input").len(), 1);
        assert!(
            json["input"][0]["image_url"]["url"]
                .as_str()
                .expect("url")
                .starts_with("data:image/jpeg;base64,")
        );
    }
}
