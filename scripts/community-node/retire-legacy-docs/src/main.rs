use anyhow::{Result, ensure};
use iroh_docs::{
    NamespaceSecret,
    store::{Query, Store},
};
use std::{collections::BTreeMap, fs::OpenOptions, path::Path};

const TOPICS: [&str; 4] = ["demo", "iroh", "nostr", "operators"];

fn namespace(topic: &str) -> iroh_docs::NamespaceId {
    // Same public namespace derivation as docs-sync/src/replicas.rs; no endpoint is opened.
    NamespaceSecret::from_bytes(
        blake3::hash(format!("kukuri-docs:topic::kukuri:topic:{topic}").as_bytes()).as_bytes(),
    )
    .id()
}

fn snapshot(store: &mut Store) -> Result<BTreeMap<String, Vec<Vec<u8>>>> {
    let namespaces = store.list_namespaces()?.collect::<Result<Vec<_>>>()?;
    let mut result = BTreeMap::new();
    for (id, _) in namespaces {
        let mut entries = store
            .get_many(id, Query::all().include_empty())?
            .map(|entry| Ok(serde_json::to_vec(&entry?)?))
            .collect::<Result<Vec<_>>>()?;
        entries.sort();
        result.insert(id.to_string(), entries);
    }
    Ok(result)
}

fn retire(source: &Path, output: &Path, apply: bool) -> Result<serde_json::Value> {
    ensure!(
        source.is_file(),
        "source must be an existing stopped-store copy"
    );
    let original_hash = blake3::hash(&std::fs::read(source)?);
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    std::io::copy(&mut std::fs::File::open(source)?, &mut destination)?;
    destination.sync_all()?;
    drop(destination);
    let mut store = Store::persistent(output)?;
    let before = snapshot(&mut store)?;
    let authors_before = store
        .list_authors()?
        .map(|a| Ok(a?.to_bytes()))
        .collect::<Result<Vec<_>>>()?;
    let mut targets = Vec::new();
    for topic in TOPICS {
        let id = namespace(topic);
        let records = store
            .get_many(id, Query::all().include_empty())?
            .collect::<Result<Vec<_>>>()?;
        targets.push(serde_json::json!({"topic":format!("kukuri:topic:{topic}"),"namespace":id.to_string(),
            "present":before.contains_key(&id.to_string()),"records":records.iter().map(|e| serde_json::json!({
                "key":String::from_utf8_lossy(e.key()),"content_hash":e.content_hash().to_string()
            })).collect::<Vec<_>>()}));
        if apply {
            store.remove_replica(&id)?;
        }
    }
    store.flush()?;
    let authors_after = store
        .list_authors()?
        .map(|a| Ok(a?.to_bytes()))
        .collect::<Result<Vec<_>>>()?;
    ensure!(authors_before == authors_after, "authors changed");
    let mut expected = before.clone();
    if apply {
        for topic in TOPICS {
            expected.remove(&namespace(topic).to_string());
        }
    }
    ensure!(
        snapshot(&mut store)? == expected,
        "non-target entries changed"
    );
    drop(store);
    let mut reopened = Store::persistent(output)?;
    ensure!(
        snapshot(&mut reopened)? == expected,
        "reopen verification failed"
    );
    drop(reopened);
    ensure!(
        blake3::hash(&std::fs::read(source)?) == original_hash,
        "source changed"
    );
    Ok(
        serde_json::json!({"applied":apply,"targets":targets,"before_namespaces":before.len(),
        "after_namespaces":expected.len(),"non_target_entries_preserved":true,"authors_preserved":true,
        "source_unchanged":true}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 4 && ["inspect", "apply"].contains(&args[1].as_str()),
        "usage: retire-legacy-docs <inspect|apply> <stopped-copy.redb> <NEW-output.redb>"
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&retire(
            Path::new(&args[2]),
            Path::new(&args[3]),
            args[1] == "apply"
        )?)?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "current_thread")]
    async fn retirement_removes_only_four_namespaces_and_preserves_source() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("before.redb");
        let mut store = Store::persistent(&source)?;
        let author = iroh_docs::Author::from_bytes(&[7; 32]);
        store.import_author(author.clone())?;
        for topic in TOPICS.into_iter().chain(["general", "test", "dev"]) {
            let secret = NamespaceSecret::from_bytes(
                blake3::hash(format!("kukuri-docs:topic::kukuri:topic:{topic}").as_bytes())
                    .as_bytes(),
            );
            let mut replica = store.new_replica(secret)?;
            replica
                .insert(
                    b"objects/fixture/state",
                    &author,
                    "11".repeat(32).parse()?,
                    1,
                )
                .await?;
            replica.delete_prefix(b"withdrawn/", &author).await?;
        }
        store.flush()?;
        drop(store);
        let preview = retire(&source, &dir.path().join("preview.redb"), false)?;
        assert_eq!(preview["after_namespaces"], 7);
        let output = dir.path().join("after.redb");
        let applied = retire(&source, &output, true)?;
        assert_eq!(applied["after_namespaces"], 3);
        let mut after = Store::persistent(&output)?;
        assert!(
            snapshot(&mut after)?
                .values()
                .all(|entries| entries.len() == 2)
        );
        drop(after);
        let repeated = retire(&output, &dir.path().join("repeated.redb"), true)?;
        assert_eq!(repeated["after_namespaces"], 3);
        assert!(retire(&source, &output, true).is_err());
        assert!(retire(&source, &source, true).is_err());
        Ok(())
    }
    #[test]
    fn missing_source_is_not_created() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let missing = dir.path().join("missing.redb");
        let output = dir.path().join("output.redb");
        assert!(retire(&missing, &output, true).is_err());
        assert!(!missing.exists() && !output.exists());
        Ok(())
    }
}
