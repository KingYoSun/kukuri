use super::*;

pub(crate) fn community_node_http_client() -> Result<Client> {
    Client::builder()
        .build()
        .context("failed to build community-node http client")
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
