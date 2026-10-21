# Maelstrom (m-lstrom) — behavioural audit vs. Matrix v1.19 (Client-Server + Server-Server)

Repo audited: `/root/m-lstrom` @ working tree
Reference material: `/tmp/opencode/spec/matrix-spec-1.19/` (spec source + `data/api/**` OpenAPI) and
`/tmp/opencode/complement-main/` (official Complement suite, `tests/csapi/*` = stable, `tests/*` = federation).

Method: for each area I read (1) the spec section / OpenAPI schema, (2) the Complement test that encodes the
behaviour, and (3) the Maelstrom implementation, then traced actual code paths. No claim below is made without a
code citation (`file:line`) or a spec/complement citation.

Legend: **Blocker** = breaks a hard spec MUST or breaks a stable Complement test outright ·
**Major** = spec deviation that breaks real clients / several tests · **Minor** = cosmetic, partial, or MSC-only.

---

## 1. SYNC — `GET /_matrix/client/v3/sync`

Code: `crates/maelstrom-api/src/handlers/sync.rs` (2163 lines), `crates/maelstrom-api/src/handlers/capabilities.rs`

### (a) What the spec / Complement require

* Spec `data/api/client-server/sync.yaml`: response object with **required** `next_batch`; `rooms.join/leave/invite`;
  `rooms.join[].state`, `.timeline` (`definitions/timeline_batch.yaml` — required `events`, optional `limited`,
  `prev_batch`), `.ephemeral`, `.account_data`, `.unread_notifications` (`highlight_count`, `notification_count`),
  optional `summary` (`m.joined_member_count`, `m.invited_member_count`, `m.heroes`);
  top-level `presence`, `account_data`, `to_device`, `device_lists`, `device_one_time_keys_count`.
* `full_state=true` → "all state events will be returned, even if `since` is non-empty"
  (`sync.yaml`, `full_state` param) — anchor `spec.matrix.org/v1.19/client-server-api/#get_matrixclientv3sync`.
* Filters (`definitions/room_event_filter.yaml` + `definitions/event_filter.yaml`): `limit`, `types`, `not_types`
  (glob `*`), `senders`, `not_senders`, `rooms`, `not_rooms`, `contains_url`, `lazy_load_members`,
  `unread_thread_notifications`.
* `limited` semantics (`definitions/timeline_batch.yaml`): "True if the number of events returned was limited by
  the `limit` on the filter."
* E2E sync extensions (`content/client-server-api/modules/end_to_end_encryption.md` §"Extensions to /sync"):
  `device_one_time_keys_count` — *Required if any unclaimed one-time keys exist*; `device_unused_fallback_key_types`
  — ***Required***; `device_lists.left` = "List of users with whom we do not share any encrypted rooms anymore".
* Complement `tests/csapi/sync_test.go`:
  * `TestSync/Can sync a joined room` — unchanged rooms must not reappear.
  * `TestSync/Full state sync includes joined rooms` — `full_state=true` must return the room + state.
  * `TestSync/Newly joined room has correct timeline in incremental sync` — timeline must contain only
    `m.room.message` (filter `types`), and `limited` must be **false** when all events fit, **true** only when the
    filter limit truncated the timeline.
  * `TestSync/Device list tracking` — `device_lists.left` must contain a user who left, even with lazy-loading.
  * `TestSyncTimelineGap` — a gappy timeline must be flagged `limited: true`.
  * `TestCumulativeJoinLeaveJoinSync`, `TestTentativeEventualJoiningAfterRejecting` — `rooms.leave` correctness.
  * `TestRoomSummary` — `m.joined_member_count` / `m.invited_member_count` must be present and correct.
  * `tests/csapi/sync_filter_test.go` — filter create/get round-trip.

### (b) What Maelstrom does

| Behaviour | Code |
|---|---|
| `since` parsed as `i64` stream position; unparsable token silently becomes `0` | `sync.rs:260-264` |
| `full_state` parsed then **discarded** | `sync.rs:266` (`let _full_state = query.full_state.unwrap_or(false);`) |
| Filters parsed: `room.include_leave`, `room.timeline.limit`, `room.timeline.types`, `room.state.types`, `room.state.lazy_load_members` | `sync.rs:288-333` |
| `timeline_types` used **only** in the `rooms.leave` branch | `sync.rs:774`, `sync.rs:795` (grep: no other use) |
| `not_types`, `contains_url`, `senders`, `not_senders`, `rooms`, `not_rooms` never read | grep over `sync.rs` → absent |
| Initial sync timeline = hard-coded 10 events, no type filter, no `state.types` filter | `sync.rs:930-936` |
| Initial sync `state` = full current state, filtered only by `lazy_load_members` (sender heuristic) | `sync.rs:948-967` |
| Incremental sync: state events of *other* rooms also in timeline; timeline limit `unwrap_or(20)` | `sync.rs:1124-1130` |
| `limited = is_newly_joined \|\| len > limit` | `sync.rs:1125` |
| `unread_notifications` hard-coded `{highlight_count: 0, notification_count: 0}` | `sync.rs:444`, `522`, `1012`, `1196`, `1454` |
| `summary` = joined/invited counts, no `m.heroes` | `sync.rs:202-208`, `986-995`, `1161-1178` |
| `device_lists` only on incremental sync; `changed`/`left` computed from member events + `_maelstrom.device_change_pos`; **no `fallback_keys`** | `sync.rs:1241-1372` |
| **No** `device_one_time_keys_count`, **no** `device_unused_fallback_key_types` in the response struct | `sync.rs:128-140` (`SyncResponse`) |
| `account_data` always re-sends *all* global account data (not a delta) | `sync.rs:1763-1791` |
| `m.push_rules` account-data content uses a **different, much smaller** default rule set than `GET /pushrules/` | `sync.rs:1683-1725` vs `capabilities.rs:643-761` |
| Long-poll only for incremental syncs (`timeout>0 && !is_initial`) | `sync.rs:386-395` |
| `to_device` section present, events deleted after ack | `sync.rs:370-381`, `534-537` |
| `presence` section built from all shared-room members | `sync.rs:1581-1654` |

### (c) Concrete gaps

1. **`full_state` is ignored** — an incremental sync with `full_state=true` does not return full state, and may
   not return the room at all (no changed events ⇒ room omitted). Violates `sync.yaml` `full_state` and breaks
   Complement `TestSync/Full state sync includes joined rooms` semantically (it only passes vacuously today because
   `checkJoinFieldsExist` tolerates a missing room). **[Major]**
2. **Timeline `types`/`not_types` filter is not applied in the incremental sync** (only in `rooms.leave`).
   A filter of `types:["m.room.message"]` will still return state events and other types.
   Complement `TestSync/Newly joined room has correct timeline in incremental sync` asserts every timeline event is
   `m.room.message`. **[Major]**
3. **`limited` is wrong for newly-joined rooms**: it is set to `true` unconditionally (`sync.rs:1125`), so a
   newly-joined room whose timeline is complete is reported as truncated. The Complement test requires
   `limited == false` when all events fit. **[Major]**
4. **Initial sync ignores the filter's `timeline.limit`** (hard-coded 10) and applies no `types`/`state.types`
   filter. **[Major]**
5. **`not_types`, `contains_url`, `senders`, `not_senders`, `rooms`, `not_rooms` are not implemented at all** —
   6 of the 9 spec filter fields. **[Major]**
6. **`unread_notifications` is always `{0,0}`** — no notification/highlight counting at all, even though read
   receipts are persisted (`handlers/receipts.rs`). Clients will show 0 unread forever. Spec:
   `sync.yaml` `unread_notifications`. **[Major]**
7. **`device_one_time_keys_count` and `device_unused_fallback_key_types` are absent from the sync response** though
   the spec marks the latter *Required*. **[Major]**
