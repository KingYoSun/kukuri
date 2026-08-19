use super::*;

pub(crate) fn community_node_http_client() -> Result<Client> {
    Client::builder()
        .build()
        .context("failed to build community-node http client")
}

/// 通報送信専用の HTTP クライアント(#703)。
///
/// 通報本文(詳細・連絡先)が転送応答で別ホストへ再送されないよう、転送を追跡しない。
/// 3xx は呼び出し側で `REPORT_REDIRECT_REJECTED` として扱う。
pub(crate) fn community_node_report_http_client() -> Result<Client> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build community-node report http client")
}

#[derive(Debug)]
pub(crate) enum CommunityNodeRequestError {
    AuthRequired,
    ConsentRequired,
    Other(anyhow::Error),
}

impl CommunityNodeRequestError {
    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        match self {
            Self::AuthRequired => anyhow!("community node authentication is required"),
            Self::ConsentRequired => anyhow!("community node consent is required"),
            Self::Other(error) => error,
        }
    }
}
