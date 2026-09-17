//! ブロック / ミュート観測の提供に対する任意文書（ADR 0026 §8.5、#1061）。
//!
//! 公開は任意で、`LegalDocumentKind::ALL` には含めない。client は固定 slug で文書を識別する。

use std::fmt::Write as _;

use anyhow::{Result, bail};

use crate::config::{LegalConfig, LegalDocumentKind, ResolvedConfig};
use crate::docs::{GeneratedFile, header, planned_section};

/// 観測提供の任意文書に使う固定 slug（`kukuri_cn_protocol::TRUST_OBSERVATION_SHARING_POLICY_SLUG`
/// と同じ値）。
pub const TRUST_OBSERVATION_SHARING_SLUG: &str = "trust_observation_sharing";

/// 文書を公開する場合は任意同意（`required: false`）かつ固定 slug であること。
pub(crate) fn validate_document(legal: &LegalConfig) -> Result<()> {
    let Some(document) = legal
        .documents
        .iter()
        .find(|document| document.kind == LegalDocumentKind::TrustObservationSharing)
    else {
        return Ok(());
    };
    if document.required {
        bail!("trust_observation_sharing は任意同意の文書です。required: false にしてください");
    }
    if document.slug.trim() != TRUST_OBSERVATION_SHARING_SLUG {
        bail!(
            "trust_observation_sharing の slug は `{TRUST_OBSERVATION_SHARING_SLUG}` にしてください"
        );
    }
    Ok(())
}

/// 文書を公開する node だけ生成する。
pub(crate) fn generated_file(config: &ResolvedConfig) -> Option<GeneratedFile> {
    config
        .legal_document(LegalDocumentKind::TrustObservationSharing)
        .map(|_| GeneratedFile {
            filename: LegalDocumentKind::TrustObservationSharing
                .filename()
                .to_string(),
            content: render(config),
        })
}

/// ブロック / ミュート観測の提供に対する任意同意の本文（ADR 0026 §8.5）。
fn render(config: &ResolvedConfig) -> String {
    let mut s = header(
        config,
        "ブロック・ミュート観測の提供",
        Some(LegalDocumentKind::TrustObservationSharing),
    );
    let _ = writeln!(s, "\n## この文書の位置づけ\n");
    let _ = writeln!(
        s,
        "この文書への同意は任意です。同意しなくても、この community node の他の機能は利用できます。\
         同意すると、あなたがこの端末で行ったブロック・ミュートの記録をこの node へ提供します。\
         同意はいつでも取り消せます。\n"
    );
    let _ = writeln!(s, "## 提供する情報\n");
    let _ = writeln!(
        s,
        "- あなたの公開鍵と、ブロック・ミュートした相手の公開鍵\n\
         - 操作の種類（ブロック・ミュート）と状態（有効・解除）\n\
         - 操作の時刻と、あなたの鍵による署名\n"
    );
    let _ = writeln!(
        s,
        "投稿本文、メッセージ、端末内のその他の設定は提供しません。\
         提供するのは、同意した後にこの端末で行った操作と、同意時にあなたが選んだ場合に限り既存のブロック・ミュートです。\n"
    );
    let _ = writeln!(s, "## 提供先と利用目的\n");
    let _ = writeln!(
        s,
        "提供先はこの community node だけです。この node は、提供された記録を、\
         各利用者から見た相手ユーザーの関係評価（relation 値）の調整にだけ使います。\
         あなたと関係の深い利用者ほど、あなたのブロック・ミュートがその利用者に表示される評価へ強く反映されます。\n"
    );
    let _ = writeln!(
        s,
        "この node は、提供者の一覧や件数を他の利用者に開示しません。記録を他の node へ共有せず、\
         ブロック・ミュートを特定の違反の判定として扱いません。\n"
    );
    let _ = writeln!(s, "## 保持期間と取消\n");
    let _ = writeln!(
        s,
        "有効な記録は操作時刻から {active} 日、解除された記録は受信から {revoked} 日で評価から除き、削除します。\
         同意を取り消すと、この node が保持するあなたの記録をすべて削除し、再び同意するまで新しい記録を受け付けません。\n",
        active = 180,
        revoked = 30,
    );
    s.push_str(&planned_section(config));
    s
}
