use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub mod adapters;

pub fn logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "marivect=info,tower_http=info".into()),
        )
        .init();
}
pub async fn pool() -> anyhow::Result<PgPool> {
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?)
}
pub fn hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = Uuid::new_v4();
        (
            self.0,
            Json(serde_json::json!({"error":{"code":self.1,"request_id":request_id}})),
        )
            .into_response()
    }
}
impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database operation failed");
        Self(StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
    }
}

// A device credential is deliberately NOT a human user or a tenant-admin session.
#[derive(sqlx::FromRow)]
pub struct SourceIdentity {
    pub tenant_id: Uuid,
    pub device_id: Uuid,
}
pub async fn authenticate(pool: &PgPool, headers: &HeaderMap) -> Result<SourceIdentity, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|s| s.len() >= 32 && s.len() <= 256)
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "invalid_credential"))?;
    sqlx::query_as::<_, SourceIdentity>("SELECT * FROM authenticate_source($1)")
        .bind(hash(token))
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "invalid_credential"))
}
pub async fn tenant_tx(
    pool: &PgPool,
    tenant: Uuid,
) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub key: String,
    pub value: f64,
    pub unit: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushEvent {
    pub schema_version: u32,
    pub message_id: String,
    pub observed_at: DateTime<Utc>,
    pub measurements: Vec<Reading>,
}
impl PushEvent {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), &'static str> {
        if self.schema_version != 1 {
            return Err("unsupported_schema");
        }
        if self.message_id.is_empty() || self.message_id.len() > 128 {
            return Err("invalid_message_id");
        }
        if self.measurements.is_empty() || self.measurements.len() > 128 {
            return Err("invalid_batch_size");
        }
        if self.observed_at > now + chrono::Duration::minutes(5)
            || self.observed_at < now - chrono::Duration::days(366)
        {
            return Err("invalid_timestamp");
        }
        let mut seen = std::collections::HashSet::new();
        for m in &self.measurements {
            if m.key.is_empty()
                || m.key.len() > 64
                || m.unit.len() > 32
                || !m.value.is_finite()
                || !seen.insert(&m.key)
            {
                return Err("invalid_measurement");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> PushEvent {
        PushEvent {
            schema_version: 1,
            message_id: "sample:1".into(),
            observed_at: Utc::now(),
            measurements: vec![Reading {
                key: "battery_voltage".into(),
                value: 12.8,
                unit: "V".into(),
            }],
        }
    }
    #[test]
    fn accepts_valid_event() {
        assert!(event().validate(Utc::now()).is_ok());
    }
    #[test]
    fn rejects_duplicate_keys() {
        let mut e = event();
        e.measurements.push(e.measurements[0].clone());
        assert!(e.validate(Utc::now()).is_err());
    }
    #[test]
    fn rejects_nan() {
        let mut e = event();
        e.measurements[0].value = f64::NAN;
        assert!(e.validate(Utc::now()).is_err());
    }
    #[test]
    fn rejects_future() {
        let mut e = event();
        e.observed_at += chrono::Duration::days(1);
        assert!(e.validate(Utc::now()).is_err());
    }
}
