use crate::state::{db::Database, repo};

pub async fn run(db: &Database) -> anyhow::Result<()> {
    match repo::account_status(db).await? {
        Some(status) => println!("google_account: {status}"),
        None => println!("google_account: not_connected"),
    }

    let job_counts = repo::job_status_counts(db).await?;
    if job_counts.is_empty() {
        println!("jobs: none");
    } else {
        println!("jobs:");
        for item in job_counts {
            println!("  {}: {}", item.status, item.count);
        }
    }

    println!("db_integrity: {}", db.integrity_check().await?);
    Ok(())
}
