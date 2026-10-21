//! Integration tests for the room authorization rules.
//!
//! These tests exercise the shared authorisation module
//! (`maelstrom_core::matrix::auth_rules`) through the client-server API, so
//! they cover both the rules themselves and the wiring in the handlers.

mod common;

use http::StatusCode;
use maelstrom_core::matrix::event::Pdu;
use maelstrom_core::matrix::room::event_type as et;
use serde_json::json;

/// Create a room owned by `token` and return its room ID.
async fn create_room(router: &axum::Router, token: &str, preset: &str) -> String {
    let (status, resp) = common::post_json_authed(
        router,
        "/_matrix/client/v3/createRoom",
        &json!({ "preset": preset }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "createRoom failed: {resp}");
    serde_json::from_str::<serde_json::Value>(&resp).unwrap()["room_id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Invite `user_id` into `room_id` and join it.
async fn invite_and_join(router: &axum::Router, room_id: &str, inviter: &str, joiner: &str, user_id: &str) {
    let (status, resp) = common::post_json_authed(
        router,
        &format!("/_matrix/client/v3/rooms/{room_id}/invite"),
        &json!({ "user_id": user_id }),
        inviter,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "invite failed: {resp}");

    let (status, resp) = common::post_json_authed(
        router,
        &format!("/_matrix/client/v3/rooms/{room_id}/join"),
        &json!({}),
        joiner,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "join failed: {resp}");
}

// ---------------------------------------------------------------------------
// Kick / ban / unban
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ordinary_member_cannot_ban_another_member() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "ar_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "ar_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@ar_bob:localhost",
    )
    .await;

    // Bob has power level 0 — he may not ban anybody.
    let (status, resp) = common::post_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/ban"),
        &json!({ "user_id": "@ar_alice:localhost" }),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "ban should be refused: {resp}");
    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["errcode"], "M_FORBIDDEN");
}

#[tokio::test]
async fn ordinary_member_cannot_kick_another_member() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "ak_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "ak_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@ak_bob:localhost",
    )
    .await;

    let (status, resp) = common::post_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/kick"),
        &json!({ "user_id": "@ak_alice:localhost" }),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "kick should be refused: {resp}");
}

#[tokio::test]
async fn room_creator_can_ban_and_unban() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "ab_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "ab_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@ab_bob:localhost",
    )
    .await;

    let (status, resp) = common::post_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/ban"),
        &json!({ "user_id": "@ab_bob:localhost" }),
        &token_alice,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "ban failed: {resp}");

    let (status, resp) = common::post_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/unban"),
        &json!({ "user_id": "@ab_bob:localhost" }),
        &token_alice,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "unban failed: {resp}");
}

#[tokio::test]
async fn non_member_cannot_ban() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "an_alice", "pass").await;
    let (token_carol, _, _) = common::register_user(&router, "an_carol", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;

    // Carol is not in the room at all.
    let (status, resp) = common::post_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/ban"),
        &json!({ "user_id": "@an_alice:localhost" }),
        &token_carol,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "ban should be refused: {resp}");
}

// ---------------------------------------------------------------------------
// Membership transitions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn user_cannot_join_a_private_room_without_an_invite() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "pj_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "pj_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "private_chat").await;

    let (status, resp) = common::post_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/join"),
        &json!({}),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "join should be refused: {resp}");
}

#[tokio::test]
async fn user_cannot_set_own_membership_to_banned() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "sb_alice", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;

    // A user may not promote themselves to "ban" through the state API.
    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/state/m.room.member/@sb_alice:localhost"),
        &json!({ "membership": "ban" }),
        &token_alice,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "self-ban should be refused: {resp}");
}

#[tokio::test]
async fn member_cannot_set_another_users_membership() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "om_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "om_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@om_bob:localhost",
    )
    .await;

    // Bob (power level 0) may not kick Alice through the state API.
    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/state/m.room.member/@om_alice:localhost"),
        &json!({ "membership": "leave" }),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "kick should be refused: {resp}");
}

// ---------------------------------------------------------------------------
// Power levels
// ---------------------------------------------------------------------------

#[tokio::test]
async fn user_cannot_raise_power_levels_above_own_level() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "pl_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "pl_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@pl_bob:localhost",
    )
    .await;

    // Bob tries to promote himself to 100.
    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/state/m.room.power_levels/"),
        &json!({ "users": { "@pl_bob:localhost": 100 }, "users_default": 0 }),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "PL change should be refused: {resp}");
}

