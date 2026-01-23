use anyhow::{anyhow, Result};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone)]
pub struct JwtConfig {
    pub issuer: String,
    pub audience: String,
    pub secret: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    pub aud: String,
    pub iss: String,
}

pub fn issue_token(pubkey: &str, config: &JwtConfig) -> Result<(String, AccessTokenClaims)> {
    let now = unix_seconds()?;
    let exp = now
        .checked_add(config.ttl_seconds)
        .ok_or_else(|| anyhow!("token expiry overflow"))?;

    let claims = AccessTokenClaims {
        sub: pubkey.to_string(),
        exp: exp as usize,
        iat: now as usize,
        jti: Uuid::new_v4().to_string(),
        aud: config.audience.clone(),
        iss: config.issuer.clone(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )?;

    Ok((token, claims))
}

pub fn verify_token(token: &str, config: &JwtConfig) -> Result<AccessTokenClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[config.audience.as_str()]);
    validation.set_issuer(&[config.issuer.as_str()]);

    let data = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

pub fn unix_seconds() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| anyhow!("invalid system clock"))
}

pub fn seconds_from_now(seconds: u64) -> Result<SystemTime> {
    let now = SystemTime::now();
    Ok(now + Duration::from_secs(seconds))
}

