//! Room authorization rules ("auth rules").
//!
//! This module implements the Matrix room authorization rules as described in
//! the room-version fragments of the specification
//! (`content/rooms/fragments/v1-auth-rules.md`, `v3-auth-rules.md` and
//! `v8-auth-rules.md`).
//!
//! The rules are the heart of Matrix's permission model: every event that is
//! sent into a room — by a local client or by a remote server — must be
//! authorised against the *current state* of that room.  Implementing them in
//! one shared place is essential: the client-server API and the federation
//! receiver must agree, otherwise a server will accept events over the
//! federation API that it would have rejected locally (and vice versa).
//!
//! # Usage
//!
//! ```rust,ignore
//! let state = RoomState::new(room_id, room_version, creator, current_state);
//! match auth_rules::check(&state, &candidate_event) {
//!     Ok(()) => { /* store the event */ }
//!     Err(e) => return Err(MatrixError::forbidden(e.message())),
//! }
//! ```
//!
//! # Room version awareness
//!
//! * Room versions 1–2 use the same authorisation rules as v3 (the only
//!   difference is event-ID signing, which is not an authorisation rule).
//! * Room versions 3–7 add the `knock` membership value.
//! * Room versions 8+ add the `restricted` / `knock_restricted` join rules and
//!   the `join_authorised_via_users_server` content property.
//!
//! Rules that depend on a version are gated on [`RoomVersion::supports_knock`]
//! and friends so that older rooms keep their historical behaviour.

use serde_json::Value;

use crate::matrix::event::Pdu;
use crate::matrix::room::event_type as et;

/// Default power levels, as documented for `m.room.power_levels`.
///
/// * `users_default` — 0
/// * `events_default` — 0
/// * `state_default` — 50
/// * `invite` — 0
/// * `kick` — 50
/// * `ban` — 50
/// * `redact` — 50
pub const DEFAULT_USERS: i64 = 0;
pub const DEFAULT_EVENTS: i64 = 0;
pub const DEFAULT_STATE: i64 = 50;
pub const DEFAULT_INVITE: i64 = 0;
pub const DEFAULT_KICK: i64 = 50;
pub const DEFAULT_BAN: i64 = 50;
pub const DEFAULT_REDACT: i64 = 50;

/// The power level of the room creator when the room has no
/// `m.room.power_levels` event (room versions 1–11).
pub const CREATOR_DEFAULT_POWER: i64 = 100;

/// The reason an event was rejected by the authorisation rules.
///
/// Every rejection maps to the spec's `M_FORBIDDEN` error code; the variants
/// exist to give operators (and tests) a precise reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// The sender is not in the room (or not joined) but the event type
    /// requires membership.
    NotJoined(String),
    /// The sender's power level is too low for the action.
    InsufficientPower(String),
    /// The event is not allowed for any other reason (join rules, bans,
    /// invalid content, ...).
    Forbidden(String),
}

