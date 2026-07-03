//! WP-S6 T3: row_mapping の enum⇔文字列写像 14 関数の characterization テスト。
//!
//! ここで固定する文字列は SQLite に永続化される値そのもの(DB 契約)。後続 WP-H1
//! (ProjectionStore の sub-trait 分割 + sqlite/memory 委譲解消)の安全網として、
//! 「テストが落ちたら疑うのは変更でありテストではない」に耐える生リテラルで固定する。
//! 期待値の生成に *_name() / parse_*() を相互利用しない(tautology 禁止)。
//! 対象: crates/store/src/row_mapping.rs の
//! follow_edge_status / object_status / reaction_key_kind / notification_kind /
//! live_status の name+parse 各 2 = 10 関数、
//! game_status_name / game_room_kind_name / parse_game_room_kind / parse_game_status = 4 関数。

use crate::models::NotificationKind;
use crate::row_mapping::{
    follow_edge_status_name, game_room_kind_name, game_status_name, live_status_name,
    notification_kind_name, object_status_name, parse_follow_edge_status, parse_game_room_kind,
    parse_game_status, parse_live_status, parse_notification_kind, parse_object_status,
    parse_reaction_key_kind, reaction_key_kind_name,
};
use kukuri_core::{
    FollowEdgeStatus, GameRoomKind, GameRoomStatus, LiveSessionStatus, ObjectStatus,
    ReactionKeyKind,
};

// ---------------------------------------------------------------------------
// follow_edge_status
// ---------------------------------------------------------------------------

#[test]
fn follow_edge_status_name_maps_all_variants() {
    let cases: [(FollowEdgeStatus, &str); 2] = [
        (FollowEdgeStatus::Active, "active"),
        (FollowEdgeStatus::Revoked, "revoked"),
    ];
    for (variant, expected) in cases {
        assert_eq!(
            follow_edge_status_name(&variant),
            expected,
            "variant: {variant:?}"
        );
    }
}

