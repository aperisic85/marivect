use crate::{auth, hash, tenant_tx, ApiError};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route(
            "/api/v1/tenants/{tenant}/devices",
            get(devices).post(create),
        )
        .route(
            "/api/v1/tenants/{tenant}/devices/{device}/latest",
            get(latest),
        )
}
#[derive(sqlx::FromRow, Serialize)]
struct Device {
    id: Uuid,
    name: String,
    capabilities: Vec<String>,
}
async fn devices(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Path(tenant): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth::access(&db, &headers, tenant).await?;
    let mut tx = tenant_tx(&db, tenant).await?;
    let rows: Vec<Device> = sqlx::query_as(
        "SELECT id,name,capabilities FROM devices WHERE tenant_id=$1 ORDER BY name,id LIMIT 200",
    )
    .bind(tenant)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"devices":rows,"limit":200})))
}
async fn latest(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Path((tenant, device)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth::access(&db, &headers, tenant).await?;
    let mut tx = tenant_tx(&db, tenant).await?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE tenant_id=$1 AND id=$2)")
            .bind(tenant)
            .bind(device)
            .fetch_one(&mut *tx)
            .await?;
    if !exists {
        return Err(ApiError(StatusCode::NOT_FOUND, "device_not_found"));
    }
    let rows=sqlx::query("SELECT s.key,s.unit,l.value_number,l.observed_at,l.received_at FROM sensors s LEFT JOIN device_latest_values l ON (l.tenant_id,l.sensor_id)=(s.tenant_id,s.id) WHERE s.tenant_id=$1 AND s.device_id=$2 ORDER BY s.key LIMIT 128")
 .bind(tenant).bind(device).fetch_all(&mut *tx).await?;
    let readings:Vec<_>=rows.iter().map(|r|serde_json::json!({"key":r.get::<String,_>("key"),"unit":r.get::<String,_>("unit"),"value":r.get::<Option<f64>,_>("value_number"),"observed_at":r.get::<Option<chrono::DateTime<chrono::Utc>>,_>("observed_at"),"received_at":r.get::<Option<chrono::DateTime<chrono::Utc>>,_>("received_at"),"quality":"unknown"})).collect();
    tx.commit().await?;
    Ok(Json(
        serde_json::json!({"device_id":device,"measurements":readings}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewSensor {
    key: String,
    unit: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewDevice {
    name: String,
    sensors: Vec<NewSensor>,
}
impl NewDevice {
    fn validate(&self) -> bool {
        let mut keys = std::collections::HashSet::new();
        !self.name.trim().is_empty()
            && self.name.len() <= 120
            && !self.name.chars().any(char::is_control)
            && !self.sensors.is_empty()
            && self.sensors.len() <= 128
            && self.sensors.iter().all(|s| {
                !s.key.is_empty()
                    && s.key.len() <= 64
                    && s.key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && s.unit.len() <= 32
                    && !s.unit.chars().any(char::is_control)
                    && keys.insert(&s.key)
            })
    }
}
async fn create(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Path(tenant): Path<Uuid>,
    Json(input): Json<NewDevice>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let session = auth::session(&db, &headers).await?;
    auth::csrf(&headers, &session)?;
    let member = auth::access(&db, &headers, tenant).await?;
    if member.role != "tenant_admin" {
        return Err(ApiError(StatusCode::FORBIDDEN, "admin_required"));
    }
    if !input.validate() {
        return Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "invalid_device"));
    }
    let mut tx = tenant_tx(&db, tenant).await?;
    let id = Uuid::new_v4();
    let token = auth::secret();
    sqlx::query("INSERT INTO devices(id,tenant_id,name,capabilities) VALUES($1,$2,$3,ARRAY['telemetry.push'])")
 .bind(id).bind(tenant).bind(input.name.trim()).execute(&mut *tx).await?;
    for s in input.sensors {
        sqlx::query("INSERT INTO sensors(id,tenant_id,device_id,key,unit) VALUES($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4())
            .bind(tenant)
            .bind(id)
            .bind(s.key)
            .bind(s.unit)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("SELECT register_source($1,$2,$3)")
        .bind(tenant)
        .bind(id)
        .bind(hash(&token))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"device_id":id,"source_token":token})),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_sensor_keys() {
        let input = NewDevice {
            name: "Device".into(),
            sensors: vec![
                NewSensor {
                    key: "a".into(),
                    unit: "V".into(),
                },
                NewSensor {
                    key: "a".into(),
                    unit: "V".into(),
                },
            ],
        };
        assert!(!input.validate());
    }
}