impl AuthError {
    /// The Matrix error code to return to the client.
    pub fn errcode(&self) -> &'static str {
        "M_FORBIDDEN"
    }

    /// A human-readable description of the rejection.
    pub fn message(&self) -> &str {
        match self {
            Self::NotJoined(msg) | Self::InsufficientPower(msg) | Self::Forbidden(msg) => msg,
        }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// A parsed room version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoomVersion(pub u32);

impl RoomVersion {
    /// Parse a room version string such as `"1"` or `"11"`.
    ///
    /// Unknown or malformed versions fall back to the oldest rules (v1),
    /// which are a subset of the newer ones.
    pub fn parse(version: &str) -> Self {
        let trimmed = version.trim();
        // Room versions are written without a "v" prefix, but be lenient.
        let digits = trimmed.trim_start_matches(['v', 'V']);
        match digits.parse::<u32>() {
            Ok(n) => Self(n),
            Err(_) => Self(1),
        }
    }

    /// The numeric version.
    pub fn number(&self) -> u32 {
        self.0
    }

    /// `knock` membership and the `knock` join rule exist from room version 8.
    pub fn supports_knock(&self) -> bool {
        self.0 >= 8
    }

    /// `restricted` / `knock_restricted` join rules exist from room version 8.
    pub fn supports_restricted(&self) -> bool {
        self.0 >= 8
    }
}

/// The effective power levels of a room.
///
/// Defaults are filled in for every value the `m.room.power_levels` event
/// does not specify.
#[derive(Debug, Clone)]
pub struct PowerLevels {
    pub users_default: i64,
    pub events_default: i64,
    pub state_default: i64,
    pub invite: i64,
    pub kick: i64,
    pub ban: i64,
    pub redact: i64,
    /// Per-user power levels (`users`).
    pub users: serde_json::Map<String, Value>,
    /// Per-event-type power levels (`events`).
    pub events: serde_json::Map<String, Value>,
    /// Per-notification-type power levels (`notifications`).
    pub notifications: serde_json::Map<String, Value>,
}

impl Default for PowerLevels {
    fn default() -> Self {
        Self {
            users_default: DEFAULT_USERS,
            events_default: DEFAULT_EVENTS,
            state_default: DEFAULT_STATE,
            invite: DEFAULT_INVITE,
            kick: DEFAULT_KICK,
            ban: DEFAULT_BAN,
            redact: DEFAULT_REDACT,
            users: serde_json::Map::new(),
            events: serde_json::Map::new(),
            notifications: serde_json::Map::new(),
        }
    }
}

impl PowerLevels {
    /// Build the power levels described by an `m.room.power_levels` event.
    pub fn from_event(event: &Pdu) -> Self {
        let mut levels = Self::default();
        let content = &event.content;

        if let Some(v) = content.get("users_default").and_then(as_i64) {
            levels.users_default = v;
        }
        if let Some(v) = content.get("events_default").and_then(as_i64) {
            levels.events_default = v;
        }
        if let Some(v) = content.get("state_default").and_then(as_i64) {
            levels.state_default = v;
        }
        if let Some(v) = content.get("invite").and_then(as_i64) {
            levels.invite = v;
        }
        if let Some(v) = content.get("kick").and_then(as_i64) {
            levels.kick = v;
        }
        if let Some(v) = content.get("ban").and_then(as_i64) {
            levels.ban = v;
        }
        if let Some(v) = content.get("redact").and_then(as_i64) {
            levels.redact = v;
        }
        if let Some(map) = content.get("users").and_then(|v| v.as_object()) {
            levels.users = map.clone();
        }
        if let Some(map) = content.get("events").and_then(|v| v.as_object()) {
            levels.events = map.clone();
        }
        if let Some(map) = content.get("notifications").and_then(|v| v.as_object()) {
            levels.notifications = map.clone();
        }

        levels
    }

    /// The power level of `user_id`.
    ///
    /// When the room has no `m.room.power_levels` event at all, the room
    /// creator has level 100 and everybody else 0 (room versions 1–11).
    pub fn user_level(&self, user_id: &str, creator: Option<&str>) -> i64 {
        if let Some(v) = self.users.get(user_id).and_then(as_i64) {
            return v;
        }
        if self.users.is_empty()
            && self.users_default == DEFAULT_USERS
            && let Some(creator) = creator
            && creator == user_id
        {
            return CREATOR_DEFAULT_POWER;
        }
        self.users_default
    }

    /// The power level required to send an event of the given type.
    ///
    /// State events fall back to `state_default`, other events to
    /// `events_default`.
    pub fn required_for(&self, event_type: &str, is_state: bool) -> i64 {
        if let Some(v) = self.events.get(event_type).and_then(as_i64) {
            return v;
        }
        if is_state {
            self.state_default
        } else {
            self.events_default
        }
    }
}

/// Read an integer power level, accepting both JSON numbers and integer
/// strings (the spec allows the latter for backwards compatibility).
fn as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}

/// The current state of a room, as seen by the authorisation rules.
pub struct RoomState<'a> {
    /// The room ID (`!opaque:server`).
    pub room_id: &'a str,
    /// The room version (e.g. `"10"`).
    pub room_version: RoomVersion,
    /// The room creator's user ID, if known.
    pub creator: Option<&'a str>,
    /// The current state events of the room.
    pub events: &'a [Pdu],
}

impl<'a> RoomState<'a> {
    /// Build a room state view.
    pub fn new(
        room_id: &'a str,
        room_version: &str,
        creator: Option<&'a str>,
        events: &'a [Pdu],
    ) -> Self {
        Self {
            room_id,
            room_version: RoomVersion::parse(room_version),
            creator,
            events,
        }
    }

