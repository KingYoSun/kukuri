//! WP-S6 T4【contract】row_mapping round-trip 層 — social 系(後続 WP-H1 の安全網)。
//!
//! 対象: profiles(sqlite/social.rs:74-97 / 123-150 のインライン写像 —
//! row_mapping.rs の外にあり列追加時に忘れやすい)/ follow_edges
//! (row_to_follow_edge + updated_at latest-wins)/ muted_authors
//! (row_to_muted_author)/ author_relationship_cache
//! (row_to_author_relationship_projection + rebuild の局所置換)。
//! 分割元の全体説明は row_mapping_roundtrip.rs の冒頭を参照。

use super::*;
use kukuri_core::{AssetRef, AssetRole, BlockEdge, BlockEdgeStatus, FollowEdge};

// ---------------------------------------------------------------------------
// profiles(sqlite/social.rs のインライン写像。picture_* 3 列を含む)
// ---------------------------------------------------------------------------

/// max fixture: 全 Option=Some。picture_asset は picture_blob_hash /
/// picture_mime / picture_bytes の 3 列に分解して永続化される。
fn profile_max() -> Profile {
    Profile {
        pubkey: "a".repeat(64).into(),
        name: Some("name-max".into()),
        display_name: Some("表示名 ✨".into()),
        about: Some("自己紹介 text".into()),
        picture: Some("https://example.invalid/avatar.png".into()),
        picture_asset: Some(AssetRef {
            hash: BlobHash::new("9".repeat(64)),
            mime: "image/webp".into(),
            bytes: 2_468,
            role: AssetRole::ProfileAvatar,
        }),
        updated_at: 1_700_000_004_000,
    }
}

/// min fixture: 全 Option=None。
fn profile_min() -> Profile {
    Profile {
        pubkey: "b".repeat(64).into(),
        name: None,
        display_name: None,
        about: None,
        picture: None,
        picture_asset: None,
        updated_at: 5,
    }
}

#[tokio::test]
async fn profile_roundtrip_preserves_all_columns_via_get_profile_and_get_profiles() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = profile_max();
    let min = profile_min();
    Store::upsert_profile(&store, max.clone())
        .await
        .expect("upsert max profile");
    Store::upsert_profile(&store, min.clone())
        .await
        .expect("upsert min profile");

    // 現挙動(観測値): sqlx-sqlite は NULL を TEXT→"" / INTEGER→0 に
    // デコードするため、全列 NULL の min fixture は Some("") で読み出され、
    // picture_asset も NULL の picture_blob_hash から
    // Some(AssetRef { hash: "", mime: "", bytes: 0 }) が再構築される。
    let expected_min = Profile {
        pubkey: min.pubkey.clone(),
        name: Some(String::new()),
        display_name: Some(String::new()),
        about: Some(String::new()),
        picture: Some(String::new()),
        picture_asset: Some(AssetRef {
            hash: BlobHash::new(""),
            mime: String::new(),
            bytes: 0,
            role: AssetRole::ProfileAvatar,
        }),
        updated_at: 5,
    };

    // get_profile(sqlite/social.rs:74-97 のインライン写像)
    assert_eq!(
        Store::get_profile(&store, max.pubkey.as_str())
            .await
            .expect("get max profile"),
        Some(max.clone())
    );
    assert_eq!(
        Store::get_profile(&store, min.pubkey.as_str())
            .await
            .expect("get min profile"),
        Some(expected_min.clone())
    );

    // get_profiles(sqlite/social.rs:123-150 の重複インライン写像)も同値を返す。
    let profiles = Store::get_profiles(
        &store,
        &[
            max.pubkey.as_str().to_string(),
            min.pubkey.as_str().to_string(),
            "c".repeat(64),
        ],
    )
    .await
    .expect("get profiles");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles.get(max.pubkey.as_str()), Some(&max));
    assert_eq!(profiles.get(min.pubkey.as_str()), Some(&expected_min));
}