#[test]
fn parse_follow_edge_status_maps_all_known_strings() {
    let cases: [(&str, FollowEdgeStatus); 2] = [
        ("active", FollowEdgeStatus::Active),
        ("revoked", FollowEdgeStatus::Revoked),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_follow_edge_status(input).expect("known value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_follow_edge_status_bails_on_unknown_values() {
    let err = parse_follow_edge_status("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown follow edge status: nonsense");
    // 空文字は別名を持たない(bail)。大文字小文字は区別される。
    assert!(parse_follow_edge_status("").is_err());
    assert!(parse_follow_edge_status("Active").is_err());
    assert!(parse_follow_edge_status("ACTIVE").is_err());
}

// ---------------------------------------------------------------------------
// object_status
// ---------------------------------------------------------------------------

#[test]
fn object_status_name_maps_all_variants() {
    let cases: [(ObjectStatus, &str); 4] = [
        (ObjectStatus::Active, "active"),
        (ObjectStatus::Edited, "edited"),
        (ObjectStatus::Deleted, "deleted"),
        (ObjectStatus::Tombstoned, "tombstoned"),
    ];
    for (variant, expected) in cases {
        assert_eq!(
            object_status_name(&variant),
            expected,
            "variant: {variant:?}"
        );
    }
}

#[test]
fn parse_object_status_maps_all_known_strings() {
    let cases: [(&str, ObjectStatus); 4] = [
        ("active", ObjectStatus::Active),
        ("edited", ObjectStatus::Edited),
        ("deleted", ObjectStatus::Deleted),
        ("tombstoned", ObjectStatus::Tombstoned),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_object_status(input).expect("known value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_object_status_bails_on_unknown_values() {
    let err = parse_object_status("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown object status: nonsense");
    assert!(parse_object_status("").is_err());
    assert!(parse_object_status("Active").is_err());
    // 正規形の近似タイポ形は受理しない(tombstone ≠ tombstoned)。
    assert!(parse_object_status("tombstone").is_err());
    // 別ドメインの実在永続値も混入しない(follow_edges.status の "revoked" —
    // parse_follow_edge_status 側の値。"active" は両ドメインで共有なのに対し、
    // こちらは object status としては未知でなければならない)。
    assert!(parse_object_status("revoked").is_err());
}

// ---------------------------------------------------------------------------
// reaction_key_kind
// ---------------------------------------------------------------------------

#[test]
fn reaction_key_kind_name_maps_all_variants() {
    let cases: [(ReactionKeyKind, &str); 2] = [
        (ReactionKeyKind::Emoji, "emoji"),
        (ReactionKeyKind::CustomAsset, "custom_asset"),
    ];
    for (variant, expected) in cases {
        assert_eq!(
            reaction_key_kind_name(&variant),
            expected,
            "variant: {variant:?}"
        );
    }
}

#[test]
fn parse_reaction_key_kind_maps_all_known_strings() {
    let cases: [(&str, ReactionKeyKind); 2] = [
        ("emoji", ReactionKeyKind::Emoji),
        ("custom_asset", ReactionKeyKind::CustomAsset),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_reaction_key_kind(input).expect("known value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_reaction_key_kind_bails_on_unknown_values() {
    let err = parse_reaction_key_kind("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown reaction key kind: nonsense");
    assert!(parse_reaction_key_kind("").is_err());
    // snake_case 以外の表記揺れは受理しない。
    assert!(parse_reaction_key_kind("customAsset").is_err());
    assert!(parse_reaction_key_kind("custom-asset").is_err());
}

// ---------------------------------------------------------------------------
// notification_kind
// ---------------------------------------------------------------------------

#[test]
fn notification_kind_name_maps_all_variants() {
    let cases: [(NotificationKind, &str); 6] = [
        (NotificationKind::Mention, "mention"),
        (NotificationKind::Reply, "reply"),
        (NotificationKind::Repost, "repost"),
        (NotificationKind::QuoteRepost, "quote_repost"),
        (NotificationKind::DirectMessage, "direct_message"),
        (NotificationKind::Followed, "followed"),
    ];
    for (variant, expected) in cases {
        assert_eq!(
            notification_kind_name(&variant),
            expected,
            "variant: {variant:?}"
        );
    }
}

#[test]
fn parse_notification_kind_maps_all_known_strings() {
    let cases: [(&str, NotificationKind); 6] = [
        ("mention", NotificationKind::Mention),
        ("reply", NotificationKind::Reply),
        ("repost", NotificationKind::Repost),
        ("quote_repost", NotificationKind::QuoteRepost),
        ("direct_message", NotificationKind::DirectMessage),
        ("followed", NotificationKind::Followed),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_notification_kind(input).expect("known value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_notification_kind_bails_on_unknown_values() {
    let err = parse_notification_kind("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown notification kind: nonsense");
    assert!(parse_notification_kind("").is_err());
    // "follow"(動詞形)や camelCase は受理しない。
    assert!(parse_notification_kind("follow").is_err());
    assert!(parse_notification_kind("quoteRepost").is_err());
}

// ---------------------------------------------------------------------------
// live_status
// ---------------------------------------------------------------------------

#[test]
fn live_status_name_maps_all_variants() {
    let cases: [(LiveSessionStatus, &str); 4] = [
        (LiveSessionStatus::Scheduled, "scheduled"),
        (LiveSessionStatus::Live, "live"),
        (LiveSessionStatus::Paused, "paused"),
        (LiveSessionStatus::Ended, "ended"),
    ];
    for (variant, expected) in cases {
        assert_eq!(live_status_name(&variant), expected, "variant: {variant:?}");
    }
}

#[test]
fn parse_live_status_maps_all_known_strings() {
    let cases: [(&str, LiveSessionStatus); 4] = [
        ("scheduled", LiveSessionStatus::Scheduled),
        ("live", LiveSessionStatus::Live),
        ("paused", LiveSessionStatus::Paused),
        ("ended", LiveSessionStatus::Ended),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_live_status(input).expect("known value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_live_status_bails_on_unknown_values() {
    let err = parse_live_status("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown live session status: nonsense");
    assert!(parse_live_status("").is_err());
    // game 側の永続値・legacy 別名は live 側へ漏れない。
    assert!(parse_live_status("waiting").is_err());
    assert!(parse_live_status("open").is_err());
}

// ---------------------------------------------------------------------------
// game_status(name は canonical のみ、parse は legacy 別名 3 種を受理)
// ---------------------------------------------------------------------------

#[test]
fn game_status_name_maps_all_variants_to_canonical_names() {
    let cases: [(GameRoomStatus, &str); 4] = [
        (GameRoomStatus::Waiting, "waiting"),
        (GameRoomStatus::Running, "running"),
        (GameRoomStatus::Paused, "paused"),
        (GameRoomStatus::Ended, "ended"),
    ];
    for (variant, expected) in cases {
        assert_eq!(game_status_name(&variant), expected, "variant: {variant:?}");
    }
}

#[test]
fn parse_game_status_maps_all_canonical_strings() {
    let cases: [(&str, GameRoomStatus); 4] = [
        ("waiting", GameRoomStatus::Waiting),
        ("running", GameRoomStatus::Running),
        ("paused", GameRoomStatus::Paused),
        ("ended", GameRoomStatus::Ended),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_game_status(input).expect("canonical value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_game_status_accepts_legacy_aliases() {
    // 旧 DB 行の互換読み出し: open / in_progress / finished(name 側は再生成しない)。
    let cases: [(&str, GameRoomStatus); 3] = [
        ("open", GameRoomStatus::Waiting),
        ("in_progress", GameRoomStatus::Running),
        ("finished", GameRoomStatus::Ended),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_game_status(input).expect("legacy alias must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_game_status_bails_on_unknown_values() {
    let err = parse_game_status("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown game room status: nonsense");
    // 空文字に別名はない(room_kind と異なる)。live 側の値も受理しない。
    assert!(parse_game_status("").is_err());
    assert!(parse_game_status("scheduled").is_err());
    assert!(parse_game_status("Waiting").is_err());
}

// ---------------------------------------------------------------------------
// game_room_kind(parse は空文字 → ScoreGame の legacy 別名を受理)
// ---------------------------------------------------------------------------

#[test]
fn game_room_kind_name_maps_all_variants() {
    let cases: [(GameRoomKind, &str); 2] = [
        (GameRoomKind::ScoreGame, "score_game"),
        (GameRoomKind::MetaverseRoom, "metaverse_room"),
    ];
    for (variant, expected) in cases {
        assert_eq!(
            game_room_kind_name(&variant),
            expected,
            "variant: {variant:?}"
        );
    }
}

#[test]
fn parse_game_room_kind_maps_all_known_strings() {
    let cases: [(&str, GameRoomKind); 2] = [
        ("score_game", GameRoomKind::ScoreGame),
        ("metaverse_room", GameRoomKind::MetaverseRoom),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_game_room_kind(input).expect("known value must parse"),
            expected,
            "input: {input:?}"
        );
    }
}

#[test]
fn parse_game_room_kind_accepts_empty_string_as_score_game() {
    // room_kind 列追加(20260527)以前の旧行は空文字 → ScoreGame として読む。
    assert_eq!(
        parse_game_room_kind("").expect("legacy empty string must parse"),
        GameRoomKind::ScoreGame
    );
}

#[test]
fn parse_game_room_kind_bails_on_unknown_values() {
    let err = parse_game_room_kind("nonsense").expect_err("unknown value must bail");
    assert_eq!(err.to_string(), "unknown game room kind: nonsense");
    // 空白のみは空文字別名に含まれない(trim しない)。
    assert!(parse_game_room_kind(" ").is_err());
    assert!(parse_game_room_kind("ScoreGame").is_err());
    assert!(parse_game_room_kind("metaverse").is_err());
}