    /// Look up a state event by type and state key.
    pub fn state_event(&self, event_type: &str, state_key: &str) -> Option<&'a Pdu> {
        self.events.iter().find(|e| {
            e.event_type == event_type && e.state_key.as_deref().unwrap_or("") == state_key
        })
    }

    /// The current membership of a user in this room, if any.
    pub fn membership(&self, user_id: &str) -> Option<&'a str> {
        self.state_event(et::MEMBER, user_id)
            .and_then(|e| e.content.get("membership"))
            .and_then(|v| v.as_str())
    }

    /// The room's join rule (`public`, `invite`, `knock`, `restricted`, ...).
    pub fn join_rule(&self) -> &'a str {
        self.state_event(et::JOIN_RULES, "")
            .and_then(|e| e.content.get("join_rule"))
            .and_then(|v| v.as_str())
            .unwrap_or("invite")
    }

    /// The room's power levels.
    ///
    /// When the room has no `m.room.power_levels` event, the documented
    /// defaults apply (creator at 100, everybody else at 0).
    pub fn power_levels(&self) -> PowerLevels {
        match self.state_event(et::POWER_LEVELS, "") {
            Some(ev) => PowerLevels::from_event(ev),
            None => PowerLevels::default(),
        }
    }

    /// The power level of a user in this room.
    pub fn user_power_level(&self, user_id: &str) -> i64 {
        let levels = self.power_levels();
        let has_pl_event = self.state_event(et::POWER_LEVELS, "").is_some();
        if has_pl_event {
            levels.user_level(user_id, self.creator)
        } else {
            // No power levels event: the creator has 100, everybody else 0.
            if self.creator == Some(user_id) {
                CREATOR_DEFAULT_POWER
            } else {
                DEFAULT_USERS
            }
        }
    }

    /// Whether the user is currently banned from the room.
    pub fn is_banned(&self, user_id: &str) -> bool {
        self.membership(user_id) == Some("ban")
    }
}

/// Check whether `event` is allowed to be sent into the room described by
/// `room`.
///
/// This implements the authorisation rules of the specification
/// (room versions 1–12).  On success the event may be stored; on failure the
/// [`AuthError`] explains why it must be rejected.
pub fn check(room: &RoomState<'_>, event: &Pdu) -> Result<(), AuthError> {
    let event_type = event.event_type.as_str();
    let state_key = event.state_key.as_deref().unwrap_or("");
    let sender = event.sender.as_str();

    // Rule 1: m.room.create
    if event_type == et::CREATE {
        return check_create(room, event);
    }

    // Rule 4: m.room.member
    if event_type == et::MEMBER {
        return check_member(room, event);
    }

    // Rule 6: the sender must be joined to send anything else.
    let sender_membership = room.membership(sender).unwrap_or("leave");
    if sender_membership != "join" {
        return Err(AuthError::NotJoined(format!(
            "User {sender} is not joined to the room (membership: {sender_membership})"
        )));
    }

    // Rule 7: m.room.third_party_invite
    if event_type == et::THIRD_PARTY_INVITE {
        let levels = room.power_levels();
        if room.user_power_level(sender) >= levels.invite {
            return Ok(());
        }
        return Err(AuthError::InsufficientPower(format!(
            "Sender power level {} is below the invite level {}",
            room.user_power_level(sender),
            levels.invite
        )));
    }

    // Rule 8: the event type's required power level.
    //
    // State events whose state key is the sender's own user ID use
    // `users_default` instead of `state_default` (so that a user can always
    // update their own state, e.g. their own member event).
    let levels = room.power_levels();
    let is_state = event.state_key.is_some();
    let required = if is_state && state_key == sender {
        levels
            .events
            .get(event_type)
            .and_then(as_i64)
            .unwrap_or(levels.users_default)
    } else {
        levels.required_for(event_type, is_state)
    };
    let sender_level = room.user_power_level(sender);
    if sender_level < required {
        return Err(AuthError::InsufficientPower(format!(
            "Sender power level {sender_level} is below the required level {required} for {event_type}"
        )));
    }

    // Rule 9: state keys that look like user IDs must belong to the sender.
    if state_key.starts_with('@') && state_key != sender {
        return Err(AuthError::Forbidden(format!(
            "Cannot send {event_type} with another user's state key"
        )));
    }

    // Rule 10: m.room.power_levels
    if event_type == et::POWER_LEVELS {
        return check_power_levels(room, event);
    }

    Ok(())
}

