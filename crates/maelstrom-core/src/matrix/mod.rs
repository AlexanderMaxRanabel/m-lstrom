//! Matrix protocol types — the complete domain model for a Matrix homeserver.
//!
//! This module is the root namespace for everything Matrix-related in
//! Maelstrom. If you're looking for a Matrix concept, it lives here.
//!
//! # Layered architecture
//!
//! The sub-modules form a dependency stack. Lower layers know nothing about
//! higher ones:
//!
//! ```text
//!   ┌─────────────────────────┐
//!   │  state   (resolution)   │  ← decides which events "win"
//!   ├─────────────────────────┤
//!   │  auth_rules (permissions)│ ← decides which events are allowed
//!   ├─────────────────────────┤
//!   │  signing (ed25519)      │  ← signs & verifies events / federation requests
//!   ├─────────────────────────┤
//!   │  event   (Pdu)          │  ← the core persistent event type
//!   │  content (typed bodies) │  ← what goes inside an event's `content` field
//!   │  edu     (ephemeral)    │  ← non-persisted data units (typing, receipts)
//!   ├─────────────────────────┤
//!   │  room    (enums)        │  ← Membership, JoinRule, HistoryVisibility, etc.
//!   │  keys    (device keys)  │  ← one-time keys, cross-signing, key queries
//!   │  error   (MatrixError)  │  ← spec-compliant JSON error responses
//!   ├─────────────────────────┤
//!   │  id      (identifiers)  │  ← UserId, RoomId, EventId, DeviceId, …
//!   │  json    (canonical)    │  ← canonical JSON for hashing and signing
//!   └─────────────────────────┘
//! ```
//!
//! # Where to start
//!
//! If you're new to the codebase, read the modules in this order:
//!
//! 1. **[`id`]** — Identifier types. Every Matrix entity is addressed by a
//!    sigil-prefixed string (`@user:server`, `!room:server`, `$event_id`).
//!    Start here to understand how they're parsed and validated.
//!
//! 2. **[`event`]** — The `Pdu` struct. This is the single most important
//!    type: every message, state change, or room action is a PDU.
//!
//! 3. **[`content`]** — Typed event content. A PDU's `content` field is
//!    untyped JSON; the content module gives you `RoomCreate`,
//!    `RoomMessage`, `RoomMember`, etc.
//!
//! 4. **[`room`]** — Enums that describe room configuration: who can join,
//!    who can read history, what the room version is.
//!
//! 5. **[`error`]** — The standard error type. Every API handler returns
//!    `Result<T, MatrixError>`, which serializes to the JSON shape the
//!    spec requires.
//!
//! 6. **[`signing`]** and **[`state`]** — Advanced topics. Signing handles
//!    Ed25519 for event hashes and federation; state resolution is the
//!    algorithm that merges conflicting room state.
//!
//! # Why not ruma?
//!
//! [Ruma](https://github.com/ruma/ruma) is excellent for clients and
//! general-purpose Matrix code, but it optimizes for completeness and
//! type safety via heavy proc-macro usage. For a homeserver, we need:
//!
//! - **Speed over ceremony** — we parse millions of events; avoiding macro
//!   layers and deeply nested generics keeps compile times fast and code
//!   greppable.
//! - **Homeserver-only types** — we don't need client request/response
//!   wrappers, appservice types, or identity server types.
//! - **Direct control** — state resolution, canonical JSON, and signing
//!   are correctness-critical. Owning the code means we can audit and
//!   optimize it without fighting an upstream API.

pub mod auth_rules;
pub mod content;
pub mod edu;
pub mod ephemeral;
pub mod error;
pub mod event;
pub mod id;
pub mod json;
pub mod keys;
pub mod room;
pub mod signing;
pub mod state;