#[tokio::test]
async fn user_cannot_change_another_users_power_level() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "pu_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "pu_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@pu_bob:localhost",
    )
    .await;

    // Bob tries to demote Alice (power level 100).
    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/state/m.room.power_levels/"),
        &json!({ "users": { "@pu_alice:localhost": 0, "@pu_bob:localhost": 0 }, "users_default": 0 }),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "PL change should be refused: {resp}");
}

#[tokio::test]
async fn events_default_is_enforced_for_messages() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "ed_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "ed_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;
    invite_and_join(
        &router,
        &room_id,
        &token_alice,
        &token_bob,
        "@ed_bob:localhost",
    )
    .await;

    // Alice raises events_default to 50 — Bob (level 0) may no longer talk.
    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/state/m.room.power_levels/"),
        &json!({ "users": { "@ed_alice:localhost": 100 }, "users_default": 0, "events_default": 50 }),
        &token_alice,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "PL change failed: {resp}");

    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/send/m.room.message/txn1"),
        &json!({ "msgtype": "m.text", "body": "hello" }),
        &token_bob,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "message should be refused: {resp}"
    );
}

#[tokio::test]
async fn non_member_cannot_send_messages() {
    let router = common::test_router();
    let (token_alice, _, _) = common::register_user(&router, "nm_alice", "pass").await;
    let (token_bob, _, _) = common::register_user(&router, "nm_bob", "pass").await;

    let room_id = create_room(&router, &token_alice, "public_chat").await;

    let (status, resp) = common::put_json_authed(
        &router,
        &format!("/_matrix/client/v3/rooms/{room_id}/send/m.room.message/txn1"),
        &json!({ "msgtype": "m.text", "body": "hello" }),
        &token_bob,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "message should be refused: {resp}");
}

// ---------------------------------------------------------------------------
// The authorisation module itself
// ---------------------------------------------------------------------------

fn pdu(event_type: &str, state_key: Option<&str>, sender: &str, content: serde_json::Value) -> Pdu {
    Pdu {
        event_id: "$test".to_string(),
        room_id: "!room:localhost".to_string(),
        sender: sender.to_string(),
        event_type: event_type.to_string(),
        state_key: state_key.map(str::to_string),
        content,
        origin_server_ts: 0,
        unsigned: None,
        stream_position: 0,
        origin: None,
        auth_events: None,
        prev_events: None,
        depth: None,
        hashes: None,
        signatures: None,
    }
}

#[test]
fn auth_rules_reject_kick_without_power() {
    use maelstrom_core::matrix::auth_rules::{check, RoomState};

    let events = vec![
        pdu(
            et::MEMBER,
            Some("@alice:localhost"),
            "@alice:localhost",
            json!({ "membership": "join" }),
        ),
        pdu(
            et::MEMBER,
            Some("@bob:localhost"),
            "@bob:localhost",
            json!({ "membership": "join" }),
        ),
        pdu(
            et::POWER_LEVELS,
            Some(""),
            "@alice:localhost",
            json!({ "users": { "@alice:localhost": 50, "@bob:localhost": 0 } }),
        ),
    ];
    let state = RoomState::new("!room:localhost", "10", Some("@alice:localhost"), &events);

    let kick = pdu(
        et::MEMBER,
        Some("@alice:localhost"),
        "@bob:localhost",
        json!({ "membership": "leave" }),
    );
    assert!(check(&state, &kick).is_err());
}

#[test]
fn auth_rules_allow_moderator_kick() {
    use maelstrom_core::matrix::auth_rules::{check, RoomState};

    let events = vec![
        pdu(
            et::MEMBER,
            Some("@alice:localhost"),
            "@alice:localhost",
            json!({ "membership": "join" }),
        ),
        pdu(
            et::MEMBER,
            Some("@bob:localhost"),
            "@bob:localhost",
            json!({ "membership": "join" }),
        ),
        pdu(
            et::POWER_LEVELS,
            Some(""),
            "@alice:localhost",
            json!({ "users": { "@alice:localhost": 50, "@bob:localhost": 0 } }),
        ),
    ];
    let state = RoomState::new("!room:localhost", "10", Some("@alice:localhost"), &events);

    let kick = pdu(
        et::MEMBER,
        Some("@bob:localhost"),
        "@alice:localhost",
        json!({ "membership": "leave" }),
    );
    assert!(check(&state, &kick).is_ok());
}

#[test]
fn room_versions_gate_the_knock_membership() {
    use maelstrom_core::matrix::auth_rules::RoomVersion;

    assert!(!RoomVersion::parse("7").supports_knock());
    assert!(RoomVersion::parse("8").supports_knock());
    assert!(RoomVersion::parse("11").supports_restricted());
    assert!(!RoomVersion::parse("10").supports_restricted());
}
