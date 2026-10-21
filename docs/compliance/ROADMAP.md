# Maelstrom → Matrix 1.x Full Compliance · Milestone Roadmap

> **Audience:** an agent (or engineer) picking this up cold, with no prior context.
> **Mission:** take Maelstrom from ~66 % Complement pass rate to **full conformance with the
> stable Matrix specification (v1.19)**, i.e. all four acceptance gates in §2 green.
> **Status of this document:** authoritative. `COMPLIANCE.md` (repo root) holds the *analysis*;
> this file holds the *execution plan*.
> **Last updated:** 2026-10-21

---

## 0. Read this first (10 minutes)

| Fact | Value | Where to verify |
|------|-------|-----------------|
| Target spec | **Matrix v1.19** (released 2026-07-08, latest stable) | `matrix-spec` tag `v1.19` |
| Project currently advertises | v1.1–v1.18 | `crates/maelstrom-api/src/handlers/versions.rs` |
| Stable endpoints in scope | 166 CS + 36 SS + 9 AS | `docs/compliance/endpoint-gap-v1.19.txt` |
| Endpoints already routed | **140 / 202 (CS+SS)** | same file, "SPEC COVERAGE SUMMARY" |
| Endpoints missing | **56**, plus 6 with the wrong HTTP method | same file |
| Registered routes total | 206 | `docs/compliance/route-inventory.md` |
| Complement stable gate | 107 test functions in `tests/csapi/` | `docs/compliance/complement-csapi-tests.txt` |
| Complement reported pass | 356/539 (66 %) — includes MSC tests | `VERIFICATION.md` (stale, see M0.4) |
| Rust toolchain | 2024 edition, Rust ≥ 1.85 | `Cargo.toml` |
| Framework | Axum 0.8, Tokio, SurrealDB v3 | `Cargo.toml` |
| Crates | `core` (types), `storage` (traits + surreal + mock), `media`, `api` (CS API), `federation` (SS API), `admin` (ops) | `crates/` |

**Do not start by writing code.** M0 (hygiene) and M1 (endpoint surface) are low-risk and
unblock measurement. M2 (federation) is where the real work is.

---

## 1. Repository map (what to touch for what)

```
crates/maelstrom-core/src/matrix/     types + protocol primitives (no I/O)
    event.rs        Pdu, StrippedEvent, sign(), content_hash(), reference_hash()
    signing.rs      canonical JSON, sign_event(), content_hash(), reference_hash()
    state.rs        resolve_state()  ← simplified v2, NOT compliant yet
    error.rs        MatrixError, ErrorCode (M_UNRECOGNIZED etc.)
    id.rs           mxid/room/event id helpers
    room.rs         event_type::* constants, membership/join-rule enums
crates/maelstrom-storage/src/
    traits.rs       Storage trait (the seam: surreal/ impl + mock.rs impl)
    surreal/        SurrealDB implementation, one file per domain
crates/maelstrom-api/src/
    router.rs       all CS API routes are merged here  ← add fallbacks here
    handlers/       one file per API area (see §3)
    extractors/     auth.rs (AuthenticatedUser), json.rs (MatrixJson)
    notify/         sync notifier (long-poll wakeups)
crates/maelstrom-federation/src/
    receiver.rs     inbound /send, /invite  ← auth checks live here
    sender.rs       outbound transaction queue
    joins.rs        make_join/send_join/make_leave/send_leave
    backfill.rs     /backfill
    state.rs        /state, /state_ids, /event
    key_server.rs   /_matrix/key/v2/*
    signing.rs      X-Matrix request signing
crates/maelstrom-admin/src/           non-Matrix admin API + SSR dashboard
db/schema.surql                         SurrealDB schema (SCHEMAFULL tables)
tests/                                  integration tests (one file per area)
scripts/complement.sh                   Complement runner
docs/compliance/                        analysis artefacts + tooling (this dir)
```

**Route registration convention:** every `crates/maelstrom-api/src/handlers/<area>.rs`
exposes `pub fn routes() -> Router<AppState>`; `router.rs` merges them. The same pattern
holds in `maelstrom-federation` and `maelstrom-admin`. Add routes in the handler file,
never in `router.rs`.

---

## 2. Definition of done — the four gates

| Gate | Criterion | Verification |
|------|-----------|--------------|
| **G1 Endpoints** | Every stable CS+SS endpoint of v1.19 is routed with the right method, and every stable AS endpoint exists | `python3 docs/compliance/diff_routes.py` → `missing: 0`, `method-mismatch: 0` |
| **G2 CS behaviour** | `complement tests/csapi/...` → 0 failures | `cd ../complement && go test -count=1 ./tests/csapi/...` |
| **G3 Federation behaviour** | `complement` federation tests green **and** Maelstrom interops with a real homeserver | `go test -count=1 ./tests/... ` + manual Synapse/Dendrite two-server test (§3, M2.7) |
| **G4 Discovery** | `GET /_matrix/client/versions` lists v1.1–v1.19 and an accurate `unstable_features` map | `curl …/_matrix/client/versions` |

**Explicitly out of scope** (Matrix 2.0 / unstable MSCs — do *not* let these block G1–G4):
MSC3706 partial state join, MSC4140 delayed events, MSC4155 invite filtering,
MSC4306 thread subscriptions, MSC4308 sliding sync, MSC2246 async upload, MSC4222
`state_after`, MSC3903 notifications, MSC3861 OIDC, MSC3939 lock/suspend,
MSC2716/4133 admin API, MSC2545 image packs, policy-room signing.

---

## 3. Milestones

