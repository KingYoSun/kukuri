use anyhow::Result;
pub use kukuri_cn_safety::provider::ScanReferenceGuard;

pub(crate) async fn check(guard: Option<&dyn ScanReferenceGuard>) -> Result<()> {
    if let Some(guard) = guard {
        guard.check().await?;
    }
    Ok(())
}