/// Rule 1: `m.room.create`.
fn check_create(room: &RoomState<'_>, event: &Pdu) -> Result<(), AuthError> {
    if event.prev_events.as_ref().is_some_and(|p| !p.is_empty()) {
        return Err(AuthError::Forbidden(
            "m.room.create event must not have prev_events".to_string(),
        ));
    }

    // The room ID's server must match the sender's server.
    if let (Some(room_server), Some(sender_server)) = (
        server_of(room.room_id),
        server_of(&event.sender),
    ) && room_server != sender_server
    {
        return Err(AuthError::Forbidden(format!(
            "Room ID server {room_server} does not match sender server {sender_server}"
        )));
    }

    if event.content.get("room_version").is_some()
        && event.content.get("room_version").and_then(|v| v.as_str()).is_none()
    {
        return Err(AuthError::Forbidden(
            "m.room.create room_version must be a string".to_string(),
        ));
    }

    // Room versions 12+ removed the `creator` property; earlier versions
    // require it.
    if room.room_version.number() < 12 && event.content.get("creator").is_none() {
        return Err(AuthError::Forbidden(
            "m.room.create event must have a creator property".to_string(),
        ));
    }

    Ok(())
}

/// Rule 4: `m.room.member`.
fn check_member(room: &RoomState<'_>, event: &Pdu) -> Result<(), AuthError> {
    let state_key = event.state_key.as_deref().unwrap_or("");
    let sender = event.sender.as_str();

    if state_key.is_empty() {
        return Err(AuthError::Forbidden(
            "m.room.member event must have a state_key".to_string(),
        ));
    }

    let membership = event
        .content
        .get("membership")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if membership.is_empty() {
        // Not a membership change (e.g. a profile update). Only the user
        // themselves may send such an event, and only if they are joined.
        if state_key != sender {
            return Err(AuthError::Forbidden(
                "Only the user may modify their own membership event".to_string(),
            ));
        }
        let sender_membership = room.membership(sender).unwrap_or("leave");
        if sender_membership != "join" {
            return Err(AuthError::NotJoined(format!(
                "User {sender} is not joined to the room"
            )));
        }
        return Ok(());
    }

    let sender_membership = room.membership(sender).unwrap_or("leave");
    let target_membership = room.membership(state_key).unwrap_or("leave");
    let levels = room.power_levels();
    let sender_level = room.user_power_level(sender);
    let target_level = room.user_power_level(state_key);

    match membership {
        "join" => {
            // Only the user themselves may send their own join event.
            if sender != state_key {
                return Err(AuthError::Forbidden(
                    "A user may only send their own join event".to_string(),
                ));
            }

            // The creator of a room may always join it.
            if target_membership == "ban" {
                return Err(AuthError::Forbidden(
                    "User is banned from the room".to_string(),
                ));
            }

            let join_rule = room.join_rule();
            match join_rule {
                "public" => Ok(()),
                "invite" => {
                    if target_membership == "invite" || target_membership == "join" {
                        Ok(())
                    } else {
                        Err(AuthError::Forbidden(
                            "Cannot join a room with an invite join rule without an invite".to_string(),
                        ))
                    }
                }
                "knock" if room.room_version.supports_knock() => {
                    if target_membership == "invite"
                        || target_membership == "join"
                        || target_membership == "knock"
                    {
                        Ok(())
                    } else {
                        Err(AuthError::Forbidden(
                            "Cannot join a room with a knock join rule without knocking".to_string(),
                        ))
                    }
                }
                "restricted" | "knock_restricted" if room.room_version.supports_restricted() => {
                    if target_membership == "invite" || target_membership == "join" {
                        Ok(())
                    } else {
                        Err(AuthError::Forbidden(
                            "Cannot join a restricted room without an invite".to_string(),
                        ))
                    }
                }
                _ => {
                    if target_membership == "join" {
                        // Re-joining a room the user is already in is allowed
                        // for any join rule.
                        Ok(())
                    } else {
                        Err(AuthError::Forbidden(format!(
                            "Cannot join a room with join rule {join_rule}"
                        )))
                    }
                }
            }
        }

        "invite" => {
            // Third-party invites have their own (signature based) rules; the
            // regular path requires the inviter to be joined.
            if event.content.get("third_party_invite").is_some() {
                return check_third_party_invite(room, event);
            }

            if sender_membership != "join" {
                return Err(AuthError::NotJoined(format!(
                    "User {sender} is not joined to the room and cannot invite"
                )));
            }

            if target_membership == "join" {
                return Err(AuthError::Forbidden(
                    "Target user is already joined to the room".to_string(),
                ));
            }
            if target_membership == "ban" {
                return Err(AuthError::Forbidden(
                    "Target user is banned from the room".to_string(),
                ));
            }

            if sender_level >= levels.invite {
                Ok(())
            } else {
                Err(AuthError::InsufficientPower(format!(
                    "Sender power level {sender_level} is below the invite level {}",
                    levels.invite
                )))
            }
        }

        "leave" => {
            // Leaving by yourself.
            if sender == state_key {
                return match target_membership {
                    "invite" | "join" => Ok(()),
                    "knock" if room.room_version.supports_knock() => Ok(()),
                    "ban" => Err(AuthError::Forbidden(
                        "Banned users cannot leave the room".to_string(),
                    )),
                    _ => Err(AuthError::Forbidden(
                        "User is not in the room and cannot leave".to_string(),
                    )),
                };
            }

            // Otherwise this is a kick (or an unban).
            if sender_membership != "join" {
                return Err(AuthError::NotJoined(format!(
                    "User {sender} is not joined to the room and cannot kick"
                )));
            }

            // Unbanning requires the ban level.
            if target_membership == "ban" && sender_level < levels.ban {
                return Err(AuthError::InsufficientPower(format!(
                    "Sender power level {sender_level} is below the ban level {}",
                    levels.ban
                )));
            }

            if sender_level >= levels.kick && target_level < sender_level {
                Ok(())
            } else {
                Err(AuthError::InsufficientPower(format!(
                    "Sender power level {sender_level} is below the kick level {} or not above the target's level {target_level}",
                    levels.kick
                )))
            }
        }

        "ban" => {
            if sender_membership != "join" {
                return Err(AuthError::NotJoined(format!(
                    "User {sender} is not joined to the room and cannot ban"
                )));
            }

            if sender_level >= levels.ban && target_level < sender_level {
                Ok(())
            } else {
                Err(AuthError::InsufficientPower(format!(
                    "Sender power level {sender_level} is below the ban level {} or not above the target's level {target_level}",
                    levels.ban
                )))
            }
        }

        "knock" if room.room_version.supports_knock() => {
            let join_rule = room.join_rule();
            if join_rule != "knock" && join_rule != "knock_restricted" {
                return Err(AuthError::Forbidden(format!(
                    "Cannot knock on a room with join rule {join_rule}"
                )));
            }
            if sender != state_key {
                return Err(AuthError::Forbidden(
                    "A user may only knock on their own behalf".to_string(),
                ));
            }
            match target_membership {
                "ban" | "invite" | "join" => Err(AuthError::Forbidden(
                    "User has already interacted with the room".to_string(),
                )),
                _ => Ok(()),
            }
        }

        other => Err(AuthError::Forbidden(format!(
            "Unknown membership value {other}"
        ))),
    }
}