| # | Milestone | Effort | Gate | Depends on |
|---|-----------|--------|------|------------|
| M0 | Compliance hygiene & measurement | 1–2 d | — | — |
| M1 | Stable endpoint surface completion | 3–5 d | G1 | M0 |
| M2 | **Authorization rules (local CS API)** | 3–5 d | G2 | M0 |
| M3 | Federation that actually federates | 8–12 d | G3 | M0, M2 |
| M4 | Sync conformance | 5–8 d | G2 | M0, M1 |
| M5 | E2EE / keys / device lists | 5–8 d | G2 | M0, M1 |
| M6 | Media, push rules, misc stable surface | 3–5 d | G2 | M1 |
| M7 | CI gates, docs, 1.0 readiness | 2–3 d | G2–G4 | M1–M6 |

M1, M4, M5, M6 are independent of each other and can run in parallel by different agents.
**M2 and M3 are the critical path** — M3's inbound auth work depends on M2's auth-rule
machinery, and both are security blockers (any member can currently ban anyone).

> Every task below is backed by a code-traced finding in
> [`behavior-audit.md`](behavior-audit.md) (59 findings: 10 Blocker / 31 Major / 18 Minor).
> Finding numbers in parentheses refer to that document's prioritised table.


---

### M0 — Compliance hygiene & measurement (1–2 days)

**Goal:** make the current state measurable and stop regressions from being invisible.

- [x] **M0.1 Router fallbacks** — `crates/maelstrom-api/src/router.rs`
  - Add `.fallback(...)` → `404` + `{"errcode":"M_UNRECOGNIZED","error":"Unrecognized request"}`
  - Add `.method_not_allowed_fallback(...)` → `405` + same body
  - *Why:* Complement `tests/csapi/unknown_endpoints_test.go` requires exactly this.
    Today Maelstrom returns an empty body (axum default) → the test fails.
  - *Check:* `MatrixError` in `crates/maelstrom-core/src/matrix/error.rs` already models
    `M_UNRECOGNIZED`; reuse it if it serialises to `{"errcode":…,"error":…}`.
  - **Done:** the fallback and the method-not-allowed fallback live in
    `maelstrom-api/src/router.rs`; the latter is additionally applied to the
    *outer* router in `src/main.rs` so the federation and admin routes answer
    with a JSON body too. `MatrixError::unrecognized_endpoint()` (404) and
    `MatrixError::method_not_allowed()` (405) carry the status codes;
    `MatrixError::unrecognized()` stays 400 for unrecognized *parameters*.
- [x] **M0.2 Close the unauthenticated admin hole** — `crates/maelstrom-api/src/handlers/appservice.rs:48-71`
  - `POST/GET/DELETE /_maelstrom/admin/v1/appservice[s]` are registered in the **client**
    router with no `AdminUser` extractor. Move them to `crates/maelstrom-admin` or add the
    `AdminUser` extractor (see `crates/maelstrom-admin/src/handlers/users.rs` for the pattern).
  - **Done:** the three routes moved to
    `crates/maelstrom-admin/src/handlers/appservice.rs` and are guarded by the
    `AdminUser` extractor; the client router now only serves the
    third-party protocol endpoints.
- [x] **M0.3 Fix discovery (G4)** — `crates/maelstrom-api/src/handlers/versions.rs`
  - Add `"v1.19"` to the `versions` array.
  - Change `UnstableFeatures` from an empty struct to `serde_json::Map<String, bool>` and
    populate it with the MSCs actually implemented, or delete those implementations:
    `org.matrix.msc3967`, `org.matrix.msc3706`, `org.matrix.msc4102`, `org.matrix.msc4186`,
    `org.matrix.msc3391`. *(Spec type: `object<string, boolean>` — matrix-spec
    `data/api/client-server/versions.yaml`.)*
  - **Done:** `v1.19` is advertised; `unstable_features` is a
    `BTreeMap<String, bool>` listing the MSCs that really have an
    implementation (`msc3967`, `msc3706`, `msc4102`, `msc3391`). `msc4186` has
    no code in the tree and is therefore **not** advertised.
- [ ] **M0.4 One source of truth for status** — `README.md`, `VERIFICATION.md`, `PROJECT.md`
  - The three docs disagree (63.8 % / 66.0 % / 62.5 %, totals 538 / 539 / 437).
  - Add `scripts/status.sh` that parses `complement-results.json` and writes
    `docs/compliance/STATUS.md`; make the three docs link to it instead of embedding numbers.
  - `VERIFICATION.md` references a `CLAUDE.md` that does not exist in the repo — add it or
    drop the reference.
- [ ] **M0.5 Lock the measurement tooling**
  - `docs/compliance/extract_spec_endpoints.py` + `docs/compliance/diff_routes.py` are the
    G1 gate. Add a CI step that runs them and fails if `missing > 0`.
  - The spec + Complement archives are vendored in `vendor/` — `make compliance-vendor`
    extracts them, no network needed.
