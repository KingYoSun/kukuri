use anyhow::Result;
use sqlx::PgPool;

use kukuri_cn_core::{get_tester_feedback, list_tester_feedback};

use crate::TesterFeedbackAction;

pub(super) async fn run(pool: &PgPool, action: TesterFeedbackAction) -> Result<()> {
    match action {
        TesterFeedbackAction::List { limit, offset } => {
            let feedback = list_tester_feedback(pool, limit, offset).await?;
            if feedback.is_empty() {
                println!("no tester feedback");
            } else {
                println!(
                    "{} tester feedback item(s) (limit={} offset={}):",
                    feedback.len(),
                    limit,
                    offset
                );
                for entry in feedback {
                    println!(
                        "{}  {}  client_version={}  os={}",
                        entry.created_at.to_rfc3339(),
                        entry.id,
                        entry.client_version,
                        entry.os,
                    );
                    println!("  やろうとしたこと:     {}", entry.what_attempted);
                    println!("  何が起きたか:         {}", entry.what_happened);
                    println!("  何が変だと思ったか:   {}", entry.what_seemed_wrong);
                }
            }
        }
        TesterFeedbackAction::Show { id } => match get_tester_feedback(pool, id.as_str()).await? {
            Some(entry) => {
                println!("id:                 {}", entry.id);
                println!("created_at:         {}", entry.created_at.to_rfc3339());
                println!("client_version:     {}", entry.client_version);
                println!("os:                 {}", entry.os);
                println!("やろうとしたこと:   {}", entry.what_attempted);
                println!("何が起きたか:       {}", entry.what_happened);
                println!("何が変だと思ったか: {}", entry.what_seemed_wrong);
            }
            None => println!("tester feedback not found: {id}"),
        },
    }
    Ok(())
}
