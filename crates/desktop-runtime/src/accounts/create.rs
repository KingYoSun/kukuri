use super::*;
use crate::CreateAccountRequest;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PreparedAccountCreation {
    pub committed: bool,
    pub operation_id: String,
    pub previous: String,
    pub next: AccountRecord,
}

pub(crate) fn prepare_account_creation(
    dir: &Path,
    mode: IdentityStorageMode,
    request: &CreateAccountRequest,
) -> Result<PreparedAccountCreation> {
    let operation = uuid::Uuid::parse_str(&request.operation_id)?.to_string();
    let mut registry = load_registry(dir)?.ok_or_else(|| anyhow!("accounts not initialized"))?;
    if let Some(pending) = &registry.pending_creation {
        if pending.operation_id == operation && pending.previous == request.account_id {
            if pending.committed && registry.active_account_id != pending.next.id {
                bail!("account creation was already completed");
            }
            if registry.active_account_id != pending.previous
                && registry.active_account_id != pending.next.id
            {
                bail!("account creation request is stale");
            }
            verify_persisted_identity(
                &account_db_path(dir, &pending.next.id),
                mode,
                &pending.next.pubkey,
            )?;
            return Ok(pending.clone());
        }
    }
    if registry.active_account_id != request.account_id {
        bail!("account creation target is no longer active");
    }
    let staging = dir
        .join("account-transitions")
        .join(format!("create-{operation}"))
        .join(DB_FILE_NAME);
    let next = lifecycle::generate_account(dir, mode, &staging)?;
    if registry
        .accounts
        .iter()
        .any(|a| a.id == next.id || a.pubkey == next.pubkey)
    {
        bail!("account creation operation already registered");
    }
    let pending = PreparedAccountCreation {
        committed: false,
        operation_id: operation,
        previous: request.account_id.clone(),
        next,
    };
    registry.pending_creation = Some(pending.clone());
    save_registry(dir, &registry)?;
    Ok(pending)
}

pub(crate) fn commit_account_creation(dir: &Path, pending: &PreparedAccountCreation) -> Result<()> {
    let mut registry = load_registry(dir)?.ok_or_else(|| anyhow!("accounts not initialized"))?;
    if registry.pending_creation.as_ref() != Some(pending)
        || registry.active_account_id != pending.previous
    {
        bail!("account creation reservation is stale");
    }
    if registry.accounts.iter().any(|a| a.id == pending.next.id) {
        bail!("account already registered");
    }
    registry.accounts.push(pending.next.clone());
    registry.profile_setup.push(pending.next.id.clone());
    registry
        .history
        .retain(|id| id != &pending.previous && id != &pending.next.id);
    registry.history.insert(0, pending.previous.clone());
    registry.active_account_id = pending.next.id.clone();
    registry.pending_creation = Some(PreparedAccountCreation {
        committed: true,
        ..pending.clone()
    });
    save_registry(dir, &registry)
}