- [x] **M0.6 Matrix-shaped error bodies** *(audit #39, #40, #41, #56–#58 — Major/Minor)*
  - 11 handlers take `axum::Json<serde_json::Value>` directly, so a malformed body yields a
    plain-text `400` instead of `M_BAD_JSON`. The repo already has the right extractor:
    `crates/maelstrom-api/src/extractors/json.rs` (`MatrixJson`) — switch those handlers
    to it. Same for path/query extractor rejections (`sync.rs:114-121`, `keys.rs:417-422`).
  - `M_UNRECOGNIZED` currently maps to **400** (`core/matrix/error.rs:144-150`); it must map
    to 404 (unknown path) / 405 (unknown method) — fix this *together with* M0.1 or the
    fallback will emit the wrong status.
  - Add the missing spec error codes: `M_USER_LIMIT_EXCEEDED`, `M_USER_LOCKED`,
    `M_USER_SUSPENDED`; namespace non-Matrix codes properly (`M_` vs `org.matrix.msc*`).
  - `TestJson` (stable suite) expects `400` + `M_BAD_JSON` for non-integer / oversized
    integers in event content — enforce in `handlers/events.rs:173-181`.
  - **Done (except the integer-range check):** all 11 handlers now use
    `MatrixJson`; a new `OptionalMatrixJson` extractor keeps the optional
    receipt body working while still reporting `M_BAD_JSON` for a malformed
    body. The three missing error codes were added to `ErrorCode` and the
    `M_UNRECOGNIZED` status codes were split into 404/405.

**Done when:** `diff_routes.py` runs from a clean checkout; `curl /_matrix/client/versions`
shows v1.19; no admin route is reachable without admin credentials; every JSON body parse
failure returns `M_BAD_JSON`; CI reports a compliance status page.

---

### M1 — Stable endpoint surface completion (3–5 days) → **G1**

**Goal:** 202/202 stable CS+SS endpoints routed with correct methods.

Source of truth: `docs/compliance/endpoint-gap-v1.19.txt` (regenerate after each batch).

**M1.1 Path-alias fixes (cheap, unblocks 2 stable tests)**
- [x] `GET /_matrix/client/v1/rooms/{roomId}/timestamp_to_event`
      *(Maelstrom only serves the `v3` path — `handlers/events.rs:129`. Complement's
      `tests/room_timestamp_to_event_test.go` calls the `v1` path.)*
- [x] `GET /_matrix/client/v1/room_summary/{roomIdOrAlias}` *(new handler;
      Complement `tests/room_summary_test.go`)*
      — implemented in `crates/maelstrom-api/src/handlers/room_summary.rs`
      (resolves aliases, reads the room state, returns the spec's
      `RoomSummary` schema).

**M1.2 High-value stable endpoints**
- [ ] `GET /_matrix/client/v3/rooms/{roomId}/context/{eventId}` — room context
      *(spec: `data/api/client-server/event_context.yaml`; also referenced by
      `apidoc_search_test.go`)*
- [ ] `GET|PUT|DELETE /_matrix/client/v3/profile/{userId}/{keyName}` — generic profile keys
      *(Maelstrom hardcodes `displayname` / `avatar_url` — `handlers/profile.rs:60-67`;
      add the generic route and delegate, keeping the old paths as aliases)*
- [x] `GET /_matrix/client/v3/pushrules/global/` — list all global rules of a kind
      *(currently only `GET /pushrules/` exists — `handlers/capabilities.rs:100`)*
      — implemented; both `/pushrules/global/` and `/pushrules/global` share
      one `merged_push_rules()` helper with `GET /pushrules/`.
- [ ] `GET /_matrix/client/v3/directory/list/room/{roomId}` — currently PUT only
- [ ] `GET /_matrix/client/v1/mutual_rooms` — new in v1.19 (MSC2666)
- [ ] `POST /_matrix/client/v3/rooms/{roomId}/report` and
      `POST /_matrix/client/v3/users/{userId}/report` — v1.13/v1.14 report content
      *(Maelstrom has only `report/{eventId}` — `handlers/reporting.rs:30`)*
- [ ] `POST /_matrix/client/v3/delete_devices` — bulk device delete
- [ ] `GET /.well-known/matrix/support`, `GET /.well-known/matrix/policy_server`
- [ ] `POST /_matrix/client/v1/appservice/{appserviceId}/ping`
- [ ] `PUT /_matrix/client/v3/directory/list/appservice/{networkId}/{roomId}`
- [ ] `GET /_matrix/client/v1/register/m.login.registration_token/validity`
- [ ] `POST /_matrix/client/v3/register/{email,msisdn}/requestToken`,
      `POST /_matrix/client/v3/account/password/{email,msisdn}/requestToken`
      *(these are identity-service backed; returning a well-formed 404/501 with a proper
      `M_*` error is acceptable if the identity API is not deployed — but the route must
      exist and answer with the documented error)*

**M1.3 Federation endpoints (routing half of M2 — behaviour still needed)**
- [ ] `GET /_matrix/federation/v1/event_auth/{roomId}/{eventId}`
- [ ] `GET /_matrix/federation/v1/user/devices/{userId}`
- [ ] `POST /_matrix/federation/v1/user/keys/claim`
- [ ] `GET /_matrix/federation/v1/make_knock/{roomId}/{userId}` +
      `PUT /_matrix/federation/v1/send_knock/{roomId}/{eventId}`
- [ ] `GET /_matrix/federation/v1/hierarchy/{roomId}`
- [ ] `GET /_matrix/federation/v1/media/download/{mediaId}`,
      `GET /_matrix/federation/v1/media/thumbnail/{mediaId}`
- [ ] `GET /_matrix/federation/v1/query/{queryType}` (generic; `profile`/`directory` exist)

**M1.4 Deprecated / legacy — decide explicitly, do not silently ignore**
- [ ] `/_matrix/client/v3/account/3pid*` — deprecated in the spec; implement only if the
      identity API is in scope, otherwise answer with the documented `M_*` error
- [ ] `GET /_matrix/client/v3/events`, `/events/{eventId}`, `/initialSync`,
      `/rooms/{roomId}/initialSync` — legacy r0-era sync, spec keeps them
- [ ] `POST /_matrix/media/v1/create`, `PUT /_matrix/media/v3/upload/{serverName}/{mediaId}`

**M1.5 Application Service API**
- [ ] The 9 AS endpoints (`POST /_matrix/app/v1/transactions/{txnId}`, `POST /_matrix/app/v1/ping`,
      `GET /_matrix/app/v1/users/{userId}`, `GET /_matrix/app/v1/rooms/{roomAlias}`,
      `GET /_matrix/app/v1/thirdparty/*`) — Maelstrom currently registers AS callbacks under
      custom prefixes (`crates/maelstrom-api/src/handlers/appservice.rs`).
      Either expose the spec paths or record the decision in `docs/compliance/STATUS.md`.

**Done when:** `python3 docs/compliance/diff_routes.py` prints `missing: 0` and
`method-mismatch: 0` for `--areas CS,SS` (and AS decisions recorded), and every new handler
has a unit test in `tests/`.

---

### M2 — Authorization rules, local CS API (3–5 days) → **G2**

**Today Maelstrom's own authorization model is largely absent.** These are security
blockers, not spec cosmetics: any joined member can ban anyone, and a PL-50 user can
rewrite the power levels to 100.

- [x] **M2.1 One shared authorization module** — new file, e.g.
      `crates/maelstrom-core/src/matrix/auth_rules.rs`
  - Implement the spec's authorization rules, room-version aware
    (matrix-spec `content/rooms/fragments/v1-auth-rules.md` … `v8-auth-rules.md`):
    `m.room.create`, `m.room.member` (join/leave/invite/kick/ban/unban/knock),
    `m.room.power_levels` (incl. *changes* to power levels), `m.room.join_rules`,
    `m.room.redaction`, `m.room.third_party_invite`.
  - API shape: `pub fn check(room_state: &RoomState, event: &Event) -> Result<(), AuthError>`
    where `AuthError` carries the spec's rejection reason and the `M_*` code to return.
  - Room versions to support: v1–v11 (v11 is the current spec default; see M2.7).
  - **Done:** `crates/maelstrom-core/src/matrix/auth_rules.rs` implements the
    v3/v8 rule set with a `RoomVersion` gate (knock / restricted join rules from
    v8), a `PowerLevels` view with the spec's defaults, and an `AuthError` that
    maps to `M_FORBIDDEN`. `RoomState` is built from the current state events;
    `handlers/util.rs::authorize_event()` is the single entry point used by the
    client-server API (and available to the federation receiver, see M3.2).
- [x] **M2.2 Power-level check for kick / ban / unban** *(audit #3 — Blocker)*
  - `handlers/rooms.rs:1256-1427` performs no PL check. Require
    `sender PL >= kick PL` (default 50) and `sender PL > target PL` (unless the sender
    has `kick` at the target's level).
  - Complement: `TestNotPresentUserCannotBanOthers`, `TestCannotKickLeftUser`,
    `TestCannotKickNonPresentUser`, `TestPowerLevels`.
  - **Done:** `kick_user`, `ban_user` and `unban_user` now call
    `authorize_member_event()` before storing the event; unbanning requires the
    `ban` level as well as the `kick` level.
- [x] **M2.3 Invite checks** *(audit #4 — Blocker)*
  - `handlers/rooms.rs:1093-1204`: enforce the inviter's `invite` PL, the target's
    `m.room.member` ban state, and the room's `join_rules`.
  - **Done:** `invite_to_room` authorises the invite through the shared rules
    (sender must be joined, `invite` power level, target must not be banned).
- [x] **M2.4 Own-member events** *(audit #5 — Blocker)*
  - `handlers/events.rs:705-753`: a user setting their **own** `m.room.member` bypasses
    every check (self-join into a private room, self-unban). Route it through M2.1 so
    join_rules / ban / restricted-room rules apply.
  - **Done:** `do_set_state` now calls `authorize_event()` for every state
    event, including `m.room.member`; the old blanket exemption for own
    member events was removed.
- [x] **M2.5 Power-level change rules** *(audit #6 — Blocker)*
  - `handlers/events.rs:710-753`: a user with PL ≥ 50 may rewrite `m.room.power_levels`.
    Per spec, changing power levels requires PL ≥ `state_default` **and** the new
    `users_default`/event levels may not exceed the sender's own level (except
    `events_default`/`state_default` up to the sender's PL). Also enforce the existing
    `users_default` rule for own-state-key events.
  - **Done:** `auth_rules::check_power_levels()` implements the spec's rules
    for `users_default` / `events_default` / `state_default` / `ban` / `redact` /
    `kick` / `invite`, the `events` and `notifications` maps and the `users`
    map (a user may not change another user's level at or above their own).
- [x] **M2.6 `events_default` for non-state events** *(audit #25 — Major)*
  - `handlers/events.rs:148-233` never checks `m.room.power_levels.events[event_type]` /
    `events_default` for message events.
  - **Done:** `send_event` authorises the event through the shared rules, so
    `events[event_type]` and `events_default` are enforced for message events.
- [ ] **M2.7 Room-version consistency** *(audit #26 — Major)*
  - `capabilities.rs:151-175` advertises room versions v12/v13 as stable while
    `rooms.rs:235-245` rejects >v11 and defaults to v10. Pick one: either implement
    v11/v12 creation (v11 is the current spec default) or stop advertising them.
- [x] **M2.8 Tests**
  - Add `tests/auth_rules_test.rs` with one case per rule (create, member join/leave/
    invite/kick/ban/unban, power-level change, redaction, third-party invite), and
    negative cases proving a low-PL user is rejected.
  - **Done:** `tests/auth_rules_test.rs` covers kick/ban/unban, join rules,
    own-member events, power-level changes and `events_default` through the
    client-server API, plus unit-level cases for the shared module. The module
    itself carries `#[cfg(test)]` unit tests for the individual rules.

**Done when:** every task in `docs/compliance/behavior-audit.md` §3 (State / Auth Rules) is
closed; `TestPowerLevels`, `TestNotPresentUserCannotBanOthers`, `TestCannotKickLeftUser`,
`TestCannotKickNonPresentUser` are green.

---

### M3 — Federation that actually federates (8–12 days) → **G3**

**This is the milestone that decides whether Maelstrom is a real homeserver.**
Today it can talk to Complement's mock server, but not to Synapse or Dendrite.

- [ ] **M3.1 Sign every locally created event** *(audit #2 — Blocker)*
  - `Pdu::sign()` exists (`crates/maelstrom-core/src/matrix/event.rs:236`) but has **no
    call sites** outside tests. Every event built in
    `handlers/events.rs:187,841,1075`, `handlers/rooms.rs:147,832`,
    `handlers/directory.rs:232`, `handlers/profile.rs:490`, `handlers/knock.rs:127`
    is created with `prev_events: None, depth: None, hashes: None, signatures: None`.
  - Refactor to **one** `create_event()` helper (put it in `handlers/util.rs`) that:
    1. resolves the room's forward extremities → `prev_events`,
    2. computes `depth = max(depth of prev_events) + 1`,
    3. sets `origin = server_name`,
    4. computes `hashes.sha256` and `signatures.<server>.<key_id>` via `Pdu::sign()`,
    5. strips `unsigned` for the federation form.
  - Apply it to **every** creation path. A partially-signed server is worse than an
    unsigned one (remote servers cache rejections).
- [ ] **M3.2 Enforce the authorization rules on inbound PDUs** *(audit #7 — Blocker)*
  - `crates/maelstrom-federation/src/receiver.rs:472-584,969-1029` currently **soft-fails**:
    invalid signatures, unknown auth events and failed auth checks are accepted with a
    `warn!`.
  - Reuse the M2.1 auth-rule module for inbound events (same rules, different entry point).
  - Reject (do not store) events that fail them, and reject PDUs missing
    `prev_events`/`depth`/`auth_events`.
  - Complement: `tests/federation_room_event_auth_test.go`, `tests/federation_event_auth_test.go`.
- [ ] **M3.3 Harden `X-Matrix` request authentication** *(audit #7 — Blocker)*
  - `receiver.rs:177,214`: a failed signature is logged and the request proceeds. Reject it.
- [ ] **M3.4 State resolution v2** *(audit #8 — Blocker, effort L)*
  - `crates/maelstrom-core/src/matrix/state.rs:90-226` is not state res v2: it handles a
    single conflicting key, has no power ordering and no mainline ordering.
  - Implement per matrix-spec `content/rooms/fragments/v2-state-res.md`: auth difference,
    reverse topological power ordering, iterative auth checks, mainline of the power-level
    event.
  - Use it in the receiver (conflicting forks) and in the `/state` + `/state_ids` responses.
- [ ] **M3.5 Correct federation response payloads**
  - `crates/maelstrom-federation/src/state.rs`: `event_id` is currently a stream-position
    approximation — it must be the real event ID.
  - `crates/maelstrom-federation/src/backfill.rs`: linear scan → real DAG walk
    (BFS over `prev_events` with depth limit).
  - `get_missing_events`: stream-range approximation → DAG walk.
  - `m.federate` / `create` event rules are not checked on the federation path
    (`receiver.rs:530-538`).
- [ ] **M3.6 Federation media endpoints** *(audit #9 — Blocker)*
  - `GET /_matrix/federation/v1/media/download/{mediaId}` and
    `.../media/thumbnail/{mediaId}` — currently absent from
    `crates/maelstrom-federation/src/router.rs:46-58`. Complement:
    `tests/federation_media_content_test.go`.
- [ ] **M3.7 Partial-state join (MSC3706) — already implemented; verify it**
  - `crates/maelstrom-federation/src/joins.rs` + `receiver.rs` implement the MSC3706
    phases. Confirm the 59 deferred `TestPartialStateJoin` tests are genuinely green
    (they are currently listed as "deferred" in `VERIFICATION.md` — that deferral may be
    obsolete).
- [ ] **M3.8 Interop test with a real homeserver** *(manual, do not skip)*
  - Compose a 2-server setup: Maelstrom + Synapse (and separately + Dendrite).
  - Assert: join over federation, message both ways, room state converges, redaction
    propagates, leaving works, and Maelstrom's device list reaches the remote server.
  - Store the compose file under `docker-compose.interop.yml` and document the steps.

**Done when:** `go test ./tests/... ` federation packages are green **and** the M3.8 interop
matrix passes in both directions.

---

### M4 — Sync conformance (5–8 days) → **G2**

**Target file:** `crates/maelstrom-api/src/handlers/sync.rs` (2163 lines) — the largest
handler and the biggest test category (88 tests, 40 %).

- [ ] **M4.1 Filters** *(audit #12, #14, #15 — Major)*
  - Implement the full filter surface from matrix-spec `data/api/client-server/sync.yaml`:
    `room_types`, `not_rooms`, `rooms`, `not_types`, `contains_url`, `types`, `senders`,
    `not_senders`, `rooms_include_leave`, `include_redundant_members`,
    `not_rooms_include_leave`, `lazy_load_members`, `room_state` / `event_state` /
    `state_key` / `event_format`.
  - Today **6 of 9** filter fields are unimplemented and the timeline `types`/`not_types`
    filter is only applied in `rooms.leave` — a `types:["m.room.message"]` filter still
    returns state events.
  - Apply one parsed `EventFilter` to both the timeline and the state section, in initial
    **and** incremental sync.
- [ ] **M4.2 `full_state`** *(audit #11 — Major)*
  - `sync.rs:266` parses `full_state=true` and discards it. When set, return the full
    `state` section for joined rooms while keeping the timeline windowed by `since`.
- [ ] **M4.3 `limited` / `prev_batch`** *(audit #13 — Major, #42 — Minor)*
  - `limited` is forced `true` for every newly-joined room; it must mean "the filter
    truncated the timeline". Fetch N+1 events to detect truncation.
  - Initial-sync `prev_batch` must be the token *before* the first returned event, not the
    event's own position (otherwise `/messages?from=prev_batch` re-delivers it).
- [ ] **M4.4 Timeline limit** *(audit #14 — Major)*
  - Initial sync hard-codes 10; it must use `room.timeline.limit`.
- [ ] **M4.5 `next_batch` monotonicity** — the token must advance whenever anything changed
    (events, account data, device lists, receipts, presence), even if only account data
    changed. *(This is the root cause recorded in `VERIFICATION.md` F1.)*
  - An invalid `since` token is currently treated as `0` while `is_initial` stays `false`
    (`sync.rs:260-264`) — return an error instead of a full replay.
- [ ] **M4.6 Sections** — `invite`, `join`, `knock`, `leave`, `ban`; leave events annotated
    with `unsigned.membership: "leave"`; `TestLeaveEventVisibility`,
    `TestOlderLeftRoomsNotInLeaveSection`, `TestSyncLeaveSection` are failing today.
- [ ] **M4.7 Ephemeral + device data** — `presence`, `account_data`, `to_device`,
    `device_lists.changed` / `fallback_keys` / `device_unused_fallback_key_types`,
    `device_one_time_keys_count` *(spec-Required, currently absent — audit #17)*,
    `unread_notifications` *(always `{0,0}` today — audit #16)*.
- [ ] **M4.8 Push rules in sync** *(audit #18 — Major)*
  - `/sync`'s `m.push_rules` and `GET /pushrules/` must return the **same** rule set.
    Extract one canonical `default_push_rules()` and use it in both places.
- [ ] **M4.9 Room summaries** — `joined_member_count`, `invited_member_count`,
    `m.heroes`, `m.name`, `m.avatar_url`, `m.topic`, `m.canonical_alias`; `summary` may
    only be omitted when it has not changed since the last sync.
- [ ] **M4.10 Long-poll** — subscribe to the notifier **before** checking for events
    (already fixed once — keep the invariant), handle `Lagged` broadcast errors, honour
    the client's `timeout` parameter (currently skipped for initial syncs).
- [ ] **M4.11 Historical membership** — `TestMembershipOnEvents` needs the membership *as of*
    a given event, not the current one.
- [ ] **M4.12 Account-data delta** *(Minor)* — global account data is re-sent in full on
    every sync; make it a delta.

**Done when:** every test in `docs/compliance/complement-csapi-tests.txt` whose name matches
`Sync|Gap|Filter|Leave` is green.

---

### M5 — E2EE, keys, device lists (5–8 days) → **G2**

**Target files:** `crates/maelstrom-api/src/handlers/keys.rs` (779 lines),
`crates/maelstrom-api/src/handlers/capabilities.rs` (1563 lines).

- [ ] **M5.1 Cross-signing signatures are discarded** *(audit #10 — Blocker)*
  - `handlers/keys.rs:539-541` accepts `POST /keys/signatures/upload` and throws the
    payload away → cross-signing is non-functional. Persist the signatures
    (`key_signature` table exists in `db/schema.surql`) and surface them in
    `/keys/query` and `/keys/device_signing/upload`.
- [ ] **M5.2 Key backup is a stub — implement it** *(audit #23, #24 — Major)*
  - `handlers/keys.rs:751-779` returns hard-coded `{"count":0,"etag":"0"}` /
    `{"sessions":{}}`; `db/schema.surql` has **no** backup table.
  - Add storage (`key_backup` + `key_backup_session`, or a `m.key_backup` account-data
    document per v1.19/MSC4287), then implement the 13 `room_keys/*` endpoints including
    the missing `PUT`/`DELETE` on `room_keys/version/{version}` and `DELETE` on
    `room_keys/keys[...]`.
  - *Gate:* `TestE2EKeyBackupReplaceRoomKeyRules` (stable suite).
- [ ] **M5.3 `/keys/changes` `left`** *(audit #19 — Major)* — always `[]` today
    (`handlers/keys.rs:460`). Populate users whose device lists changed and went away.
- [ ] **M5.4 `fallback_keys`** *(audit #20 — Major)* — unsupported anywhere (upload,
    storage, and the sync field `device_unused_fallback_key_types`).
- [ ] **M5.5 One-time key claim order** *(audit #21 — Major)*
  - `storage/surreal/keys.rs:226` claims in `key_id DESC` order; MSC4225 requires
    **upload order** (oldest first) so a client cannot starve a peer.
- [ ] **M5.6 `/keys/claim` must federate** *(audit #22 — Major)*
  - `handlers/keys.rs:391-412` only serves local users; for remote users it must call
    `POST /_matrix/federation/v1/user/keys/claim` on their server.
- [ ] **M5.7 Device-list tracking** — track per-encrypted-room device membership so that
    `device_lists.changed` is accurate; ensure remote device-list updates reach `/sync`
    within the 5 s Complement window (`TestDeviceListUpdates`).
- [ ] **M5.8 One-time keys** — counts in `/sync` (`device_one_time_keys_count`),
    `one_time_key` scarcity handling, `key_id` format per spec.
- [ ] **M5.9 `/keys/query` polish** *(Minor)* — honour `timeout`; return
    `unsigned.device_display_name`.
- [ ] **M5.10 Key-backup `count`/`etag`** *(Minor)* — currently always `0`.

**Done when:** `TestUploadKey`, `TestKeyChangesLocal`, `TestDeviceListUpdates`,
`TestE2EKeyBackupReplaceRoomKeyRules`, `TestUserQueryKeys`, `TestKeyClaimOrdering` are green.

---

### M6 — Media, push rules, remaining stable surface (3–5 days) → **G2**

- [ ] **M6.1 Media correctness**
  - `GET /_matrix/media/v3/config` must return the live config (mimetypes, size limits) —
    Complement `TestMediaConfig`.
  - Authenticated media: `GET /_matrix/client/v1/media/{download,thumbnail,config,preview_url}`
    — present, verify shape.
  - Federation media: `GET /_matrix/federation/v1/media/{download,thumbnail}/{mediaId}`
    (M3.6) + `TestContentCSAPIMediaV1`, `tests/federation_media_content_test.go`.
  - **Always emit `Content-Disposition`** — it is required since v1.12 and is currently
    omitted when there is no filename *(audit #27 — Major)*.
  - **Serve `text/html` / `image/svg+xml` as `attachment`** — today everything is served
    `inline`, which is a stored-XSS vector *(audit #31 — Major)*.
  - Thumbnails: set `Content-Disposition: inline`; remote thumbnails currently 404
    *(audit #28, #29 — Major)*.
  - Add `Cache-Control` to media responses *(audit #50 — Minor)*.
  - Non-thumbnailable content must return `400 M_UNKNOWN`, not the original
    *(audit #51 — Minor)*.
  - Cache thumbnails instead of re-decoding on every request *(audit #52 — Minor)*.
- [ ] **M6.2 URL preview** *(audit #30 — Major)*
  - `media/src/preview.rs:31-47` returns the **raw remote `og:image` URL**; the spec
    requires the preview image to be **cached and served as an `mxc://` URI**, with
    `og:image:width` / `og:image:height`.
  - Keep the SSRF allowlist (no private addresses, no redirects to private ranges).
- [ ] **M6.3 Remote media proxying** *(audit #32 — Major)*
  - Remote media is proxied over unauthenticated plain HTTPS and the remote filename is
    lost. Use the federation media endpoints (M3.6) or at least preserve the filename and
    set the correct content type.
- [ ] **M6.4 Push rules** *(audit #33–#38, #53–#55)*
  - `default_push_rules()` (`handlers/capabilities.rs:643`) must contain the spec's
    mandated defaults: add `.m.rule.is_user_mention` and `.m.rule.suppress_edits`; fix
    `.m.rule.is_room_mention` to use `event_property_is` (it uses `event_match`, so it
    **never matches**); drop the legacy `.m.rule.contains_user_name` (removed in v1.17).
  - Add `GET /pushrules/global/`.
  - **User rules must take priority over server defaults** — today they are appended
    *after* them (`capabilities.rs:529-551`), so they never win.
  - Builtin rules' `actions`/`enabled` must be modifiable (spec allows it), and
    `GET .../enabled` / `.../actions` must not fabricate values.
  - `PUT /pushrules/global/{kind}/{ruleId}` must reject reserved/invalid rule ids.
  - Honour `before`/`after` ordering parameters.
  - Room-upgrade rule carry-over (`TestPushRuleRoomUpgrade`).
- [ ] **M6.5 Third-party / appservice stubs** — `/thirdparty/protocols`,
    `/thirdparty/location`, `/thirdparty/user` return hard-coded empties. Either implement
    against the `appservice` table or document the limitation.
- [ ] **M6.6 Misc** — `POST /_matrix/client/v3/refresh` always fails `M_UNKNOWN`
    (`handlers/capabilities.rs`); implement refresh-token rotation or return the documented
    `M_UNKNOWN` with the right shape. `GET /voip/turnServer` returns an empty TURN config.

**Done when:** `TestMediaConfig`, `TestUrlPreview`, `TestContent`, `TestPushSync`,
`TestPublicRooms`, `TestRoomAlias` are green.

---

### M7 — CI gates, docs, 1.0 readiness (2–3 days) → **G2–G4**

- [ ] **M7.1 Make CI enforce compliance**
  - `.github/workflows/ci.yml` currently runs Complement with `continue-on-error: true` and
    only `./tests/csapi/...`. Change to:
    1. a **blocking** job for `tests/csapi/...` (G2),
    2. a second blocking job for the federation packages (G3),
    3. a third job running `docs/compliance/diff_routes.py` and failing on
       `missing > 0` (G1),
    4. keep the MSC suite as a *non-blocking* informational job.
- [ ] **M7.2 Docs** — update `README.md` (status table), `VERIFICATION.md` (failure list →
    milestone state), `PROJECT.md` (phase 10/11 checkboxes) from the single generated
    status page (M0.4). Remove the "deferred" claims that are now false.
- [ ] **M7.3 Release checklist** — `1.0` requires: G1–G4 green, `cargo clippy` clean,
    `cargo test` green, admin dashboard functional, deployment guide written, and a
    CHANGELOG.

---

## 4. Working rules for this codebase

Taken from `VERIFICATION.md` ("Critical Rules") plus conventions visible in the tree — obey them:

1. **Tests live in `tests/`, never inline in source files.** One file per area
   (`tests/auth_test.rs`, `tests/sync_test.rs`, …); integration tests boot the real Axum
   router against mock storage (`crates/maelstrom-storage/src/mock.rs`).
2. **`cargo fmt --all && cargo clippy --all-targets --all-features` must be clean** (CI
   sets `RUSTFLAGS: -Dwarnings`).
3. **`cargo test --all` must pass before running Complement.**
4. **Always `RecordId::new()`** — never `type::thing()` or manual string splitting
   (SurrealDB v3 API).
5. **Never change the sync timeline limit from 20** — Complement sync tests are calibrated
   to it.
6. **Never change the `require_membership` fallback** — departed users depend on it.
7. **Do not speculatively change working handler behaviour** — add new code paths; keep the
   old ones and add tests.
8. **Test each change individually**: `make complement-filter FILTER=TestXxx`, then re-run
   the whole suite to check for regressions.
9. **Follow the layering**: `api` → `storage` (trait) → `storage/surreal` + `storage/mock`.
   If you add a storage method, implement it in **both** backends or the mock-based tests
   break.
10. **Structured logging** via `tracing`, never `println!`.

---

## 5. How to run things

```bash
# Build / test
cargo build --release
cargo fmt --all && cargo clippy --all-targets --all-features
cargo test --all

# Services (SurrealDB + RustFS)
docker compose -f docker-compose.dev.yml up -d
cp config/example.toml config/local.toml

# Complement (needs Docker + Go ≥ 1.22)
make complement                        # everything
make complement-filter FILTER=TestSync # one test / prefix
make complement-report                 # re-render the report from the last run
cd ../complement && go test -count=1 -timeout 30m ./tests/csapi/...   # the G2 gate
# (or against the vendored copy: cd vendor/complement-main && go test ./tests/csapi/...)

# G1 gate (endpoint coverage) — works offline, archives are vendored
make compliance-vendor   # extract vendor/matrix-spec-v1.19 + vendor/complement-main
make compliance-extract  # regenerate docs/compliance/endpoints-v1.19.json
make compliance-diff     # regenerate the gap report; prints the coverage line
```

**Upstream sources are vendored** in `vendor/` (see `vendor/README.md`): the matrix-spec
v1.19 archive and a Complement snapshot. No network access is needed for the compliance
tooling; `make compliance-vendor` extracts them.

**Environment caveat:** the sandbox used to produce this roadmap has a Termux/Android Rust
toolchain that cannot link (`-llog`/`-lunwind` missing) and no Docker, so `cargo test` and
Complement **cannot run there** — validate in CI or on a normal Linux x86_64/aarch64 host.


---

## 6. Where the evidence lives

| File | What it contains |
|------|------------------|
| `COMPLIANCE.md` (root) | Full analysis: definition of done, measured state, gap tables, phase plan |
| `docs/compliance/ROADMAP.md` (this file) | Execution plan: milestones, tasks, acceptance gates, agent split |
| `docs/compliance/behavior-audit.md` | Code-traced behavioural audit: 6 areas, 59 findings (10 Blocker / 31 Major / 18 Minor) with `file:line` + spec + Complement test per finding |
| `docs/compliance/endpoint-gap-v1.19.txt` | Machine diff: missing / wrong-method / extra endpoints |
| `docs/compliance/endpoints-v1.19.json` | The 235 stable spec endpoints (CS 166, SS 36, AS 9, ID 23, PG 1) |
| `docs/compliance/route-inventory.md` | All 206 routes with implemented-vs-stub classification |
| `docs/compliance/complement-csapi-tests.txt` | The 107 stable Complement test functions (the G2 gate list) |
| `docs/compliance/extract_spec_endpoints.py` | Extracts the spec endpoint inventory from a matrix-spec checkout |
| `docs/compliance/diff_routes.py` | Diffs spec vs implementation (the G1 gate) |
| `vendor/` | Vendored matrix-spec v1.19 + Complement archives (see `vendor/README.md`) |
| `VERIFICATION.md` | Historical failure analysis (partly stale — reconcile in M0.4) |
| `PROJECT.md` | Phase plan; Phase 10/11 checkboxes are the tracking surface |

External references (vendored in `vendor/`, re-fetch if you need a newer version):

* `matrix-spec` v1.19 — `https://github.com/matrix-org/matrix-spec` (tag `v1.19`);
  OpenAPI per endpoint under `data/api/{client-server,server-server,application-service}/`;
  narrative spec under `content/`.
* Complement — `https://github.com/matrix-org/complement`; stable tests in `tests/csapi/`,
  MSC tests in `tests/mscXXXX/`, federation tests in `tests/federation_*.go`.
* Spec web pages — `https://spec.matrix.org/v1.19/client-server-api/` and
  `https://spec.matrix.org/v1.19/server-server-api/`.


---

## 7. Risk register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Signing + auth-rules work touches every event path | Regressions everywhere | Land behind a single `create_event()` helper (M3.1) and one auth-rules module (M2.1); run Complement after each of M3.1 / M3.2 / M3.3 separately |
| **Turning on auth rules breaks existing flows** (kick/ban/invite/PL changes now enforced) | Previously "working" client flows start returning 403 | Ship M2 behind a config flag first (`config.example.toml`), default **on** for 1.0; add regression tests for every permission the admin dashboard uses |
| Rejecting previously-accepted inbound events breaks Complement flows | Federation tests fail differently | Pair the reject path with the auth-rule implementation in the same change; use `make complement-filter FILTER=Federation` |
| State res v2 is subtle | Divergent state between servers | Port the algorithm structure from Synapse/Conduit (reference only, do not copy code) and add property-style unit tests over fork shapes |
| SurrealDB schema is `SCHEMAFULL` | New fields (key backup, device lists) need a migration | Add fields via `DEFINE FIELD IF NOT EXISTS` in `db/schema.surql`; keep migrations additive and idempotent |
| The 91 "deferred" MSC tests may not actually be deferred | Milestone estimates wrong | Re-measure the full suite in M0 before committing to M3–M6 estimates |
| CI currently hides Complement failures | False green | M7.1 (do it early, even if in parallel with M1) |
| The behavioural audit was produced by reading code, not running it | Some findings may be wrong | Spot-check every Blocker before implementing (they are all one-line `file:line` reads) |

---

## 8. Suggested agent split (if running in parallel)

| Agent | Milestones | Files owned |
|-------|-----------|-------------|
| A | M0 + M7 | `router.rs`, `versions.rs`, `handlers/appservice.rs`, `core/matrix/error.rs`, `extractors/json.rs`, `ci.yml`, docs |
| B | M1 (CS part) + M6 | `handlers/{events,profile,capabilities,directory,reporting,media}.rs`, `media/src/preview.rs` |
| C | M1 (SS part) + M3 | `crates/maelstrom-federation/**`, `core/matrix/{event,state}.rs` |
| D | M2 + M4 | `core/matrix/auth_rules.rs` (new), `handlers/{events,rooms}.rs`, `handlers/sync.rs` |
| E | M5 | `handlers/keys.rs`, `storage/surreal/keys.rs`, `db/schema.surql` |

Coordination rules:

* B and E both touch `handlers/capabilities.rs` — B owns push rules, E owns keys; split the
  file by line range and merge carefully.
* D and B both touch `handlers/events.rs` — D owns auth-rule checks, B owns routing; land
  D's changes first (they are security blockers) and rebase B.
* C owns `core/matrix/{event,state}.rs` and the whole federation crate; nobody else edits
  them without asking C.
* E owns `db/schema.surql` (key-backup tables) — D must not add tables without coordinating.

**Sequencing recommendation for a single agent:** M0 → M1 → M2 → M3 → M4 → M5 → M6 → M7.
The audit's "quick wins" (M0.1, M0.6, M4.2–M4.4, M6.1 Content-Disposition, M6.4 push-rule
defaults) are all S-effort and each unblocks at least one stable Complement test — do them
first even if the milestone order says otherwise.

