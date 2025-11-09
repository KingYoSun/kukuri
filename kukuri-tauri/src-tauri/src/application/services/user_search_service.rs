use crate::application::ports::repositories::UserRepository;
use crate::domain::entities::User;
use crate::shared::{AppError, ValidationFailureKind};
use chrono::Utc;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub(crate) const DEFAULT_LIMIT: usize = 20;
pub(crate) const MAX_LIMIT: usize = 50;
pub(crate) const MAX_FETCH: usize = 250;
const RATE_LIMIT_MAX_REQUESTS: usize = 30;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);

#[derive(Clone, Copy)]
pub enum SearchSort {
    Relevance,
    Recency,
}

impl SearchSort {
    pub fn try_from_str(value: Option<&str>) -> Result<Self, AppError> {
        match value.unwrap_or("relevance").to_lowercase().as_str() {
            "relevance" => Ok(SearchSort::Relevance),
            "recency" => Ok(SearchSort::Recency),
            other => Err(AppError::validation(
                ValidationFailureKind::Generic,
                format!("Unsupported search sort: {other}"),
            )),
        }
    }
}

pub struct SearchUsersParams {
    pub query: String,
    pub cursor: Option<String>,
    pub limit: usize,
    pub sort: SearchSort,
    pub allow_incomplete: bool,
    pub viewer_pubkey: Option<String>,
}

#[derive(Debug)]
pub struct SearchUsersResult {
    pub users: Vec<User>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_count: usize,
    pub took_ms: u128,
}

pub struct UserSearchService {
    repository: Arc<dyn UserRepository>,
    rate_limiter: RateLimiter,
}

impl UserSearchService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self {
            repository,
            rate_limiter: RateLimiter::new(RATE_LIMIT_MAX_REQUESTS, RATE_LIMIT_WINDOW),
        }
    }

    pub async fn search(&self, params: SearchUsersParams) -> Result<SearchUsersResult, AppError> {
        let normalized_query = params.query.trim();
        if normalized_query.len() < 2 && !params.allow_incomplete {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "検索キーワードは2文字以上で入力してください",
            ));
        }

        let rate_key = params
            .viewer_pubkey
            .clone()
            .unwrap_or_else(|| format!("anon:{}", normalized_query.to_lowercase()));
        self.rate_limiter.check_and_record(&rate_key).await?;

        let start = Instant::now();
        let fetch_limit = MAX_FETCH.min(params.limit + MAX_LIMIT);
        let raw_users = self
            .repository
            .search_users(normalized_query, fetch_limit)
            .await?;

        let (following, followers) = if let Some(pubkey) = params.viewer_pubkey.as_ref() {
            let following = self.repository.list_following_pubkeys(pubkey).await?;
            let followers = self.repository.list_follower_pubkeys(pubkey).await?;
            (
                HashSet::from_iter(following.into_iter()),
                HashSet::from_iter(followers.into_iter()),
            )
        } else {
            (HashSet::new(), HashSet::new())
        };

        let query_lower = normalized_query.to_lowercase();
        let mut ranked: Vec<RankedUser> = raw_users
            .into_iter()
            .map(|user| {
                let rank = compute_rank(&user, &query_lower, &following, &followers);
                RankedUser { user, rank }
            })
            .collect();

        match params.sort {
            SearchSort::Relevance => ranked.sort_by(|a, b| {
                b.rank
                    .partial_cmp(&a.rank)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.user.npub.cmp(&b.user.npub))
            }),
            SearchSort::Recency => ranked.sort_by(|a, b| {
                b.user
                    .updated_at
                    .cmp(&a.user.updated_at)
                    .then_with(|| a.user.npub.cmp(&b.user.npub))
            }),
        }

        let total_count = ranked.len();
        let start_index = cursor_start_index(&ranked, &params)?;
        let end_index = (start_index + params.limit).min(total_count);
        let has_more = end_index < total_count;

        let users: Vec<User> = ranked[start_index..end_index]
            .iter()
            .map(|entry| entry.user.clone())
            .collect();

        let next_cursor = if has_more && !users.is_empty() {
            Some(encode_cursor(params.sort, &ranked[end_index - 1]))
        } else {
            None
        };

        Ok(SearchUsersResult {
            users,
            next_cursor,
            has_more,
            total_count,
            took_ms: start.elapsed().as_millis(),
        })
    }
}

struct RankedUser {
    user: User,
    rank: f64,
}

