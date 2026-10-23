use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use marivect::{authenticate, tenant_tx, ApiError, PushEvent};
use sqlx::{PgPool, Row};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    marivect::logging();
    let db = marivect::pool().await?;
    let app = Router::new()
        .route("/health/live", get(|| async { "ok" }))
        .route("/health/ready", get(ready))
        .route("/api/v1/ingest/push", post(push))
        .route("/api/v1/source/latest", get(latest))
        .layer(DefaultBodyLimit::max(65536))
        .layer(TraceLayer::new_for_http())
        .with_state(db);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("API listening on :8080");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("signal handler");
        tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=terminate.recv()=>{} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
async fn ready(State(db): State<PgPool>) -> Result<&'static str, ApiError> {
    sqlx::query("SELECT 1 FROM devices LIMIT 0")
        .execute(&db)
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "not_ready"))?;
    Ok("ready")
}
async fn push(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Json(event): Json<PushEvent>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let identity = authenticate(&db, &headers).await?;
    event
        .validate(chrono::Utc::now())
        .map_err(|code| ApiError(StatusCode::UNPROCESSABLE_ENTITY, code))?;
    let mut tx = tenant_tx(&db, identity.tenant_id).await?;
    // Transaction-scoped advisory lock serializes one source without UPDATE privileges.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text || ':' || $2::uuid::text, 0))")
        .bind(identity.tenant_id).bind(identity.device_id).execute(&mut *tx).await?;
    let payload = serde_json::to_value(&event).expect("validated JSON");
    if let Some(existing)=sqlx::query("SELECT id,payload FROM ingestion_events WHERE tenant_id=$1 AND device_id=$2 AND message_id=$3")
        .bind(identity.tenant_id).bind(identity.device_id).bind(&event.message_id).fetch_optional(&mut *tx).await? {
        let old:serde_json::Value=existing.get("payload");
        if old!=payload { return Err(ApiError(StatusCode::CONFLICT,"message_id_conflict")); }
        let id:Uuid=existing.get("id");
        return Ok(Json(serde_json::json!({"event_id":id,"status":"processed","duplicate":true})));
    }
    // Validate ALL keys and units before writing any readings.
    let mut sensors = Vec::new();
    for m in &event.measurements {
        let id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM sensors WHERE tenant_id=$1 AND device_id=$2 AND key=$3 AND unit=$4",
        )
        .bind(identity.tenant_id)
        .bind(identity.device_id)
        .bind(&m.key)
        .bind(&m.unit)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unknown_sensor_or_unit",
        ))?;
        sensors.push(id);
    }
    let event_id = Uuid::new_v4();
    sqlx::query("INSERT INTO ingestion_events(id,tenant_id,device_id,message_id,payload,status) VALUES($1,$2,$3,$4,$5,'processed')")
        .bind(event_id).bind(identity.tenant_id).bind(identity.device_id).bind(&event.message_id).bind(payload).execute(&mut *tx).await?;
    for (m, sensor) in event.measurements.iter().zip(sensors) {
        sqlx::query("INSERT INTO measurements(tenant_id,device_id,sensor_id,event_id,observed_at,value_number) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(identity.tenant_id).bind(identity.device_id).bind(sensor).bind(event_id).bind(event.observed_at).bind(m.value).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO device_latest_values(tenant_id,device_id,sensor_id,observed_at,value_number) VALUES($1,$2,$3,$4,$5) ON CONFLICT(tenant_id,sensor_id) DO UPDATE SET observed_at=excluded.observed_at,value_number=excluded.value_number,received_at=excluded.received_at WHERE excluded.observed_at>device_latest_values.observed_at")
            .bind(identity.tenant_id).bind(identity.device_id).bind(sensor).bind(event.observed_at).bind(m.value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Json(
        serde_json::json!({"event_id":event_id,"status":"processed","duplicate":false}),
    ))
}
async fn latest(
    State(db): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let identity = authenticate(&db, &headers).await?;
    let mut tx = tenant_tx(&db, identity.tenant_id).await?;
    let rows=sqlx::query("SELECT s.key,s.unit,l.value_number,l.observed_at,l.received_at FROM sensors s JOIN device_latest_values l ON (l.tenant_id,l.sensor_id)=(s.tenant_id,s.id) WHERE s.tenant_id=$1 AND s.device_id=$2 ORDER BY s.key LIMIT 128")
        .bind(identity.tenant_id).bind(identity.device_id).fetch_all(&mut *tx).await?;
    let values:Vec<_>=rows.iter().map(|r|serde_json::json!({"key":r.get::<String,_>("key"),"unit":r.get::<String,_>("unit"),"value":r.get::<f64,_>("value_number"),"observed_at":r.get::<chrono::DateTime<chrono::Utc>,_>("observed_at"),"received_at":r.get::<chrono::DateTime<chrono::Utc>,_>("received_at"),"quality":"unknown"})).collect();
    tx.commit().await?;
    Ok(Json(
        serde_json::json!({"tenant_id":identity.tenant_id,"device_id":identity.device_id,"measurements":values}),
    ))
}
