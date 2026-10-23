#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = marivect::pool().await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    Ok(())
}