/// The `third_party_invite` branch of the membership rules.
fn check_third_party_invite(room: &RoomState<'_>, event: &Pdu) -> Result<(), AuthError> {
    let state_key = event.state_key.as_deref().unwrap_or("");

    if room.is_banned(state_key) {
        return Err(AuthError::Forbidden(
            "Target user is banned from the room".to_string(),
        ));
    }

    let third_party = event
        .content
        .get("third_party_invite")
        .ok_or_else(|| AuthError::Forbidden("Missing third_party_invite content".to_string()))?;

    let signed = third_party
        .get("signed")
        .ok_or_else(|| AuthError::Forbidden("Missing third_party_invite.signed".to_string()))?;

    let mxid = signed
        .get("mxid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuthError::Forbidden("Missing third_party_invite.signed.mxid".to_string()))?;
    let token = signed
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuthError::Forbidden("Missing third_party_invite.signed.token".to_string()))?;

    if mxid != state_key {
        return Err(AuthError::Forbidden(
            "third_party_invite.signed.mxid does not match the state key".to_string(),
        ));
    }

    let invite_event = room.state_event(et::THIRD_PARTY_INVITE, token).ok_or_else(|| {
        AuthError::Forbidden("No matching m.room.third_party_invite event".to_string())
    })?;

    if invite_event.sender != event.sender {
        return Err(AuthError::Forbidden(
            "Sender does not match the m.room.third_party_invite sender".to_string(),
        ));
    }

    Ok(())
}

