use crate::{hash, ApiError};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const COOKIE: &str = "marivect_session";
static PASSWORD_WORK: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
pub fn secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}
pub fn password_hash(password: &str) -> anyhow::Result<String> {
    let salt =
        SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("{e}"))
}
pub fn verify(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).is_ok_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
pub fn email(address: &str) -> Option<String> {
    let e = address.trim().to_lowercase();
    (e.len() <= 254
        && e.contains('@')
        && !e.chars().any(char::is_whitespace)
        && !e.chars().any(char::is_control))
    .then_some(e)
}
pub fn cookie_token(headers: &HeaderMap) -> Result<String, ApiError> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|p| p.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, v)| v.to_owned())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "login_required"))
}
pub fn origin(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = std::env::var("APP_ORIGIN").unwrap_or_else(|_| "http://localhost:8080".into());
    if headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(expected.as_str()) {
        return Err(ApiError(StatusCode::FORBIDDEN, "invalid_origin"));
    }
    Ok(())
}
pub fn csrf(headers: &HeaderMap, session: &Session) -> Result<(), ApiError> {
    origin(headers)?;
    if headers.get("x-csrf-token").and_then(|v| v.to_str().ok())
        != Some(session.csrf_token.as_str())
    {
        return Err(ApiError(StatusCode::FORBIDDEN, "csrf_required"));
    }
    Ok(())
}
#[derive(sqlx::FromRow, Serialize)]
pub struct Session {
    pub user_id: Uuid,
    pub email: String,
    pub csrf_token: String,
}
#[derive(sqlx::FromRow, Serialize)]
pub struct Membership {
    pub tenant_id: Uuid,
    pub name: String,
    pub role: String,
}
pub async fn session(db: &PgPool, headers: &HeaderMap) -> Result<Session, ApiError> {
    sqlx::query_as("SELECT * FROM auth_session($1)")
        .bind(hash(&cookie_token(headers)?))
        .fetch_optional(db)
        .await?
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "login_required"))
}
pub async fn memberships(db: &PgPool, headers: &HeaderMap) -> Result<Vec<Membership>, ApiError> {
    sqlx::query_as("SELECT * FROM auth_memberships($1)")
        .bind(hash(&cookie_token(headers)?))
        .fetch_all(db)
        .await
        .map_err(Into::into)
}
pub async fn access(
    db: &PgPool,
    headers: &HeaderMap,
    tenant: Uuid,
) -> Result<Membership, ApiError> {
    session(db, headers).await?;
    memberships(db, headers)
        .await?
        .into_iter()
        .find(|m| m.tenant_id == tenant)
        .ok_or(ApiError(StatusCode::FORBIDDEN, "tenant_access_denied"))
}
pub fn cookie(value: &str, clear: bool) -> String {
    let secure = std::env::var("COOKIE_SECURE").as_deref() != Ok("false");
    format!(
        "{COOKIE}={value}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}{}",
        if clear { 0 } else { 43200 },
        if secure { "; Secure" } else { "" }
    )
}
pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/logout", post(logout))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    email: String,
    password: String,
}
async fn login(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Json(input): Json<Login>,
) -> Result<Response, ApiError> {
    origin(&headers)?;
    let address = email(&input.email).ok_or(ApiError(StatusCode::UNAUTHORIZED, "invalid_login"))?;
    if input.password.len() > 1024 || input.password.is_empty() {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "invalid_login"));
    }
    let permit = PASSWORD_WORK
        .try_acquire()
        .map_err(|_| ApiError(StatusCode::TOO_MANY_REQUESTS, "try_later"))?;
    let row = sqlx::query("SELECT * FROM auth_lookup($1)")
        .bind(address)
        .fetch_optional(&db)
        .await?;
    let id = row.as_ref().map(|r| r.get::<Uuid, _>("id"));
    let encoded = row.as_ref().map(|r| r.get::<String, _>("password_hash"));
    let password = input.password;
    let valid = tokio::task::spawn_blocking(move || match encoded {
        Some(h) => verify(&password, &h),
        None => {
            let _ = password_hash(&password);
            false
        }
    })
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal_error"))?;
    drop(permit);
    if !valid {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "invalid_login"));
    }
    let token = secret();
    let csrf = secret();
    let mut tx = db.begin().await?;
    // Rotate existing browser session to avoid retaining it after another login.
    if let Ok(previous) = cookie_token(&headers) {
        sqlx::query("SELECT auth_close($1)")
            .bind(hash(&previous))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("SELECT auth_open($1,$2,$3)")
        .bind(id.expect("verified user"))
        .bind(hash(&token))
        .bind(&csrf)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let mut response = Json(serde_json::json!({"csrf_token":csrf})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie(&token, false).parse().expect("ASCII token"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    Ok(response)
}
async fn me(State(db): State<PgPool>, headers: HeaderMap) -> Result<Response, ApiError> {
    let s = session(&db, &headers).await?;
    let m = memberships(&db, &headers).await?;
    let mut response = Json(serde_json::json!({"user":s,"memberships":m})).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    Ok(response)
}
async fn logout(State(db): State<PgPool>, headers: HeaderMap) -> Result<Response, ApiError> {
    let s = session(&db, &headers).await?;
    csrf(&headers, &s)?;
    sqlx::query("SELECT auth_close($1)")
        .bind(hash(&cookie_token(&headers)?))
        .execute(&db)
        .await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie("", true).parse().unwrap());
    Ok(response)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_roundtrip() {
        let h = password_hash("test-password-long").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify("test-password-long", &h));
        assert!(!verify("wrong", &h));
    }
    #[test]
    fn ignores_wrong_cookie() {
        let mut h = HeaderMap::new();
        h.insert(
            "cookie",
            "another=123; marivect_session=short".parse().unwrap(),
        );
        assert!(cookie_token(&h).is_err());
    }
    #[test]
    fn normalizes_email() {
        assert_eq!(email(" A@EXAMPLE.COM "), Some("a@example.com".into()));
        assert!(email("bad value").is_none());
    }
}
