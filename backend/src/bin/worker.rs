//! Intentionally idle: no fake polling, alarm processing or command execution.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    marivect::logging();
    let db = marivect::pool().await?;
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            _ = tick.tick() => { sqlx::query("SELECT 1").execute(&db).await?; tracing::info!("worker heartbeat; adapters pending implementation"); }
            _ = tokio::signal::ctrl_c() => break,
            _ = async { #[cfg(unix)] { terminate.recv().await; } #[cfg(not(unix))] { std::future::pending::<()>().await; } } => break,
        }
    }
    db.close().await;
    Ok(())
}
