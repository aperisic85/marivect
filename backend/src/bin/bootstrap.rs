//! Local administrator-only provisioning. Password arrives on stdin, never argv or .env.
use sqlx::Row;
use std::io::Read;
use uuid::Uuid;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let address = marivect::auth::email(&std::env::var("BOOTSTRAP_EMAIL")?)
        .ok_or_else(|| anyhow::anyhow!("invalid email"))?;
    let tenant: Uuid = std::env::var("BOOTSTRAP_TENANT_ID")?.parse()?;
    let role = std::env::var("BOOTSTRAP_ROLE").unwrap_or_else(|_| "tenant_admin".into());
    anyhow::ensure!(
        ["tenant_admin", "operator", "viewer"].contains(&role.as_str()),
        "invalid role"
    );
    let mut password = String::new();
    std::io::stdin().take(1026).read_to_string(&mut password)?;
    let password = password.trim_end_matches(['\r', '\n']);
    anyhow::ensure!(
        (12..=1024).contains(&password.len()),
        "password must have 12–1024 bytes"
    );
    let db = marivect::pool().await?;
    let mut tx = db.begin().await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenants WHERE id=$1)")
        .bind(tenant)
        .fetch_one(&mut *tx)
        .await?;
    anyhow::ensure!(
        exists,
        "tenant does not exist: seed first or supply an existing tenant UUID"
    );
    let user = sqlx::query("SELECT id,password_hash FROM users WHERE email=$1")
        .bind(&address)
        .fetch_optional(&mut *tx)
        .await?;
    let uid = if let Some(row) = user {
        anyhow::ensure!(
            marivect::auth::verify(password, &row.get::<String, _>("password_hash")),
            "existing user password does not match; not changed"
        );
        row.get::<Uuid, _>("id")
    } else {
        let uid = Uuid::new_v4();
        let encoded = marivect::auth::password_hash(password)?;
        sqlx::query("INSERT INTO users(id,email,password_hash) VALUES($1,$2,$3)")
            .bind(uid)
            .bind(&address)
            .bind(encoded)
            .execute(&mut *tx)
            .await?;
        uid
    };
    sqlx::query("INSERT INTO memberships(tenant_id,user_id,role) VALUES($1,$2,$3) ON CONFLICT(tenant_id,user_id) DO UPDATE SET role=excluded.role")
 .bind(tenant).bind(uid).bind(&role).execute(&mut *tx).await?;
    tx.commit().await?;
    println!("User {address} has role {role} in tenant {tenant}. Password not printed or stored locally.");
    Ok(())
}