/// Rule 10: changes to `m.room.power_levels`.
fn check_power_levels(room: &RoomState<'_>, event: &Pdu) -> Result<(), AuthError> {
    let sender = event.sender.as_str();
    let sender_level = room.user_power_level(sender);

    // The `users` property must be an object of integer values.
    if let Some(users) = event.content.get("users") {
        let Some(map) = users.as_object() else {
            return Err(AuthError::Forbidden(
                "m.room.power_levels users must be an object".to_string(),
            ));
        };
        for (user_id, value) in map {
            if !user_id.starts_with('@') {
                return Err(AuthError::Forbidden(format!(
                    "Invalid user ID {user_id} in m.room.power_levels"
                )));
            }
            if as_i64(value).is_none() {
                return Err(AuthError::Forbidden(format!(
                    "Invalid power level for {user_id} in m.room.power_levels"
                )));
            }
        }
    }

    let old = room.state_event(et::POWER_LEVELS, "");
    let Some(old) = old else {
        // No previous power levels event: the first one is unrestricted.
        return Ok(());
    };
    let old_levels = PowerLevels::from_event(old);

    // Rules 10.3: the scalar levels may not be raised above the sender's
    // level, and existing values above the sender's level may not be kept.
    for name in [
        "users_default",
        "events_default",
        "state_default",
        "ban",
        "redact",
        "kick",
        "invite",
    ] {
        let old_value = old
            .content
            .get(name)
            .and_then(as_i64)
            .unwrap_or(default_for_level(name));
        let new_value = event
            .content
            .get(name)
            .and_then(as_i64)
            .unwrap_or(default_for_level(name));

        if old_value != new_value {
            if old_value > sender_level {
                return Err(AuthError::InsufficientPower(format!(
                    "Cannot change {name}: current value {old_value} is above the sender's power level {sender_level}"
                )));
            }
            if new_value > sender_level {
                return Err(AuthError::InsufficientPower(format!(
                    "Cannot set {name} to {new_value}: above the sender's power level {sender_level}"
                )));
            }
        }
    }

    // Rules 10.4 / 10.5: the `events` map.
    check_level_map(
        &old_levels.events,
        event.content.get("events").and_then(|v| v.as_object()),
        sender_level,
        "events",
    )?;

    // Rules 10.4 / 10.5: the `notifications` map (room version 8+).
    check_level_map(
        &old_levels.notifications,
        event.content.get("notifications").and_then(|v| v.as_object()),
        sender_level,
        "notifications",
    )?;

    // Rule 10.6 / 10.7: the `users` map.
    let new_users = event.content.get("users").and_then(|v| v.as_object());
    if let Some(new_users) = new_users {
        for (user_id, new_value) in new_users {
            let new_level = as_i64(new_value).unwrap_or(DEFAULT_USERS);
            let old_level = old_levels.users.get(user_id).and_then(as_i64);

            match old_level {
                Some(old_level) if old_level != new_level => {
                    if user_id != sender && old_level >= sender_level {
                        return Err(AuthError::InsufficientPower(format!(
                            "Cannot change the power level of {user_id}: current level {old_level} is not below the sender's power level {sender_level}"
                        )));
                    }
                    if new_level > sender_level {
                        return Err(AuthError::InsufficientPower(format!(
                            "Cannot set the power level of {user_id} to {new_level}: above the sender's power level {sender_level}"
                        )));
                    }
                }
                Some(_) => {}
                None => {
                    if new_level > sender_level {
                        return Err(AuthError::InsufficientPower(format!(
                            "Cannot set the power level of {user_id} to {new_level}: above the sender's power level {sender_level}"
                        )));
                    }
                }
            }
        }
    }

    // Removing a user from the `users` map is a change too: the old value may
    // not be above the sender's level unless it is the sender's own entry.
    for (user_id, old_value) in &old_levels.users {
        let still_present = new_users
            .and_then(|m| m.get(user_id))
            .and_then(as_i64)
            .is_some();
        if !still_present {
            let old_level = as_i64(old_value).unwrap_or(DEFAULT_USERS);
            if user_id != sender && old_level >= sender_level {
                return Err(AuthError::InsufficientPower(format!(
                    "Cannot remove the power level of {user_id}: current level {old_level} is not below the sender's power level {sender_level}"
                )));
            }
        }
    }

    Ok(())
}