#[tokio::test]
async fn profile_picture_asset_role_reads_back_as_profile_avatar() {
    // role 列は存在せず永続化されない。読み出し側は常に ProfileAvatar を再構築する。
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let mut profile = profile_max();
    profile.pubkey = "d".repeat(64).into();
    let mut asset = profile.picture_asset.clone().expect("fixture asset");
    asset.role = AssetRole::ImageOriginal;
    profile.picture_asset = Some(asset.clone());
    Store::upsert_profile(&store, profile.clone())
        .await
        .expect("upsert profile");

    let fetched = Store::get_profile(&store, profile.pubkey.as_str())
        .await
        .expect("get profile")
        .expect("profile exists");
    let fetched_asset = fetched.picture_asset.expect("picture asset");
    assert_eq!(fetched_asset.role, AssetRole::ProfileAvatar);
    assert_eq!(fetched_asset.hash, asset.hash);
    assert_eq!(fetched_asset.mime, asset.mime);
    assert_eq!(fetched_asset.bytes, asset.bytes);
}

#[tokio::test]
async fn profile_picture_mime_and_bytes_null_read_back_as_empty_and_zero() {
    // 既存 DB 互換: picture_blob_hash ありで picture_mime / picture_bytes が
    // NULL の行の読み出しを固定する。put 経由では作れないため生 SQL で NULL にする。
    //
    // 現挙動(観測値): sqlx-sqlite は NULL を TEXT→"" / INTEGER→0 に
    // デコードして try_get が Ok を返すため、sqlite/social.rs:74-97 の
    // "application/octet-stream" / unwrap_or_default フォールバックは
    // NULL 列では発火せず、mime は "" になる(フォールバックは実質デッドコード)。
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = profile_max();
    Store::upsert_profile(&store, max.clone())
        .await
        .expect("upsert profile");
    sqlx::query("UPDATE profiles SET picture_mime = NULL, picture_bytes = NULL WHERE pubkey = ?1")
        .bind(max.pubkey.as_str())
        .execute(store.pool())
        .await
        .expect("null out picture_mime/picture_bytes");

    let fetched = Store::get_profile(&store, max.pubkey.as_str())
        .await
        .expect("get profile")
        .expect("profile exists");
    assert_eq!(
        fetched.picture_asset,
        Some(AssetRef {
            hash: BlobHash::new("9".repeat(64)),
            mime: String::new(),
            bytes: 0,
            role: AssetRole::ProfileAvatar,
        })
    );
}

// ---------------------------------------------------------------------------
// follow_edges(row_to_follow_edge + updated_at latest-wins)
// 既存 sqlite_store.rs のテストは envelope 経由で status のみ検証 — ここでは
// 直接 upsert で全列 round-trip と latest-wins の境界(tie)を固定する。
// ---------------------------------------------------------------------------

fn follow_edge(
    subject: &str,
    target: &str,
    status: FollowEdgeStatus,
    updated_at: i64,
    envelope_id: &str,
) -> FollowEdge {
    FollowEdge {
        subject_pubkey: subject.into(),
        target_pubkey: target.into(),
        status,
        updated_at,
        envelope_id: EnvelopeId::from(envelope_id),
    }
}

#[tokio::test]
async fn follow_edge_roundtrip_preserves_all_columns_and_ordering() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let subject = "a".repeat(64);
    let target_active = "b".repeat(64);
    let target_revoked = "c".repeat(64);
    let active = follow_edge(
        &subject,
        &target_active,
        FollowEdgeStatus::Active,
        300,
        "env-follow-1",
    );
    let revoked = follow_edge(
        &subject,
        &target_revoked,
        FollowEdgeStatus::Revoked,
        200,
        "env-follow-2",
    );
    Store::upsert_follow_edge(&store, active.clone())
        .await
        .expect("upsert active edge");
    Store::upsert_follow_edge(&store, revoked.clone())
        .await
        .expect("upsert revoked edge");

    // 主キー updated_at DESC の順序ごと全列固定(副キー target_pubkey ASC の
    // tie-break はここでは未行使 — T6 の pagination テストと T8 の backend_parity が担保)。
    assert_eq!(
        Store::list_follow_edges_by_subject(&store, subject.as_str())
            .await
            .expect("list by subject"),
        vec![active.clone(), revoked]
    );
    assert_eq!(
        Store::list_follow_edges_by_target(&store, target_active.as_str())
            .await
            .expect("list by target"),
        vec![active]
    );
}

