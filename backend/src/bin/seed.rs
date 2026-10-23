use uuid::Uuid;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    anyhow::ensure!(
        std::env::var("APP_ENV").as_deref() == Ok("development"),
        "demo seed is development-only"
    );
    let db = marivect::pool().await?;
    let mut tx = db.begin().await?;
    for (n, var) in [(1, "DEMO_TOKEN_A"), (2, "DEMO_TOKEN_B")] {
        let token = std::env::var(var)?;
        anyhow::ensure!(
            token.len() >= 32,
            "demo token must contain at least 32 characters"
        );
        let tenant = Uuid::from_u128(n);
        let device = Uuid::from_u128(100 + n);
        sqlx::query("INSERT INTO tenants(id,name) VALUES($1,$2) ON CONFLICT(id) DO NOTHING")
            .bind(tenant)
            .bind(format!("DEMO organizacija {n}"))
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT set_config('app.tenant_id',$1,true)")
            .bind(tenant.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO devices(id,tenant_id,name,capabilities) VALUES($1,$2,$3,ARRAY['telemetry.push']) ON CONFLICT(tenant_id,id) DO NOTHING")
            .bind(device).bind(tenant).bind(format!("SIMULATOR {n}")).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO sensors(id,tenant_id,device_id,key,unit) VALUES($1,$2,$3,'battery_voltage','V') ON CONFLICT(tenant_id,device_id,key) DO NOTHING")
            .bind(Uuid::from_u128(200+n)).bind(tenant).bind(device).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO source_credentials(tenant_id,device_id,token_hash) VALUES($1,$2,$3) ON CONFLICT(tenant_id,device_id) DO UPDATE SET token_hash=excluded.token_hash,revoked=false")
            .bind(tenant).bind(device).bind(marivect::hash(&token)).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    println!("Two demo sources created. No measurements have been simulated yet.");
    Ok(())
}