8. **`rooms.join[].summary` has no `m.heroes`** (optional in spec, but required by the "Required if the room's
   name/canonical_alias are unset" wording) and `summary` is omitted entirely when no membership change happened
   in the window — the spec allows omission only "if this field has not changed since the last sync". **[Minor]**
9. **Initial sync `prev_batch` = the oldest returned event's own position**, not the token *before* it, so
   `GET /messages?from=prev_batch` re-delivers that event. **[Minor]**
10. **Invalid `since` token is silently treated as `0`** while `is_initial` stays `false` → the client receives a
    full "incremental" sync from the beginning of time instead of an error. **[Minor]**
11. **Long-polling is disabled for initial syncs** (`sync.rs:386`); the spec's `timeout` applies to any `/sync`. **[Minor]**
12. **`account_data` is not delta-based** — the whole global + per-room account data is re-sent on every sync
    (acceptable but wasteful, and it means deletions rely on the MSC3391 sentinel hack). **[Minor]**
13. **`m.push_rules` content in `/sync` disagrees with `GET /pushrules/`** — the sync copy has 3 override rules and
    1 underride rule (`sync.rs:1683-1725`) while `GET /pushrules/` returns ~30 rules (`capabilities.rs:643-761`).
    Two different views of the same state ⇒ clients behave differently depending on which they read.
    **[Major]**

### (d) Suggested fix & risk

* Honour `full_state`: when `full_state==true`, build the join map like `build_initial_sync` (full `state` section)
  but keep the timeline windowed by `since`. **Effort S, risk low** (pure read-path change).
* Thread the parsed filter (`types`, `not_types`, `senders`, `not_senders`, `contains_url`, `rooms`, `not_rooms`)
  into `build_incremental_sync`/`build_initial_sync` as a single `EventFilter` struct and apply it to both the
  timeline and the state section. **Effort M, risk medium** (can change which events clients see; add filter unit
  tests).
* Fix `limited` to mean "the filter limit truncated the timeline" only; keep `prev_batch` = first event position − 1.
  **Effort S, risk low.**
* Use `timeline.limit` (default 10) in `build_initial_sync` instead of the hard-coded 10. **Effort S, risk low.**
* Compute `unread_notifications` from the stored `m.read` receipt per room + a simple notification predicate
  (non-`m.notice` message events). Highlight counts need push-rule evaluation — start with `m.mentions`.
  **Effort M, risk medium** (must not break `TestRoomSummary`).
* Add `device_one_time_keys_count` (from `count_one_time_keys`) and `device_unused_fallback_key_types` (empty list
  until fallback keys are implemented) to `SyncResponse`. **Effort S, risk low.**
* Extract the canonical default push-rule set into one function used by both `GET /pushrules/` and the
  `m.push_rules` account-data blob. **Effort S, risk low.**

---

## 2. E2EE / KEYS

Code: `crates/maelstrom-api/src/handlers/keys.rs`, `crates/maelstrom-api/src/handlers/capabilities.rs`
(plus `crates/maelstrom-storage/src/surreal/keys.rs`)

### (a) Spec / Complement requirements

* `data/api/client-server/keys.yaml`:
  * `POST /keys/upload` — `device_keys` (per `definitions/device_keys.yaml`: `user_id`, `device_id`, `algorithms`,
    `keys`, `signatures`), `one_time_keys`, `fallback_keys` (v1.2); response **required** `one_time_key_counts`.
  * `POST /keys/query` — `device_keys` map required; response `device_keys`, `master_keys`, `self_signing_keys`,
    `user_signing_keys`, `failures` (per-server failure objects).
  * `POST /keys/claim` — `one_time_keys` required; response `one_time_keys`, `failures`.
    "One-time keys are given out **in the order that they were uploaded**".
  * `GET /keys/changes?from=&to=` — response `changed` and `left` ("users who may have left all the end-to-end
    encrypted rooms they previously shared").
* `data/api/client-server/key_backup.yaml`: `POST/GET /room_keys/version`, `GET/PUT /room_keys/version/{v}`,
  `DELETE /room_keys/version/{v}`, `PUT/GET/DELETE /room_keys/keys/{roomId}/{sessionId}`,
  `PUT/GET/DELETE /room_keys/keys/{roomId}`, `PUT/GET/DELETE /room_keys/keys`.
* `content/client-server-api/modules/end_to_end_encryption.md` — `/keys/claim` and `/keys/query` must be
  **federated** to the remote user's homeserver (the ASCII diagrams at lines 120-140 show `<federation>` on both).
* Complement:
  * `tests/csapi/upload_keys_test.go` — upload device keys + OTKs, `one_time_key_counts` must match, invalid
    device keys ⇒ `400 M_BAD_JSON`, keys for another user ⇒ 400, query returns `device_keys.<user>.<device>` with
    `algorithms`/`keys`/`signatures`, query for a user with no keys ⇒ empty object, claim returns the key and
    removes it.
  * `tests/csapi/upload_keys_test.go:TestKeyClaimOrdering` — MSC4225: keys must be issued in **upload order**.
  * `tests/csapi/upload_keys_test.go:TestUploadKeyIdempotency{,Overlap}` — re-uploading the same OTK must not
    error and the count must stay correct (A,B,C + B,C,D ⇒ 4).
  * `tests/csapi/keychanges_test.go` — after a new login+key upload, `device_lists.changed` must contain the user
    and `GET /keys/changes?from&to` must return it in `changed.0`.
  * `tests/csapi/device_lists_test.go` — `device_lists.changed` / `.left` semantics for join/leave/rejoin, local
    and remote.
  * `tests/csapi/e2e_key_backup_test.go` — per-session key replacement rules (verified > first_message_index >
    forwarded_count) and `?version=` handling.
  * `tests/csapi/user_query_keys_test.go` — `/keys/query` for a user with no keys.

### (b) What Maelstrom does

* Routes: `keys.rs:57-89` — implements upload/query/claim/changes/device_signing/signatures plus
  `room_keys/version`, `room_keys/version/{v}`, `room_keys/keys/{roomId}/{sessionId}`,
  `room_keys/keys/{roomId}`, `room_keys/keys`. **Missing**: `PUT/DELETE /room_keys/version/{v}`,
  `DELETE` variants of all three key endpoints.
* `keys_upload` (`keys.rs:94-209`): validates `user_id`/`device_id` match, requires `algorithms`+`keys`; stores
  device keys; stores `one_time_keys`; **ignores `fallback_keys`** (grep: no `fallback` handling anywhere in
  `crates/`); returns `one_time_key_counts`.
* `keys_query` (`keys.rs:214-386`): splits local/remote; local keys from storage; remote users via
  `/_matrix/federation/v1/user/keys/query`; returns `master_keys`/`self_signing_keys`/`user_signing_keys` only when
  non-empty; **ignores the `timeout` request field**; does not add `unsigned.device_display_name`.
* `keys_claim` (`keys.rs:391-412`): passes the request straight to storage — **no federation** for remote users,
  no `timeout` handling; `failures` always `{}`.
* Storage `claim_one_time_keys` (`crates/maelstrom-storage/src/surreal/keys.rs:206-263`): selects
  `ORDER BY key_id DESC LIMIT 1` → **not upload order** (no upload sequence/timestamp is stored).
* `keys_changes` (`keys.rs:424-462`): `from`/`to` parsed as `i64`; `changed` derived from
  `_maelstrom.device_change_pos` of users in shared rooms; **`left` is hard-coded `[]`** (`keys.rs:460`).
* `keys_device_signing_upload` (`keys.rs:467-534`): requires UIA (correct) and stores master/self/user-signing keys.
* `keys_signatures_upload` (`keys.rs:539-541`): **stub** — accepts the body and returns `{"failures":{}}`,
  **discarding every signature**. Cross-signing device signatures are therefore never stored or returned.
* Key backup: per-session `PUT/GET` works and implements the replacement rules (`keys.rs:667-716`);
  `GET /room_keys/version` returns `count: 0`, `etag: "0"` always (`keys.rs:617-618`);
  `PUT /room_keys/keys/{roomId}` (`keys.rs:751-758`), `GET /room_keys/keys/{roomId}` (`761-766`),
  `PUT /room_keys/keys` (`769-774`), `GET /room_keys/keys` (`776-779`) are **stubs returning empty results**.
* `fallback_keys` are not stored at all ⇒ `device_unused_fallback_key_types` can never be computed.

### (c) Concrete gaps

1. **`/keys/changes` never returns `left`** — hard-coded `[]` (`keys.rs:460`). The spec requires `left` to list
   users who no longer share an encrypted room; Complement's device-list flows depend on it (the `/sync` side of
   `left` is implemented, the `/keys/changes` side is not). **[Major]**
2. **`/keys/signatures/upload` discards all signatures** — cross-signing is therefore non-functional: device
   signatures, self-signing signatures and user-signing signatures are never persisted, so `/keys/query` can never
   return a verified cross-signing identity. **[Blocker for E2EE]**
3. **`fallback_keys` are not supported** (upload ignores them, no storage, no
   `device_unused_fallback_key_types`). Spec `keys.yaml` `fallback_keys` (added v1.2) + E2E module. **[Major]**
4. **One-time keys are not issued in upload order** (`ORDER BY key_id DESC`, `surreal/keys.rs:226`) — violates
   `keys.yaml` ("in the order that they were uploaded") and makes `TestKeyClaimOrdering` pass only by accident
   (`:1` sorts above `:0`). **[Major]**
5. **`/keys/claim` does not federate** to remote users' homeservers (`keys.rs:391-412`) — E2EE key exchange with
   remote users is impossible; the spec's E2E module explicitly shows `/keys/claim <federation>`. **[Major]**
6. **Bulk key-backup endpoints are stubs** (`keys.rs:751-779`): a client that PUTs its whole backup and then GETs it
   gets nothing back — silent data loss for the most common client flow. Also `count`/`etag` are always `0`
   (`keys.rs:617-618`, `720`), so clients cannot detect changes. **[Major]**
7. **Missing key-backup endpoints**: `PUT /room_keys/version/{version}` (update `auth_data`), `DELETE
   /room_keys/version/{version}`, `DELETE /room_keys/keys/...` (3 variants) — all required by
   `key_backup.yaml`. **[Major]**
8. **`/keys/query` ignores the `timeout` field** and never returns a `failures` entry for a *reachable* server
   with an unknown user (correct) but also never federates `timeout`; remote failures are reported as
   `{"errcode":"M_UNKNOWN"}` objects (`keys.rs:350-356`) — acceptable shape. **[Minor]**
9. **`/keys/query` does not attach `unsigned.device_display_name`** (`keys.yaml` `DeviceInformation.unsigned`),
   so clients cannot show device names. **[Minor]**
10. **OTK re-upload idempotency** relies on `DELETE`+`CREATE` per key id (`surreal/keys.rs:160-165`); the
    "upload A,B,C then B,C,D ⇒ count 4" case works only because the delete is by key id, but a *changed* key body
    under the same id silently replaces the old one (no `first_seen`/`device_id` conflict detection). **[Minor]**

### (d) Suggested fix & risk

* Persist signatures: add a `signature` table (or a JSON blob per device key) and merge incoming
  `signatures` from `/keys/signatures/upload` into the stored device key / cross-signing key, then return them from
  `/keys/query`. **Effort M, risk medium** (needs signature validation before storing).
* Implement `left` in `/keys/changes` by reusing the same "user no longer shares a room" logic as
  `sync.rs:compute_device_lists` (factor it into a shared helper). **Effort S, risk low.**
* Add `fallback_keys` storage + `device_unused_fallback_key_types` in `/sync`; mark a fallback key "used" when it
  is returned by `/keys/claim`. **Effort M, risk medium.**
* Add a monotonically increasing `upload_seq` to stored OTKs and claim with `ORDER BY upload_seq ASC`.
  **Effort M, risk medium** (schema change + backfill).
* Implement the bulk backup endpoints by storing the request body under the current version and returning it
  verbatim; track a real `count` and `etag`. **Effort M, risk low-medium.**
* Federate `/keys/claim` (and honour `timeout`) via the federation client to the remote user's server.
  **Effort M, risk medium** (no standard S2S claim endpoint; must use the remote CS API over an authenticated
  request — verify the S2S spec supports this before implementing).

---

## 3. STATE / AUTH RULES

Code: `crates/maelstrom-api/src/handlers/events.rs`, `crates/maelstrom-api/src/handlers/rooms.rs`,
`crates/maelstrom-core/src/matrix/state.rs`, `crates/maelstrom-federation/src/receiver.rs`

### (a) Spec / Complement requirements

* Room-version authorisation rules, e.g. `content/rooms/v11.md` §"Authorisation rules" (v1-v11 share the same
  rule shape, v6+ differ in the membership rules). Anchor: `spec.matrix.org/v1.19/rooms/v11/#authorisation-rules`:
  * rule 2 — `auth_events` must be present, must contain `m.room.create`, must match the auth-events-selection
    algorithm, no duplicate `(type, state_key)` pairs.
  * rule 4.3 `membership: join` — sender must equal `state_key`; ban check; `join_rule` handling
    (`invite`/`knock`/`restricted`/`knock_restricted`/`public`).
  * rule 4.4 `membership: invite` — sender must be joined; target must not be joined/banned;
    **"If the `sender`'s power level is greater than or equal to the *invite level*, allow"**.
  * rule 4.5 `membership: leave` — self-leave allowed from invite/join/knock; **kick level** check and
    "target's power level < sender's power level".
  * rule 4.6 `membership: ban` — sender joined; **ban level** check; target PL < sender PL.
  * rule 5 — "If the `sender`'s current membership state is not `join`, reject" (for all other events).
  * rule 7 — required power level per event type (`events` / `state_default` / `events_default`).
  * rule 8 — `state_key` starting with `@` must equal the sender.
  * rule 9 — `m.room.power_levels` change rules (cannot raise a level above your own).
* Server-Server: `content/server-server-api.md` §"Checks performed on receipt of a PDU" (lines 444-470):
  a valid event, **signature checks (drop)**, **hash checks (redact)**, **authorisation rules based on auth events
  (reject)**, **authorisation rules based on state before the event (reject)**, **based on current state (soft
  fail)**.
* Event format for federation: `origin`, `prev_events`, `depth`, `auth_events`, `signatures`, `hashes` (v3+) —
  `content/server-server-api.md` §"PDUs" (line 427+), `content/rooms/v11.md` §"Event format".
* State resolution v2: `content/rooms/v2.md` (State Resolution Version 2) — unconflicted state, conflicted
  events split into auth-related/non-auth-related, **reverse topological power ordering**, iterative auth checks.
* Complement: `tests/federation_event_auth_test.go` (`/event_auth` chain), `tests/federation_room_event_auth_test.go`,
  `tests/csapi/power_levels_test.go` (default PL fields, PL content round-trip, "PUT power_levels should not
  explode"), `tests/csapi/room_kick_test.go`, `tests/csapi/room_ban_test.go` (403 for non-member / left user),
  `tests/csapi/room_members_test.go`.

### (b) What Maelstrom does

* **Locally created events carry no federation fields**: `events.rs:187-207` builds
  `Pdu { …, origin: None, auth_events: Some(...), prev_events: None, depth: None, hashes: None, signatures: None }`.
  `Pdu::sign()` exists (`crates/maelstrom-core/src/matrix/event.rs:236`) but is **never called from production
  code** (grep: only used in `event.rs` unit tests). `to_federation_json()` therefore omits
  `origin`/`prev_events`/`depth`/`hashes`/`signatures` (`event.rs:175-206`).
* Events are pushed to remote servers verbatim by `federate_event` → `TransactionSender::queue_pdu`
  (`handlers/util.rs:217-229`, `maelstrom-federation/src/sender.rs:83-95`) inside a transaction whose
  `origin` is the local server (`sender.rs:175-180`).
* **Local auth rules are only partially implemented**:
  * `send_event` (`events.rs:148-233`): membership must be `join`; **no power-level check for message events**
    (`events_default` is never consulted).
  * `do_set_state` (`events.rs:685-753`): PL check for state events (uses `state_default`/`events[event_type]`,
    or `users_default` when the state_key is the sender) — but the check is **skipped entirely for the sender's own
    `m.room.member` event** (`events.rs:705`, `710`) with no membership-transition validation, so a user can write
    `m.room.member` with `membership: "ban"` or `"join"` for themselves.
  * `invite_to_room` (`rooms.rs:1093-1204`): checks the sender is joined, but **never checks the invite level** or
    the target's current membership ban state.
  * `kick_user` (`rooms.rs:1256-1314`), `ban_user` (`1318-1364`), `unban_user` (`1368-1427`): check only that the
    sender is joined — **no kick-level, no ban-level, no target-PL comparison**. Any joined member can ban anyone.
  * `create_room` (`rooms.rs:227-431`): no `m.federate` handling, no `room_version` 12 support.
  * `room_version` accepted values are `"1".."11"` (`rooms.rs:238`) and the default is `"10"` (`rooms.rs:235`),
    while `GET /capabilities` advertises `"12": "stable"` and `"13": "stable"` with default `"11"`
    (`capabilities.rs:151-175`).
* **Federation receiver auth is soft-fail** (`maelstrom-federation/src/receiver.rs`):
  * signature verification failure → `warn!` and accept (`:472-514`);
  * "sender's server != origin" → warn (`:516-528`);
  * "sender not in room" → warn (`:540-564`);
  * missing `auth_events` → warn and accept (`:566-584`);
  * power-level failure → warn only (`:969-1029`).
* **State resolution** (`crates/maelstrom-core/src/matrix/state.rs:90-226`) is a simplification: it compares only
  the *one* conflicting `(type, state_key)` pair, has no mainline ordering, no reverse-topological power ordering,
  and its "auth check" is only a power-level comparison. The receiver calls it with two single-key state sets
  (`receiver.rs:606-670`, `resolve_state(&[set1, set2], &HashMap::new())`) — it never builds the full state sets at
  the fork tips, so it is not state resolution v2.

### (c) Concrete gaps

1. **Locally created events are not valid federation PDUs** — no `origin`, `prev_events`, `depth`, `hashes`,
   `signatures`. Every event Maelstrom sends to another homeserver is malformed per
   `content/server-server-api.md` §PDUs and `content/rooms/v11.md` §Event format; remote servers must reject them.
   `Pdu::sign()` is already implemented but never wired up. **[Blocker]**
2. **Kick/ban/unban have no power-level checks** — violates v11 auth rules 4.5.4 / 4.6.2 (kick level, ban level,
   target-PL comparison). Any joined user can ban/kick anyone in the room. **[Blocker]**
3. **Invite has no invite-level check** and no ban check on the target — v11 rule 4.4. **[Blocker]**
4. **`m.room.member` state events written by a user for themselves bypass all checks** (`events.rs:705-710`):
   a user can set `membership: "ban"` (or `join` in an invite-only room) for their own `state_key`, and the server
   accepts it. v11 rules 4.1/4.3/4.4. **[Blocker]**
5. **No power-level check for non-state events** — `events_default` is never applied to `PUT /send`
   (`events.rs:148-233`), so a room with `events_default: 50` still accepts messages from PL 0 users. v11 rule 7.
   **[Major]**
6. **No `m.room.power_levels` change rules** (v11 rule 9) — a PL-50 user can rewrite the PL event to grant
   themselves PL 100, because `do_set_state` only compares the *sender's* PL against the required level for the
   event type (`events.rs:710-753`). **[Blocker]**
7. **Federation receiver accepts events that fail signature / auth checks** (`receiver.rs:489-514`, `557-563`,
   `576-584`, `1018-1028`) — the S2S spec says these MUST be dropped/rejected. **[Blocker]**
8. **State resolution is not state resolution v2** (`state.rs:90-226`, `receiver.rs:606-670`) — only a single
   conflicting key is considered, no auth chain, no power ordering over the DAG, no mainline ordering. Divergent
   state after a split-brain. **[Blocker]**
9. **Room version support contradicts the capabilities endpoint**: `capabilities` advertises 12 and 13 as stable
   (`capabilities.rs:169-170`) but `createRoom` rejects anything above 11 (`rooms.rs:238-245`); the spec's stable
   list stops at 12 and says servers **SHOULD** default to v12 (`content/rooms/_index.md:56`), Maelstrom defaults
   to v10 (`rooms.rs:235`) and advertises v11 as default (`capabilities.rs:156`). Also v12 changes the room ID to a
   hash of the create event and gives creators infinite power — neither is implemented. **[Major]**
10. **`m.room.member` `state_key` ownership rule (v11 rule 8) is only enforced for non-member events**
    (`events.rs:757-761`) — correct per spec, but the *content* of the member event is not validated at all.
    **[Minor]**
11. **`m.room.create` auth rule 1 ("if it has any `prev_events`, reject") and the `m.federate` rule 3 are not
    checked** on the federation path. **[Minor]**

### (d) Suggested fix & risk

* **Wire up event signing** (`Pdu::sign()` + `content_hash`/`reference_hash`), set `origin`, `depth`,
  `prev_events` (the current forward tips) and `auth_events` on every locally created event, and store them.
  **Effort M, risk medium** (touches every event-creation path: `events.rs`, `rooms.rs` `store_state_event`).
* Implement a single `auth_rules::check(room_version, state, event)` module implementing the v11 rules
  (member transitions, invite/kick/ban levels, PL change rules, state_key ownership) and call it from
  `send_event`, `do_set_state`, `invite`, `kick`, `ban`, `unban`, and from the federation receiver.
  **Effort L, risk medium-high** (can start rejecting requests that previously succeeded — needs a migration
  window and tests).
* In the receiver, turn the `warn!` soft-fails into `Err(MatrixError::forbidden(...))` / drops, per
  `server-server-api.md` §"Checks performed on receipt of a PDU". **Effort S, risk high** (may break interop with
  the current Complement federation server; must be done together with the signing fix).
* Replace `state.rs::resolve_state` with a real state-res-v2 implementation over full state sets (mainline
  ordering + reverse topological power ordering + iterative auth) and feed it the state at each fork tip.
  **Effort L, risk high.**
* Reconcile room versions: implement v12 (hash room IDs, infinite creator PL) or stop advertising it; make
  `capabilities.rs` the single source of truth used by `create_room`'s validation. **Effort M, risk low** (or L if
  v12 is implemented).

---

## 4. MEDIA / CONTENT REPO

Code: `crates/maelstrom-api/src/handlers/media.rs`, `crates/maelstrom-media/*`,
`crates/maelstrom-federation/src/router.rs`

### (a) Spec / Complement requirements

* `data/api/client-server/content-repo.yaml`:
  * `GET /media/v3/download/{serverName}/{mediaId}` and
    `GET /client/v1/media/download/{serverName}/{mediaId}[/{fileName}]` — response **must** carry
    `Content-Type` and (since v1.12) **required** `Content-Disposition`, "MUST be one of `inline` or `attachment`,
    and SHOULD contain a file name"; "If the upload was made with a `filename`, this header MUST contain the same
    `filename`".
  * `GET /media/v3/thumbnail/...` — response **required** `Content-Disposition: inline` ("SHOULD contain a file
    name (e.g. `thumbnail.png`)") and `Content-Type`; `400 M_UNKNOWN` "if the server cannot thumbnail the content".
  * `GET /media/v3/config` → `m.upload.size`.
  * `GET /media/v3/preview_url?url=` → OpenGraph fields; the spec's preview module expects the server to
    **cache the image and return an `mxc://` URI** plus `og:image:width` / `og:image:height` /
    `matrix:image:size`.
  * Serving inline content: `content/client-server-api/modules/*` — content types such as `text/html` and
    `image/svg+xml` MUST be served as `attachment`.
* Server-Server: `/_matrix/federation/v1/media/download/{mediaId}` and
  `/_matrix/federation/v1/media/thumbnail/{mediaId}` (the thumbnail endpoint returns a **multipart** body).
* Complement: `tests/media_filename_test.go` (Content-Disposition must be parseable and carry the right filename
  for ASCII/unicode/override names, over both the legacy and the `client/v1` endpoints, and over federation),
  `tests/media_nofilename_test.go` (download without a filename must still return the right bytes and mime-type),
  `tests/media_thumbnail_test.go` (local + remote thumbnails, `/_matrix/federation/v1/media/thumbnail/{id}`
  multipart response, unauthenticated `client/v1` thumbnail ⇒ 401),
  `tests/federation_media_content_test.go` (remote media download via `client/v1`),
  `tests/csapi/media_misc_test.go` (`GET /media/v3/config` → `m.upload.size` number),
  `tests/csapi/url_preview_test.go` (`og:title`, `og:type`, `og:url`, `og:image` must be an `mxc://` URI,
  `og:image:width`, `og:image:height`, `matrix:image:size`).

### (b) What Maelstrom does

* Routes (`media.rs:58-92`): `client/v1/media/{upload,download,download/{fileName},thumbnail,config,preview_url}`
  and the legacy `media/v3/*` equivalents. **No `/_matrix/federation/v1/media/*` endpoints at all**
  (`crates/maelstrom-federation/src/router.rs:46-58` merges key_server, receiver, joins, state, backfill,
  user_keys, queries, invite — no media).
* `upload` (`media.rs:106-172`): 413 `M_TOO_LARGE` over the configured limit ✓; stores the file in S3 with the
  client-supplied `Content-Type`; records `content_type`, `content_length`, `filename`; returns `content_uri` ✓.
* `serve_media` (`media.rs:216-261`): sets `Content-Type` and `Content-Length` ✓; sets `Content-Disposition`
  **only if a filename is known** (`media.rs:244-249`) and always uses `inline`; **no `Cache-Control` header**
  (grep: no `CACHE_CONTROL` anywhere in `crates/`); no safe/unsafe content-type classification.
* `proxy_remote_media` (`media.rs:264-313`): for remote MXCs it fetches
  `https://{server}/_matrix/media/v3/download/...` over plain HTTPS with no federation auth and no
  `Authorization`/signature; drops the remote `Content-Disposition` filename (only the URL-supplied override is
  used) and does not forward `Cache-Control`.
* `thumbnail` (`media.rs:324-373`): looks the record up with `get_media(server_name, media_id)` — **remote media
  is not supported** (⇒ 404 for a remote MXC); generates a PNG/JPEG thumbnail (`maelstrom-media/src/thumbnail.rs`);
  on failure it **serves the original bytes instead of a `400 M_UNKNOWN`** (`media.rs:364-371`); sets no
  `Content-Disposition`.
* `preview_url` (`media.rs:401-411` → `maelstrom-media/src/preview.rs`): fetches the URL, parses `og:*` meta tags
  into `OgMetadata` (`preview.rs:31-47`): `og:title`, `og:description`, `og:image` (**the raw remote URL, not an
  `mxc://`**), `og:url`, `og:type`, `og:site_name`, `matrix:image:size` (byte length). **No `og:image:width` /
  `og:image:height`**, and the image is never cached in the media store.

### (c) Concrete gaps

1. **No federation media endpoints** (`/_matrix/federation/v1/media/download/{mediaId}` and
   `.../media/thumbnail/{mediaId}`) — required by the Server-Server API
   (`data/api/server-server/media.yaml`); `tests/media_thumbnail_test.go:TestFederationThumbnail` and
   `tests/federation_media_content_test.go` cannot pass. **[Blocker]**
2. **`Content-Disposition` is omitted whenever no filename is known** — the spec marks it *required* since v1.12
   for both download and thumbnail. `tests/media_filename_test.go` does `mime.ParseMediaType(res.Header.Get(
   "Content-Disposition"))` and `t.Fatalf`s on an empty header. **[Major]**
3. **Thumbnails of remote media 404** (`media.rs:334-338` only looks in the local store) —
   `tests/media_thumbnail_test.go:TestRemotePngThumbnail` expects 200. **[Major]**
4. **Thumbnails never set `Content-Disposition: inline; filename="thumbnail.png"`** (spec: required header). **[Major]**
5. **URL preview does not return an `mxc://` `og:image` and omits `og:image:width`/`og:image:height`** —
   `tests/csapi/url_preview_test.go` asserts all three. **[Major]**
6. **No `Cache-Control` on media responses** — the spec's content repo section recommends
   `Cache-Control: public,max-age=…` / `no-store` semantics for media; also the authenticated-media module
   expects caching behaviour. **[Minor]**
7. **Content types are always served `inline`**, including `text/html` and `image/svg+xml` — the spec's
   "Serving inline content" rules require `attachment` for those (XSS via stored HTML/SVG). **[Major, security]**
8. **Remote media is proxied over unauthenticated plain HTTPS** (`media.rs:271`) and the remote
   `Content-Disposition` filename is dropped ⇒ `tests/media_filename_test.go` "over federation" cases fail, and
   the proxy is spoofable. **[Major]**
9. **No thumbnail caching** — every request re-downloads from S3 and re-encodes (spec recommends caching;
   performance). **[Minor]**
10. **Non-image content is served as the original instead of `400 M_UNKNOWN`** (`media.rs:364-371`) — the spec
    defines a 400 for "the server cannot thumbnail the content". **[Minor]**
11. **No `Cache-Control`/`Content-Disposition` on `preview_url`**, and no caching of the preview result. **[Minor]**

### (d) Suggested fix & risk

* Add `/_matrix/federation/v1/media/download/{mediaId}` and `.../thumbnail/{mediaId}` (multipart response) to the
  federation router, reading from the same media store, and change `proxy_remote_media` to use the *federation*
  endpoint with a signed request instead of plain HTTPS. **Effort M, risk medium.**
* Always emit `Content-Disposition` on download and thumbnail: `inline; filename="…"` for safe types,
  `attachment; filename="…"` for `text/html`, `image/svg+xml`, `application/xhtml+xml`, etc.; derive a fallback
  filename (e.g. `<media_id>` + extension) when the upload had none. **Effort S, risk low.**
* Make `thumbnail` fall back to the remote-proxy path when the media is not local. **Effort S–M, risk low.**
* For URL preview: fetch the `og:image`, upload it into the media store, return `mxc://…`, and decode the image to
  get `og:image:width`/`og:image:height` (the `image` crate is already a dependency). **Effort M, risk low.**
* Add `Cache-Control: public, max-age=…` on immutable media ids and `no-store` on authenticated responses.
  **Effort S, risk low.**

---

## 5. PUSH RULES

Code: `crates/maelstrom-api/src/handlers/capabilities.rs` (lines 72-147 routes, 521-957 handlers)

### (a) Spec / Complement requirements

* `data/api/client-server/pushrules.yaml`:
  * `GET /pushrules/` → `{ "global": <Ruleset> }` (required `global`).
  * **`GET /pushrules/global/`** → the global ruleset itself (all rules of all kinds).
  * `GET/PUT/DELETE /pushrules/global/{kind}/{ruleId}`; `GET/PUT .../enabled`; `GET/PUT .../actions`.
    `kind` ∈ {override, underride, sender, room, content}. `PUT` must reject rule ids starting with `.`
    ("reserved for server-default rules") and slashes/backslashes; rules "MUST be enabled by default";
    `before`/`after` ordering parameters; 404 `M_NOT_FOUND` for unknown rules.
* Server-default rules (`content/client-server-api/modules/push.md` §"Predefined Rules", anchors
  `spec.matrix.org/v1.19/client-server-api/#default-override-rules`):
  * override: `.m.rule.master`, `.m.rule.suppress_notices`, `.m.rule.invite_for_me`, `.m.rule.member_event`,
    **`.m.rule.is_user_mention`**, **`.m.rule.is_room_mention`**, `.m.rule.tombstone`, `.m.rule.reaction`,
    `.m.rule.room.server_acl`, **`.m.rule.suppress_edits`**.
  * underride: `.m.rule.call`, `.m.rule.encrypted_room_one_to_one`, `.m.rule.room_one_to_one`, `.m.rule.message`,
    `.m.rule.encrypted`.
  * The legacy body/display-name content rules were removed in v1.17 ("the legacy default push rules that looked
    for mentions in the `body` of the event were removed").
* Complement `tests/csapi/push_test.go`:
  * `TestPushRuleCacheHealth` — after `PUT /pushrules/global/sender/{user}` with `actions:["dont_notify"]`,
    `GET /pushrules/` must show `global.sender.0.actions.0 == "dont_notify"`, and repeated GETs must not change it.
  * `TestPushSync` — an initial `/sync` must contain an `m.push_rules` account-data event with `content.global`;
    and every push-rule mutation (PUT rule / enabled / actions) must **wake up an incremental `/sync`** that
    delivers `m.push_rules`.

### (b) What Maelstrom does

* Routes (`capabilities.rs:72-147`): `GET /_matrix/client/v3/pushrules/` ✓, `GET /pushers`, `POST /pushers/set`,
  `PUT/GET/DELETE /pushrules/global/{kind}/{ruleId}`, `PUT/GET .../enabled`, `PUT/GET .../actions`.
  **`GET /_matrix/client/v3/pushrules/global/` is not implemented** (no route) → falls through to the router's
  default 404.
* `get_pushrules` (`capabilities.rs:521-554`): returns `{global: default_push_rules()}` with user rules merged in
  (replacing a default rule with the same id, else appended).
* `default_push_rules()` (`capabilities.rs:643-761`): a large set including `.m.rule.master`,
  `.m.rule.suppress_notices`, `.m.rule.invite_for_me`, `.m.rule.member_event`, `.m.rule.is_room_mention`,
  `.m.rule.tombstone`, `.m.rule.reaction`, `.m.rule.poll_response/.poll_start` (+ MSC3930 variants),
  `.m.rule.room.server_acl`, content `.m.rule.contains_user_name`, and underrides `.m.rule.call`,
  `.m.rule.room_one_to_one`, `.m.rule.encrypted_room_one_to_one`, `.m.rule.message`, `.m.rule.encrypted`,
  `.m.rule.poll_*` (+ MSC3930 variants).
  **Missing**: `.m.rule.is_user_mention`, `.m.rule.suppress_edits`.
  **Wrong**: `.m.rule.is_room_mention` uses `event_match` on `content.m\.mentions.room` with pattern `"true"`
  instead of the spec's `event_property_is` with `value: true`; `.m.rule.contains_user_name` is a legacy
  pre-v1.17 rule that the spec removed.
  (`.m.rule.suppress_notices` / `.m.rule.member_event` / `.m.rule.reaction` use `["dont_notify"]` where the spec
  writes `[]` — per `push.md:190-191` ("a rule with `["dont_notify"]` actions MUST be equivalent to a rule with an
  empty actions array") this is *not* a deviation.)
* `set_pushrule` (`capabilities.rs:787-819`): accepts any `rule_id` (no rejection of ids starting with `.` or
  containing `/`/`\`), ignores `before`/`after`, defaults `enabled` to `true` ✓, then calls `save_user_push_rules`
  → notifies `Notification::AccountData` ✓ (so `/sync` should wake up).
* `set_pushrule_enabled` / `set_pushrule_actions` (`capabilities.rs:867-936`): only mutate **user** rules; a
  request for a *server-default* rule (e.g. `.m.rule.message`) silently does nothing and returns `{}` — the spec
  says the actions of builtin rules can be changed ("This can be used to change the actions of builtin rules").
  `get_pushrule_enabled` / `get_pushrule_actions` only look at user rules and fall back to
  `{"enabled": true}` / `{"actions": ["notify"]}` — so `GET .../actions` for `.m.rule.suppress_notices` returns
  `["notify"]` instead of the default `[]`.
* `get_pushrule` (`capabilities.rs:821-849`): user rules first, then defaults ✓, else 404 `M_NOT_FOUND` ✓.
* The `m.push_rules` blob delivered by `/sync` (`sync.rs:1660-1798`) is built from a **different, smaller** default
  set (`sync.rs:1683-1725`: 3 override rules, 1 content rule, 1 underride rule) — inconsistent with
  `GET /pushrules/`.
* Push rules are stored in the private account-data namespace `_maelstrom.push_rules`, and the notifier only
  fires `Notification::AccountData` — the sync handler's account-data path is what wakes it up ✓.

### (c) Concrete gaps

1. **`GET /_matrix/client/v3/pushrules/global/` is missing** — a spec-mandated endpoint
   (`data/api/client-server/pushrules.yaml`, `getPushRulesGlobal`). Clients that use it get an empty-body 404. **[Major]**
2. **`.m.rule.is_user_mention` and `.m.rule.suppress_edits` are missing** from the server-default set
   (`capabilities.rs:643-761` vs `push.md` §"Default Override Rules"). Users therefore get no highlight for
   `m.mentions.user_ids` mentions and no suppression for edits. **[Major]**
3. **`.m.rule.is_room_mention` uses the wrong condition kind** (`event_match` + `pattern:"true"` instead of
   `event_property_is` + `value:true`) → room mentions never match. **[Major]**
4. **Legacy `.m.rule.contains_user_name` content rule still shipped** although v1.17 removed the body/display-name
   default rules; it also notifies+highlights on every message containing the localpart, which is a
   privacy/noise regression. **[Minor]**
5. **`/sync`'s `m.push_rules` content disagrees with `GET /pushrules/`** (`sync.rs:1683-1725` vs
   `capabilities.rs:643-761`) — a client that reads push rules from `/sync` gets a different rule set than one
   that reads `GET /pushrules/`. **[Major]**
6. **Builtin (server-default) rules cannot be modified** — `PUT .../actions` on `.m.rule.message` is a no-op
   (`capabilities.rs:913-936`); the spec explicitly allows changing builtin rule actions. **[Major]**
7. **`PUT /pushrules/global/{kind}/{ruleId}` does not reject reserved/invalid rule ids** (leading `.`, `/`, `\`)
   — spec: "If the string starts with a dot, the request MUST be rejected". A user can shadow a server-default
   rule id. **[Major]**
8. **`before`/`after` ordering parameters are ignored** — the new rule is always appended, so ordering semantics
   (spec §"If both `after` and `before` are provided…") are not honoured. **[Minor]**
9. **`GET .../enabled` and `GET .../actions` for a builtin rule return fabricated defaults**
   (`{"enabled": true}` / `{"actions": ["notify"]}`) rather than the rule's real default values. **[Minor]**
10. `GET /pushrules/` merges user rules into the *defaults* array rather than keeping a separate user-rules
    section — the spec's `Ruleset` (`definitions/push_ruleset.yaml`) has `override/underride/sender/room/content`
    arrays of rules, and user rules are supposed to be ordered before server defaults within each kind. The current
    merge appends user rules *after* the defaults, so a user rule never takes priority. **[Major]**

### (d) Suggested fix & risk

* Add `GET /_matrix/client/v3/pushrules/global/` returning the same ruleset as `GET /pushrules/`'s `global`.
  **Effort S, risk low.**
* Extract the canonical default rule set into `maelstrom-core` (`push.rs`) and use it from both
  `capabilities.rs` and `sync.rs`; add `.m.rule.is_user_mention` and `.m.rule.suppress_edits`; fix
  `.m.rule.is_room_mention` to `event_property_is`; drop `.m.rule.contains_user_name`. **Effort S, risk low**
  (rule content only; verify the highlight behaviour with `push_test.go`).
* Merge user rules *before* server defaults per kind and keep the two layers distinguishable (e.g. keep the
  `default: true/false` flag and sort by it). **Effort M, risk low-medium** (changes rule precedence).
* Reject `rule_id`s starting with `.` or containing `/`/`\` with `400 M_INVALID_PARAM`; implement `before`/`after`.
  **Effort S, risk low.**
* Allow `PUT .../actions` and `.../enabled` on builtin rules by storing per-user overrides of builtin rules.
  **Effort M, risk low.**

---

## 6. ERROR SEMANTICS + UNKNOWN ENDPOINTS

Code: `crates/maelstrom-api/src/router.rs`, `crates/maelstrom-core/src/matrix/error.rs`,
`crates/maelstrom-api/src/extractors/*`

### (a) Spec / Complement requirements

* `content/client-server-api/_index.md` §"Errors": every failed request returns
  `{"errcode": "<M_*>", "error": "<message>"}`; `errcode` and `error` MUST always be present.
* `M_UNRECOGNIZED` (same file): "The server did not understand the request. This is expected to be returned with
  a 404 HTTP status code if the endpoint is not implemented **or a 405 HTTP status code if the endpoint is
  implemented, but the incorrect HTTP method is used**."
* `content/server-server-api.md:87-89` — the same rule for the Server-Server prefix.
* Complement `tests/unknown_endpoints_test.go` (`TestUnknownEndpoints`) asserts, for every one of
  `/_matrix/unknown`, `/_matrix/client/unknown`, `/_matrix/client/v1/unknown`, `/_matrix/client/v3/room/unknown`,
  `/_matrix/federation/unknown`, `/_matrix/federation/v1/unknown`, `/_matrix/key/unknown`,
  `/_matrix/key/v2/unknown`, `/_matrix/media/unknown`, `/_matrix/media/v3/unknown`:
  **404 + `errcode == "M_UNRECOGNIZED"`**; and for `PUT /_matrix/client/v3/login`,
  `PUT /_matrix/federation/v1/version`, `PUT /_matrix/key/v2/query`, `PATCH /_matrix/media/v3/upload`:
  **405 + `errcode == "M_UNRECOGNIZED"`**.

### (b) What Maelstrom does

* `router.rs:11-49` builds the router by merging 25 sub-routers and **installs no `.fallback(...)` handler**;
  `src/main.rs:496-498` merges the federation and admin routers the same way. Axum's default behaviour therefore
  applies: unknown path → `404` with an **empty body**; known path with the wrong method → `405` with an
  **empty body** (and an `Allow` header).
* `ErrorCode::Unrecognized` (`error.rs:256-259`) and `MatrixError::unrecognized()` (`error.rs:144-150`) exist, but
  `unrecognized()` maps to **400**, not 404/405, and is **never called** (grep over `crates/maelstrom-api/src` →
  no call sites). `M_UNRECOGNIZED` is therefore never returned by the server.
* `MatrixError::into_response` (`error.rs:191-199`) is correct for handler-generated errors: status +
  `content-type: application/json` + `{"errcode":…,"error":…}` ✓.
* Handlers that use the `MatrixJson` extractor get Matrix-compliant `M_NOT_JSON`/`M_BAD_JSON`
  (`extractors/json.rs:40-77`) ✓.
* **11 handlers use the raw `axum::Json` extractor** (`keys.rs:97, 217, 394, 470, 547, 651, 755, 771`,
  `receipts.rs:51`, `register.rs:318`, `to_device.rs:52`) → a malformed body produces Axum's default
  `400` with a **plain-text** body (`Failed to deserialize the JSON body`), not `M_BAD_JSON`.
* Axum's `Query`/`Path` extractor rejections (e.g. `?timeout=abc` on `/sync`, a non-numeric `from` on
  `/keys/changes`) also produce plain-text 400s.
* Error-code inventory: `ErrorCode` (`error.rs:230-363`) covers the common codes ✓, but:
  * missing from the spec's list: `M_USER_LIMIT_EXCEEDED`, `M_USER_LOCKED`, `M_USER_SUSPENDED`,
    `M_THREEPID_MEDIUM_NOT_SUPPORTED`, `M_UNKNOWN_DEVICE`;
  * uses non-namespaced `M_*` codes that belong to MSCs: `M_DUPLICATE_ANNOTATION`, `M_NOT_YET_UPLOADED`,
    `M_CANNOT_OVERWRITE_MEDIA`, `M_UNKNOWN_POS`, `M_URL_NOT_SET`, `M_BAD_ALIAS` — per
    `content/client-server-api/_index.md` these should be namespaced (e.g. `org.matrix.msc…`);
  * `M_RESOURCE_LIMIT_EXCEEDED` is defined but never used; the spec requires it to carry `admin_contact`.
* `CORS` is `CorsLayer::permissive()` (`router.rs:47`) — allows any origin; not a spec issue but worth noting.

### (c) Concrete gaps

1. **Unknown endpoints return an empty-body 404, not `{"errcode":"M_UNRECOGNIZED"}`** — `router.rs:11-49` has no
   fallback handler. `tests/unknown_endpoints_test.go` fails on all 10 unknown-path cases. **[Blocker]**
2. **Wrong methods return an empty-body 405, not `M_UNRECOGNIZED`** — same root cause; the 4 wrong-method cases
   fail. **[Blocker]**
3. **11 endpoints return non-Matrix (plain-text) 400s** for malformed JSON because they use `axum::Json` instead
   of `MatrixJson` (`keys.rs:97` etc.). The spec requires `M_BAD_JSON`/`M_NOT_JSON` JSON bodies. **[Major]**
4. **Query/path deserialization errors are plain-text 400s** (Axum defaults) — e.g. `/sync?timeout=abc`,
   `/keys/changes?from=xyz`. **[Major]**
5. **Non-namespaced MSC error codes** are emitted under the `M_` namespace (`error.rs:340-358`) — the spec
   requires custom codes to be namespaced (`content/client-server-api/_index.md` §Errors). **[Minor]**
6. **Missing spec error codes**: `M_USER_LIMIT_EXCEEDED`, `M_USER_LOCKED`, `M_USER_SUSPENDED`,
   `M_THREEPID_MEDIUM_NOT_SUPPORTED`, `M_UNKNOWN_DEVICE`. **[Minor]**
7. `M_RESOURCE_LIMIT_EXCEEDED` is defined but never returned, and the spec requires it to carry `admin_contact`
   when used. **[Minor]**
8. **`M_UNRECOGNIZED` is mapped to 400** in `MatrixError::unrecognized()` (`error.rs:144-150`) even though the
   spec only defines it with 404/405 — if the fallback is implemented with this helper, the status would be wrong.
   **[Minor (latent)]**

### (d) Suggested fix & risk

* Add a global fallback to the router:
  `Router::new().fallback(not_found)` returning `404` + `{"errcode":"M_UNRECOGNIZED","error":"Unrecognized
  request"}`; and add a `method_not_allowed_fallback` via `Router::method_not_allowed_fallback` (Axum 0.7+) or a
  `map_response` layer that rewrites Axum's 405 into the Matrix body, returning `405` + `M_UNRECOGNIZED`.
  **Effort S, risk very low** (pure routing; also fix the admin/health routes if they should not be Matrix-shaped).
* Replace the remaining `axum::Json` extractors with `MatrixJson` and add a `Query` wrapper that maps rejections to
  `400 M_BAD_JSON`. **Effort S, risk low.**
* Rename the MSC error codes to their namespaced forms (`org.matrix.msc2716.bad_alias`, etc.) or keep them but mark
  them `unstable` in the docs. **Effort S, risk low.**

---

## Final prioritized findings

| # | Severity | Area | Finding | Location | Fix |
|---|---|---|---|---|---|
| 1 | Blocker | 6 | No router fallback: unknown paths return empty-body 404, wrong methods empty-body 405 (spec requires `M_UNRECOGNIZED`; `TestUnknownEndpoints` fails) | `crates/maelstrom-api/src/router.rs:11-49` | S |
| 2 | Blocker | 3 | Locally created events have no `origin`/`prev_events`/`depth`/`hashes`/`signatures` → malformed federation PDUs; `Pdu::sign()` never used | `crates/maelstrom-api/src/handlers/events.rs:187-207` | M |
| 3 | Blocker | 3 | Kick/ban/unban have no power-level checks (any joined member can ban anyone) | `crates/maelstrom-api/src/handlers/rooms.rs:1256-1427` | M |
| 4 | Blocker | 3 | Invite has no invite-level / target-ban check | `crates/maelstrom-api/src/handlers/rooms.rs:1093-1204` | S |
| 5 | Blocker | 3 | User's own `m.room.member` state event bypasses all auth checks (can self-join/ban) | `crates/maelstrom-api/src/handlers/events.rs:705-753` | M |
| 6 | Blocker | 3 | No `m.room.power_levels` change rules (PL-50 user can rewrite PL to 100) | `crates/maelstrom-api/src/handlers/events.rs:710-753` | M |
| 7 | Blocker | 3 | Federation receiver soft-fails signature/auth/auth-event/PL checks instead of dropping/rejecting | `crates/maelstrom-federation/src/receiver.rs:472-584, 969-1029` | S (after #2) |
| 8 | Blocker | 3 | `resolve_state` is not state resolution v2 (single conflicting key, no power ordering, no mainline ordering) | `crates/maelstrom-core/src/matrix/state.rs:90-226`, `receiver.rs:606-670` | L |
| 9 | Blocker | 4 | No `/_matrix/federation/v1/media/download|thumbnail` endpoints | `crates/maelstrom-federation/src/router.rs:46-58` | M |
| 10 | Blocker | 2 | `/keys/signatures/upload` discards all signatures → cross-signing non-functional | `crates/maelstrom-api/src/handlers/keys.rs:539-541` | M |
| 11 | Major | 1 | `full_state=true` is parsed and ignored | `crates/maelstrom-api/src/handlers/sync.rs:266` | S |
| 12 | Major | 1 | Incremental sync never applies the timeline `types`/`not_types` filter | `crates/maelstrom-api/src/handlers/sync.rs:303-313, 774, 795` | M |
| 13 | Major | 1 | `limited` is set true for every newly-joined room (spec: only when the filter truncated) | `crates/maelstrom-api/src/handlers/sync.rs:1125` | S |
| 14 | Major | 1 | Initial sync ignores `room.timeline.limit` and all type filters | `crates/maelstrom-api/src/handlers/sync.rs:930-967` | S |
| 15 | Major | 1 | `not_types`, `contains_url`, `senders`, `not_senders`, `rooms`, `not_rooms` filters unimplemented | `crates/maelstrom-api/src/handlers/sync.rs:288-333` | M |
| 16 | Major | 1 | `unread_notifications` always `{0,0}` — no notification/highlight counting | `crates/maelstrom-api/src/handlers/sync.rs:1012, 1196` | M |
| 17 | Major | 1 | `device_one_time_keys_count` and `device_unused_fallback_key_types` missing from the sync response (spec: Required) | `crates/maelstrom-api/src/handlers/sync.rs:128-140` | S |
| 18 | Major | 1 | `/sync`'s `m.push_rules` content ≠ `GET /pushrules/` content | `sync.rs:1683-1725` vs `capabilities.rs:643-761` | S |
| 19 | Major | 2 | `/keys/changes` `left` is hard-coded `[]` | `crates/maelstrom-api/src/handlers/keys.rs:460` | S |
| 20 | Major | 2 | `fallback_keys` unsupported (upload/storage/`device_unused_fallback_key_types`) | `crates/maelstrom-api/src/handlers/keys.rs:94-209` | M |
| 21 | Major | 2 | OTKs are claimed in `key_id DESC` order, not upload order (MSC4225) | `crates/maelstrom-storage/src/surreal/keys.rs:226` | M |
| 22 | Major | 2 | `/keys/claim` does not federate to remote users' servers | `crates/maelstrom-api/src/handlers/keys.rs:391-412` | M |
| 23 | Major | 2 | Bulk key-backup endpoints are stubs (silent backup loss) | `crates/maelstrom-api/src/handlers/keys.rs:751-779` | M |
| 24 | Major | 2 | Missing `PUT/DELETE /room_keys/version/{v}` and all `DELETE /room_keys/keys*` endpoints | `crates/maelstrom-api/src/handlers/keys.rs:70-88` | S |
| 25 | Major | 3 | No power-level check for non-state events (`events_default` unused) | `crates/maelstrom-api/src/handlers/events.rs:148-233` | S |
| 26 | Major | 3 | `capabilities` advertises room v12/v13 stable but `createRoom` rejects >v11; default is v10 (spec SHOULD be v12) | `capabilities.rs:151-175` vs `rooms.rs:235-245` | M |
| 27 | Major | 4 | `Content-Disposition` omitted when no filename (spec: required since v1.12) | `crates/maelstrom-api/src/handlers/media.rs:238-249` | S |
| 28 | Major | 4 | Thumbnails of remote media return 404 | `crates/maelstrom-api/src/handlers/media.rs:334-338` | S |
| 29 | Major | 4 | Thumbnails never set `Content-Disposition: inline` | `crates/maelstrom-api/src/handlers/media.rs:358-371` | S |
| 30 | Major | 4 | URL preview returns the raw remote `og:image` (not `mxc://`) and omits `og:image:width/height` | `crates/maelstrom-media/src/preview.rs:31-47` | M |
| 31 | Major | 4 | Media always served `inline`, including `text/html` / `image/svg+xml` (spec requires `attachment`) | `crates/maelstrom-api/src/handlers/media.rs:244-249` | S |
| 32 | Major | 4 | Remote media proxied over unauthenticated plain HTTPS; remote filename lost | `crates/maelstrom-api/src/handlers/media.rs:264-313` | M |
| 33 | Major | 5 | `GET /pushrules/global/` (all global rules) not implemented | `crates/maelstrom-api/src/handlers/capabilities.rs:100` | S |
| 34 | Major | 5 | `.m.rule.is_user_mention` and `.m.rule.suppress_edits` missing from the default rule set | `crates/maelstrom-api/src/handlers/capabilities.rs:643-761` | S |
| 35 | Major | 5 | `.m.rule.is_room_mention` uses `event_match` instead of `event_property_is` → never matches | `crates/maelstrom-api/src/handlers/capabilities.rs:660-665` | S |
| 36 | Major | 5 | User rules are appended *after* server defaults → user rules never take priority | `crates/maelstrom-api/src/handlers/capabilities.rs:529-551` | M |
| 37 | Major | 5 | Builtin rules' actions/enabled cannot be modified (spec allows it) | `crates/maelstrom-api/src/handlers/capabilities.rs:867-957` | M |
| 38 | Major | 5 | `PUT /pushrules/global/{kind}/{ruleId}` does not reject reserved/invalid rule ids | `crates/maelstrom-api/src/handlers/capabilities.rs:787-819` | S |
| 39 | Major | 6 | 11 handlers use raw `axum::Json` → plain-text 400 instead of `M_BAD_JSON` | `crates/maelstrom-api/src/handlers/keys.rs:97,217,394,470,547,651,755,771` | S |
| 40 | Major | 6 | Query/path extractor rejections produce plain-text 400s (not `M_BAD_JSON`) | e.g. `sync.rs:114-121`, `keys.rs:417-422` | S |
| 41 | Major | 1/3 | JSON content accepts non-integer / oversized integers (Complement `TestJson` expects 400 `M_BAD_JSON`) | `crates/maelstrom-api/src/handlers/events.rs:173-181` | S |
| 42 | Minor | 1 | Initial-sync `prev_batch` = first event's own position (duplicate on pagination) | `crates/maelstrom-api/src/handlers/sync.rs:938-941` | S |
| 43 | Minor | 1 | Invalid `since` token silently treated as 0 | `crates/maelstrom-api/src/handlers/sync.rs:260-264` | S |
| 44 | Minor | 1 | Long-poll disabled for initial syncs | `crates/maelstrom-api/src/handlers/sync.rs:386-395` | S |
| 45 | Minor | 1 | `summary` lacks `m.heroes` and is omitted unless a membership change occurred | `crates/maelstrom-api/src/handlers/sync.rs:202-208, 1161` | S |
| 46 | Minor | 1 | Global `account_data` re-sent in full on every sync instead of as a delta | `crates/maelstrom-api/src/handlers/sync.rs:1763-1791` | M |
| 47 | Minor | 2 | `/keys/query` ignores `timeout`; no `unsigned.device_display_name` | `crates/maelstrom-api/src/handlers/keys.rs:214-386` | S |
| 48 | Minor | 2 | Key-backup `count`/`etag` always `0` | `crates/maelstrom-api/src/handlers/keys.rs:617-618, 720` | S |
| 49 | Minor | 3 | `m.federate` / create-event auth rules not checked on the federation path | `crates/maelstrom-federation/src/receiver.rs:530-538` | S |
| 50 | Minor | 4 | No `Cache-Control` on any media response | `crates/maelstrom-api/src/handlers/media.rs:240-249` | S |
| 51 | Minor | 4 | Non-thumbnailable content served as the original instead of `400 M_UNKNOWN` | `crates/maelstrom-api/src/handlers/media.rs:364-371` | S |
| 52 | Minor | 4 | No thumbnail caching (re-decode on every request) | `crates/maelstrom-api/src/handlers/media.rs:344-372` | S |
| 53 | Minor | 5 | Legacy `.m.rule.contains_user_name` content rule still shipped (removed in v1.17) | `crates/maelstrom-api/src/handlers/capabilities.rs:694-698` | S |
| 54 | Minor | 5 | `before`/`after` push-rule ordering parameters ignored | `crates/maelstrom-api/src/handlers/capabilities.rs:787-819` | S |
| 55 | Minor | 5 | `GET .../enabled` / `.../actions` for builtin rules return fabricated values | `crates/maelstrom-api/src/handlers/capabilities.rs:892-957` | S |
| 56 | Minor | 6 | Non-namespaced MSC error codes emitted as `M_*` | `crates/maelstrom-core/src/matrix/error.rs:340-358` | S |
| 57 | Minor | 6 | Missing spec error codes (`M_USER_LIMIT_EXCEEDED`, `M_USER_LOCKED`, `M_USER_SUSPENDED`, …) | `crates/maelstrom-core/src/matrix/error.rs:230-363` | S |
| 58 | Minor | 6 | `M_UNRECOGNIZED` helper maps to 400, not 404/405 (latent bug once the fallback lands) | `crates/maelstrom-core/src/matrix/error.rs:144-150` | S |
| 59 | Minor | 6 | `M_RESOURCE_LIMIT_EXCEEDED` defined but unused (spec wants `admin_contact`) | `crates/maelstrom-core/src/matrix/error.rs:326-327` | S |

### Totals by severity

* **Blocker: 10**
* **Major: 31**
* **Minor: 18**
* **Total: 59**