/// Compare an old and a new per-event-type power level map.
fn check_level_map(
    old: &serde_json::Map<String, Value>,
    new: Option<&serde_json::Map<String, Value>>,
    sender_level: i64,
    what: &str,
) -> Result<(), AuthError> {
    let empty = serde_json::Map::new();
    let new = new.unwrap_or(&empty);

    // Removed or changed entries: the old value may not exceed the sender's.
    for (key, old_value) in old {
        let old_level = as_i64(old_value).unwrap_or(DEFAULT_EVENTS);
        let new_level = new.get(key).and_then(as_i64);
        if new_level != Some(old_level) && old_level > sender_level {
            return Err(AuthError::InsufficientPower(format!(
                "Cannot change {what}.{key}: current value {old_level} is above the sender's power level {sender_level}"
            )));
        }
    }

    // Added or changed entries: the new value may not exceed the sender's.
    for (key, new_value) in new {
        let new_level = as_i64(new_value).unwrap_or(DEFAULT_EVENTS);
        let old_level = old.get(key).and_then(as_i64);
        if old_level != Some(new_level) && new_level > sender_level {
            return Err(AuthError::InsufficientPower(format!(
                "Cannot set {what}.{key} to {new_level}: above the sender's power level {sender_level}"
            )));
        }
    }

    Ok(())
}

/// The documented default for a scalar power level.
fn default_for_level(name: &str) -> i64 {
    match name {
        "users_default" => DEFAULT_USERS,
        "events_default" => DEFAULT_EVENTS,
        "state_default" => DEFAULT_STATE,
        "invite" => DEFAULT_INVITE,
        "kick" => DEFAULT_KICK,
        "ban" => DEFAULT_BAN,
        "redact" => DEFAULT_REDACT,
        _ => DEFAULT_USERS,
    }
}