#[tokio::test]
async fn follow_edge_upsert_latest_wins_and_tie_overwrites() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let subject = "d".repeat(64);
    let target = "e".repeat(64);
    let current = follow_edge(
        &subject,
        &target,
        FollowEdgeStatus::Active,
        300,
        "env-current",
    );
    Store::upsert_follow_edge(&store, current.clone())
        .await
        .expect("upsert current edge");

    // 古い updated_at(299 < 300)は黙って無視される。
    let stale = follow_edge(
        &subject,
        &target,
        FollowEdgeStatus::Revoked,
        299,
        "env-stale",
    );
    Store::upsert_follow_edge(&store, stale)
        .await
        .expect("upsert stale edge");
    assert_eq!(
        Store::list_follow_edges_by_subject(&store, subject.as_str())
            .await
            .expect("list after stale"),
        vec![current]
    );

    // 同値 updated_at(300)は上書きされる(既存 > 新規のときだけ無視 = tie は新規勝ち)。
    let tie = follow_edge(&subject, &target, FollowEdgeStatus::Revoked, 300, "env-tie");
    Store::upsert_follow_edge(&store, tie.clone())
        .await
        .expect("upsert tie edge");
    assert_eq!(
        Store::list_follow_edges_by_subject(&store, subject.as_str())
            .await
            .expect("list after tie"),
        vec![tie]
    );

    // 新しい updated_at(301)は当然上書き。
    let newer = follow_edge(
        &subject,
        &target,
        FollowEdgeStatus::Active,
        301,
        "env-newer",
    );
    Store::upsert_follow_edge(&store, newer.clone())
        .await
        .expect("upsert newer edge");
    assert_eq!(
        Store::list_follow_edges_by_subject(&store, subject.as_str())
            .await
            .expect("list after newer"),
        vec![newer]
    );
}

#[tokio::test]
async fn block_edge_roundtrip_and_latest_wins() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let subject = "1".repeat(64);
    let target = "2".repeat(64);
    let active = BlockEdge {
        subject_pubkey: subject.as_str().into(),
        target_pubkey: target.as_str().into(),
        status: BlockEdgeStatus::Active,
        updated_at: 300,
        envelope_id: EnvelopeId::from("env-block-active"),
    };
    Store::upsert_block_edge(&store, active.clone())
        .await
        .expect("upsert active block");
    Store::upsert_block_edge(
        &store,
        BlockEdge {
            status: BlockEdgeStatus::Revoked,
            updated_at: 299,
            envelope_id: EnvelopeId::from("env-block-stale"),
            ..active.clone()
        },
    )
    .await
    .expect("ignore stale unblock");

    assert_eq!(
        Store::list_block_edges_by_subject(&store, subject.as_str())
            .await
            .expect("list by subject"),
        vec![active.clone()]
    );
    assert_eq!(
        Store::list_block_edges_by_target(&store, target.as_str())
            .await
            .expect("list by target"),
        vec![active]
    );
}

// ---------------------------------------------------------------------------
// muted_authors(row_to_muted_author)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn muted_author_roundtrip_preserves_all_columns_and_ordering() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let newer = MutedAuthorRow {
        author_pubkey: "a".repeat(64),
        muted_at: 200,
    };
    let older = MutedAuthorRow {
        author_pubkey: "b".repeat(64),
        muted_at: 100,
    };
    SocialProjectionStore::put_muted_author(&store, older.clone())
        .await
        .expect("put older muted");
    SocialProjectionStore::put_muted_author(&store, newer.clone())
        .await
        .expect("put newer muted");

    assert_eq!(
        SocialProjectionStore::get_muted_author(&store, newer.author_pubkey.as_str())
            .await
            .expect("get muted"),
        Some(newer.clone())
    );
    // 主キー muted_at DESC の順序ごと固定(副キー author_pubkey ASC の
    // tie-break はここでは未行使 — T6 の pagination テストと T8 の backend_parity が担保)。
    assert_eq!(
        SocialProjectionStore::list_muted_authors(&store)
            .await
            .expect("list muted"),
        vec![newer, older]
    );
}