fn cursor_start_index(
    ranked: &[RankedUser],
    params: &SearchUsersParams,
) -> Result<usize, AppError> {
    if let Some(cursor) = params.cursor.as_ref() {
        match parse_cursor(cursor)? {
            Cursor::Relevance { rank, npub } => {
                if matches!(params.sort, SearchSort::Relevance) {
                    if let Some(pos) = ranked.iter().position(|entry| {
                        entry.user.npub == npub && (entry.rank - rank).abs() < f64::EPSILON
                    }) {
                        return Ok(pos + 1);
                    }
                    if let Some(pos) = ranked.iter().position(|entry| entry.user.npub == npub) {
                        return Ok(pos + 1);
                    }
                }
            }
            Cursor::Recency { updated_at, npub } => {
                if matches!(params.sort, SearchSort::Recency) {
                    if let Some(pos) = ranked.iter().position(|entry| {
                        entry.user.npub == npub
                            && entry.user.updated_at.timestamp_millis() == updated_at
                    }) {
                        return Ok(pos + 1);
                    }
                }
            }
        }
    }
    Ok(0)
}

fn compute_rank(
    user: &User,
    query: &str,
    following: &HashSet<String>,
    followers: &HashSet<String>,
) -> f64 {
    let text_score = compute_text_score(user, query);
    let mutual_score = compute_mutual_score(user, following, followers);
    let recency_score = compute_recency_score(user);
    text_score * 0.7 + mutual_score * 0.2 + recency_score * 0.1
}

fn compute_text_score(user: &User, query: &str) -> f64 {
    let lowered = query.to_lowercase();
    let mut score = 0.0;

    let name = user.profile.display_name.to_lowercase();
    if name.starts_with(&lowered) {
        score += 4.0;
    } else if name.contains(&lowered) {
        score += 3.0;
    }

    if user.npub.to_lowercase().contains(&lowered) {
        score += 2.0;
    }

    if user.profile.bio.to_lowercase().contains(&lowered) {
        score += 1.0;
    }

    score
}

fn compute_mutual_score(
    user: &User,
    following: &HashSet<String>,
    followers: &HashSet<String>,
) -> f64 {
    let mut score = 0.0;
    if following.contains(user.pubkey()) {
        score += 1.0;
    }
    if followers.contains(user.pubkey()) {
        score += 1.0;
    }
    score
}

fn compute_recency_score(user: &User) -> f64 {
    let now = Utc::now();
    let elapsed = now
        .signed_duration_since(user.updated_at)
        .num_seconds()
        .max(0) as f64;
    let hours = elapsed / 3600.0;
    1.0 / (1.0 + hours)
}

enum Cursor {
    Relevance { rank: f64, npub: String },
    Recency { updated_at: i64, npub: String },
}

fn encode_cursor(sort: SearchSort, entry: &RankedUser) -> String {
    match sort {
        SearchSort::Relevance => format!("rel:{:.5}:{}", entry.rank, entry.user.npub),
        SearchSort::Recency => format!(
            "rec:{}:{}",
            entry.user.updated_at.timestamp_millis(),
            entry.user.npub
        ),
    }
}

fn parse_cursor(cursor: &str) -> Result<Cursor, AppError> {
    let mut parts = cursor.splitn(3, ':');
    match parts.next() {
        Some("rel") => {
            let rank = parts
                .next()
                .ok_or_else(|| AppError::InvalidInput("Invalid cursor".into()))?
                .parse::<f64>()
                .map_err(|_| AppError::InvalidInput("Invalid cursor rank".into()))?;
            let npub = parts
                .next()
                .ok_or_else(|| AppError::InvalidInput("Invalid cursor".into()))?;
            Ok(Cursor::Relevance {
                rank,
                npub: npub.to_string(),
            })
        }
        Some("rec") => {
            let timestamp = parts
                .next()
                .ok_or_else(|| AppError::InvalidInput("Invalid cursor".into()))?
                .parse::<i64>()
                .map_err(|_| AppError::InvalidInput("Invalid cursor timestamp".into()))?;
            let npub = parts
                .next()
                .ok_or_else(|| AppError::InvalidInput("Invalid cursor".into()))?;
            Ok(Cursor::Recency {
                updated_at: timestamp,
                npub: npub.to_string(),
            })
        }
        _ => Err(AppError::InvalidInput("Invalid cursor prefix".into())),
    }
}

struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }
    }

    async fn check_and_record(&self, key: &str) -> Result<(), AppError> {
        let mut guard = self.requests.lock().await;
        let now = Instant::now();
        let entries = guard.entry(key.to_string()).or_default();
        entries.retain(|instant| now.duration_since(*instant) < self.window);
        if entries.len() >= self.max_requests {
            let retry_after = self
                .window
                .checked_sub(now.duration_since(entries[0]))
                .unwrap_or_default();
            return Err(AppError::rate_limited(
                "一定時間後に再試行してください",
                retry_after.as_secs().max(1),
            ));
        }
        entries.push(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::UserRepository;
    use crate::infrastructure::database::{
        connection_pool::ConnectionPool, repository::Repository, sqlite_repository::SqliteRepository,
    };
    use chrono::{Duration as ChronoDuration, Utc};

    async fn setup_service() -> (UserSearchService, Arc<SqliteRepository>) {
        let pool = ConnectionPool::from_memory().await.unwrap();
        let repository = Arc::new(SqliteRepository::new(pool.clone()));
        repository.initialize().await.unwrap();
        let service = UserSearchService::new(Arc::clone(&repository) as Arc<dyn UserRepository>);
        (service, repository)
    }

    async fn insert_user(
        repository: &Arc<SqliteRepository>,
        npub: &str,
        display_name: &str,
        bio: &str,
        updated_offset_secs: i64,
    ) -> User {
        let pubkey = format!("pubkey_{npub}");
        let mut user = User::new(npub.to_string(), pubkey.clone());
        user.profile.display_name = display_name.to_string();
        user.profile.bio = bio.to_string();
        user.updated_at = Utc::now() - ChronoDuration::seconds(updated_offset_secs);
        repository.create_user(&user).await.unwrap();
        user
    }

    #[tokio::test]
    async fn search_returns_ranked_results_with_cursor() {
        let (service, repository) = setup_service().await;
        insert_user(&repository, "npub1alice", "Alice", "nostr dev", 10).await;
        insert_user(&repository, "npub1alicia", "Alicia", "nostr dev", 20).await;
        insert_user(&repository, "npub1bob", "Bob", "rustacean", 5).await;

        let first = service
            .search(SearchUsersParams {
                query: "ali".to_string(),
                cursor: None,
                limit: 1,
                sort: SearchSort::Relevance,
                allow_incomplete: false,
                viewer_pubkey: None,
            })
            .await
            .expect("first page");

        assert_eq!(first.users.len(), 1);
        assert!(first.has_more);
        let cursor = first.next_cursor.clone().expect("cursor present");

        let second = service
            .search(SearchUsersParams {
                query: "ali".to_string(),
                cursor: Some(cursor),
                limit: 1,
                sort: SearchSort::Relevance,
                allow_incomplete: false,
                viewer_pubkey: None,
            })
            .await
            .expect("second page");

        assert_eq!(second.users.len(), 1);
        assert_ne!(first.users[0].npub, second.users[0].npub);
    }

    #[tokio::test]
    async fn search_prioritizes_follow_relationships() {
        let (service, repository) = setup_service().await;
        let viewer = insert_user(&repository, "npub1viewer", "Viewer", "viewer", 1).await;
        let alice = insert_user(&repository, "npub1alice", "Alice", "nostr dev", 10).await;
        let bob = insert_user(&repository, "npub1bob", "Bob", "nostr dev", 5).await;

        repository
            .add_follow_relation(viewer.pubkey(), alice.pubkey())
            .await
            .unwrap();
        repository
            .add_follow_relation(alice.pubkey(), viewer.pubkey())
            .await
            .unwrap();

        let result = service
            .search(SearchUsersParams {
                query: "nostr".to_string(),
                cursor: None,
                limit: 5,
                sort: SearchSort::Relevance,
                allow_incomplete: false,
                viewer_pubkey: Some(viewer.pubkey().to_string()),
            })
            .await
            .expect("search");

        assert_eq!(result.users.first().unwrap().npub, alice.npub);
        assert!(result.users.iter().any(|user| user.npub == bob.npub));
    }

    #[tokio::test]
    async fn rate_limiter_blocks_excess_requests() {
        let (service, repository) = setup_service().await;
        insert_user(&repository, "npub1alice", "Alice", "nostr dev", 1).await;

        for _ in 0..RATE_LIMIT_MAX_REQUESTS {
            service
                .search(SearchUsersParams {
                    query: "alice".to_string(),
                    cursor: None,
                    limit: 1,
                    sort: SearchSort::Relevance,
                    allow_incomplete: false,
                    viewer_pubkey: None,
                })
                .await
                .expect("search within limit");
        }

        let err = service
            .search(SearchUsersParams {
                query: "alice".to_string(),
                cursor: None,
                limit: 1,
                sort: SearchSort::Relevance,
                allow_incomplete: false,
                viewer_pubkey: None,
            })
            .await
            .expect_err("rate limit triggered");
        assert!(matches!(err, AppError::RateLimited { .. }));
    }

    #[tokio::test]
    async fn allow_incomplete_accepts_short_queries() {
        let (service, repository) = setup_service().await;
        insert_user(&repository, "npub1alice", "Alice", "nostr dev", 1).await;

        let result = service
            .search(SearchUsersParams {
                query: "a".to_string(),
                cursor: None,
                limit: 5,
                sort: SearchSort::Relevance,
                allow_incomplete: true,
                viewer_pubkey: None,
            })
            .await
            .expect("short query allowed");

        assert_eq!(result.total_count, 1);
    }
}