/// The server part of a Matrix ID (`!room:server` → `server`).
fn server_of(id: &str) -> Option<&str> {
    id.rsplit_once(':').map(|(_, server)| server)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pdu(
        event_type: &str,
        state_key: Option<&str>,
        sender: &str,
        content: Value,
    ) -> Pdu {
        Pdu {
            event_id: "$event".to_string(),
            room_id: "!room:server".to_string(),
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

    fn member(user: &str, membership: &str) -> Pdu {
        pdu(
            et::MEMBER,
            Some(user),
            user,
            json!({ "membership": membership }),
        )
    }

    fn power_levels(users: Value, users_default: i64) -> Pdu {
        pdu(
            et::POWER_LEVELS,
            Some(""),
            "@alice:server",
            json!({ "users": users, "users_default": users_default }),
        )
    }

    fn room<'a>(events: &'a [Pdu], version: &str) -> RoomState<'a> {
        RoomState::new("!room:server", version, Some("@alice:server"), events)
    }

    #[test]
    fn creator_has_power_level_without_power_levels_event() {
        let events = vec![member("@alice:server", "join")];
        let state = room(&events, "10");
        assert_eq!(state.user_power_level("@alice:server"), 100);
        assert_eq!(state.user_power_level("@bob:server"), 0);
    }

    #[test]
    fn member_cannot_kick_without_kick_level() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 0 }), 0),
        ];
        let state = room(&events, "10");
        let kick = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@bob:server",
            json!({ "membership": "leave" }),
        );
        assert!(check(&state, &kick).is_err());
    }

    #[test]
    fn moderator_can_kick_lower_powered_user() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 0 }), 0),
        ];
        let state = room(&events, "10");
        let kick = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@alice:server",
            json!({ "membership": "leave" }),
        );
        assert!(check(&state, &kick).is_ok());
    }

    #[test]
    fn cannot_kick_user_with_equal_or_higher_power() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 50 }), 0),
        ];
        let state = room(&events, "10");
        let kick = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@alice:server",
            json!({ "membership": "leave" }),
        );
        assert!(check(&state, &kick).is_err());
    }

    #[test]
    fn cannot_ban_without_ban_level() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 10, "@bob:server": 0 }), 0),
        ];
        let state = room(&events, "10");
        let ban = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@alice:server",
            json!({ "membership": "ban" }),
        );
        assert!(check(&state, &ban).is_err());
    }

    #[test]
    fn unban_requires_ban_level() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "ban"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 0 }), 0),
        ];
        let state = room(&events, "10");
        let unban = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@alice:server",
            json!({ "membership": "leave" }),
        );
        assert!(check(&state, &unban).is_ok());
    }

    #[test]
    fn self_join_into_private_room_is_rejected() {
        let events = vec![
            member("@alice:server", "join"),
            pdu(
                et::JOIN_RULES,
                Some(""),
                "@alice:server",
                json!({ "join_rule": "invite" }),
            ),
        ];
        let state = room(&events, "10");
        let join = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@bob:server",
            json!({ "membership": "join" }),
        );
        assert!(check(&state, &join).is_err());
    }

    #[test]
    fn self_join_into_public_room_is_allowed() {
        let events = vec![
            member("@alice:server", "join"),
            pdu(
                et::JOIN_RULES,
                Some(""),
                "@alice:server",
                json!({ "join_rule": "public" }),
            ),
        ];
        let state = room(&events, "10");
        let join = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@bob:server",
            json!({ "membership": "join" }),
        );
        assert!(check(&state, &join).is_ok());
    }

    #[test]
    fn cannot_rewrite_power_levels_above_own_level() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 50 }), 0),
        ];
        let state = room(&events, "10");
        let pl = pdu(
            et::POWER_LEVELS,
            Some(""),
            "@bob:server",
            json!({ "users": { "@bob:server": 100 }, "users_default": 0 }),
        );
        assert!(check(&state, &pl).is_err());
    }

    #[test]
    fn cannot_change_another_users_power_level_above_own_level() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 50 }), 0),
        ];
        let state = room(&events, "10");
        let pl = pdu(
            et::POWER_LEVELS,
            Some(""),
            "@bob:server",
            json!({ "users": { "@alice:server": 0, "@bob:server": 50 }, "users_default": 0 }),
        );
        assert!(check(&state, &pl).is_err());
    }

    #[test]
    fn message_event_respects_events_default() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 0 }), 50),
        ];
        let state = room(&events, "10");
        let message = pdu(
            et::MESSAGE,
            None,
            "@bob:server",
            json!({ "body": "hi", "msgtype": "m.text" }),
        );
        assert!(check(&state, &message).is_err());
    }

    #[test]
    fn state_event_requires_state_default() {
        let events = vec![
            member("@alice:server", "join"),
            member("@bob:server", "join"),
            power_levels(json!({ "@alice:server": 50, "@bob:server": 0 }), 0),
        ];
        let state = room(&events, "10");
        let topic = pdu(
            et::TOPIC,
            Some(""),
            "@bob:server",
            json!({ "topic": "hello" }),
        );
        assert!(check(&state, &topic).is_err());
    }

    #[test]
    fn non_member_cannot_send_events() {
        let events = vec![member("@alice:server", "join")];
        let state = room(&events, "10");
        let message = pdu(
            et::MESSAGE,
            None,
            "@bob:server",
            json!({ "body": "hi", "msgtype": "m.text" }),
        );
        assert!(matches!(
            check(&state, &message),
            Err(AuthError::NotJoined(_))
        ));
    }

    #[test]
    fn invite_requires_sender_to_be_joined() {
        let events = vec![member("@alice:server", "join")];
        let state = room(&events, "10");
        let invite = pdu(
            et::MEMBER,
            Some("@bob:server"),
            "@carol:server",
            json!({ "membership": "invite" }),
        );
        assert!(check(&state, &invite).is_err());
    }
}