// ---------------------------------------------------------------------------
// author_relationship_cache(row_to_author_relationship_projection)
// bool 4 列 + friend_of_friend_via_pubkeys_json + rebuild の局所置換を固定。
// ---------------------------------------------------------------------------

/// max fixture: bool 4 列すべて true・via_pubkeys 2 要素。
fn relationship_max(local: &str) -> AuthorRelationshipProjectionRow {
    AuthorRelationshipProjectionRow {
        local_author_pubkey: local.to_string(),
        author_pubkey: "1".repeat(64),
        following: true,
        followed_by: true,
        mutual: true,
        friend_of_friend: true,
        friend_of_friend_via_pubkeys: vec!["2".repeat(64), "3".repeat(64)],
        derived_at: 1_700_000_005_000,
    }
}

/// min fixture: bool 4 列すべて false・via_pubkeys 空 Vec。
fn relationship_min(local: &str) -> AuthorRelationshipProjectionRow {
    AuthorRelationshipProjectionRow {
        local_author_pubkey: local.to_string(),
        author_pubkey: "4".repeat(64),
        following: false,
        followed_by: false,
        mutual: false,
        friend_of_friend: false,
        friend_of_friend_via_pubkeys: Vec::new(),
        derived_at: 0,
    }
}

#[tokio::test]
async fn author_relationship_roundtrip_preserves_all_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let local = "a".repeat(64);
    let max = relationship_max(&local);
    let min = relationship_min(&local);
    SocialProjectionStore::rebuild_author_relationships(
        &store,
        local.as_str(),
        vec![max.clone(), min.clone()],
    )
    .await
    .expect("rebuild relationships");

    assert_eq!(
        SocialProjectionStore::get_author_relationship(
            &store,
            local.as_str(),
            max.author_pubkey.as_str()
        )
        .await
        .expect("get max relationship"),
        Some(max.clone())
    );
    assert_eq!(
        SocialProjectionStore::get_author_relationship(
            &store,
            local.as_str(),
            min.author_pubkey.as_str()
        )
        .await
        .expect("get min relationship"),
        Some(min.clone())
    );

    let listed = SocialProjectionStore::list_author_relationships(
        &store,
        local.as_str(),
        &[
            max.author_pubkey.clone(),
            min.author_pubkey.clone(),
            "f".repeat(64),
        ],
    )
    .await
    .expect("list relationships");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed.get(max.author_pubkey.as_str()), Some(&max));
    assert_eq!(listed.get(min.author_pubkey.as_str()), Some(&min));
}

#[tokio::test]
async fn author_relationship_rebuild_replaces_only_given_local_author() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let local_a = "a".repeat(64);
    let local_b = "b".repeat(64);
    let a_before = relationship_max(&local_a);
    let b_row = relationship_max(&local_b);
    SocialProjectionStore::rebuild_author_relationships(
        &store,
        local_a.as_str(),
        vec![a_before.clone()],
    )
    .await
    .expect("seed local_a");
    SocialProjectionStore::rebuild_author_relationships(
        &store,
        local_b.as_str(),
        vec![b_row.clone()],
    )
    .await
    .expect("seed local_b");

    let mut a_after = relationship_min(&local_a);
    a_after.author_pubkey = "5".repeat(64);
    SocialProjectionStore::rebuild_author_relationships(
        &store,
        local_a.as_str(),
        vec![a_after.clone()],
    )
    .await
    .expect("rebuild local_a");

    // local_a は丸ごと差し替え(旧行は消える)、local_b の行は無傷。
    assert_eq!(
        SocialProjectionStore::get_author_relationship(
            &store,
            local_a.as_str(),
            a_before.author_pubkey.as_str(),
        )
        .await
        .expect("get replaced row"),
        None
    );
    assert_eq!(
        SocialProjectionStore::get_author_relationship(
            &store,
            local_a.as_str(),
            a_after.author_pubkey.as_str(),
        )
        .await
        .expect("get new row"),
        Some(a_after)
    );
    assert_eq!(
        SocialProjectionStore::get_author_relationship(
            &store,
            local_b.as_str(),
            b_row.author_pubkey.as_str(),
        )
        .await
        .expect("get local_b row"),
        Some(b_row)
    );
}
