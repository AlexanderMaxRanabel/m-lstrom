# Matrix 1.x Full Compliance Plan

> Maelstrom homeserver — path to full compliance with the **stable Matrix v1.x specification**
> Target spec: **Matrix v1.19** (latest stable release of `matrix-org/matrix-spec`)
> Generated: 2026-10-21

---

## 1. What "Matrix 1 full compliance" means

Matrix "v1" is the stable, released specification line (v1.1 → v1.19). Anything behind
`/_matrix/client/unstable/...` or an un-merged MSC is *not* part of Matrix 1, and features
that were MSCs but have since been **accepted into the stable spec** (relations, threads,
spaces, knocking, read markers, reports, mutual rooms, room summary, device management,
policy rooms, …) *are* part of Matrix 1 and must be implemented on their **stable paths**.

Full compliance = all four of the following hold:

| # | Criterion | How it is measured | Current |
|---|-----------|--------------------|---------|
| C1 | **Endpoint completeness** | Every endpoint in the stable CS API (166), SS API (36) and AS API (9) of matrix-spec v1.19 is routed and behaves per spec | 140/202 CS+SS routed (69%) |
| C2 | **Behavioural conformance (CS)** | `complement` `tests/csapi/...` — the stable-spec suite — is 100% green | 356/539 reported (66%), of which ~91 are MSC/unstable |
| C3 | **Behavioural conformance (SS)** | `complement` `tests/federation_*.go` + `msc*` federation tests green; outbound PDUs are signed and pass real servers' auth rules | federation largely soft-fail |
| C4 | **Discovery correctness** | `GET /_matrix/client/versions` advertises exactly the supported stable versions and the `unstable_features` that are actually implemented | advertises v1.1–v1.18, `unstable_features: {}` |

### 1.1 Delta v1.18 → v1.19 (released 2026-07-08)

Maelstrom advertises v1.18; v1.19 is the current stable release. What v1.19 adds
(`content/changelog/v1.19.md` in the spec source):

| Change | Matrix 1 requirement? | Action |
|--------|----------------------|--------|
| `GET /_matrix/client/v1/mutual_rooms` (MSC2666) | **yes** — new stable endpoint | implement |
| `m.key_backup` account data (MSC4287) | yes — clarifies backup semantics | implement with the key-backup work |
| Room directory ordering is server-defined (MSC4423) | yes — relaxes a requirement | none (Maelstrom may keep largest-first) |
| Image packs `m.room.image_pack` / `m.image_pack.rooms` (MSC2545) | yes — new stable event types | optional: treat as ordinary state events |
| Encrypted history sharing (MSC4268) | yes — clarifies key sharing | optional |
| `unsigned.replaces_state` in client events | yes — response field | add to edited events |
| Pagination tokens for `GET /rooms/{roomId}/context/{eventId}` | yes | part of the room-context work |

Explicitly **out of scope** for "Matrix 1 full compliance" (unstable MSCs, optional work):
MSC3706 partial state join, MSC4140 delayed events, MSC4155 invite filtering, MSC4306 thread
subscriptions, MSC4308 sliding sync, MSC2246 async upload, MSC4222 `state_after`, MSC3903
notifications, MSC3861 OIDC login, MSC3939 account lock/suspend, MSC2716+ admin API.
These are *Matrix 2.0* / optional surface area and must not block C1–C4.

---

## 2. Current state (measured, not estimated)

| Metric | Value | Source |
|--------|-------|--------|
| Workspace crates | 6 (`core`, `storage`, `media`, `api`, `federation`, `admin`) | `Cargo.toml` |
| Rust LOC | ~34 k | `find . -name '*.rs' \| xargs wc -l` |
| Registered routes | 206 (method+path) | `docs/compliance/route-inventory.md` |
| Stable spec endpoints (CS+SS, v1.19) | 202 | `docs/compliance/endpoints-v1.19.json` |
| Stable endpoints routed | **140 / 202 (69 %)** | `diff.py` output |
| Stable endpoints missing | **56** (+6 with wrong method) | below |
| Handlers that are stubs/partial | 25 | `impl-endpoints.md` |
| Complement reported | 356/539 (66 %) | `VERIFICATION.md` |
| Complement stable-only (`tests/csapi`) | 107 test funcs (237 incl. subtests) | complement main |
| Unit/integration tests | 107 | `tests/` |
| Advertised spec version | v1.18 (v1.19 released) | `handlers/versions.rs` |
| `unstable_features` advertised | `{}` (empty) although MSC3967/3706/4102/4186/3391 are implemented | `handlers/versions.rs` |

---

## 3. Gap analysis

### 3.1 Endpoint gaps (C1)

Legend: **P0** = required for stable spec / breaks real clients or the Complement stable suite ·
**P1** = stable spec endpoint, cheap to add · **P2** = optional or legacy/deprecated.

#### 3.1.1 Client-Server API — missing (36)

| Prio | Method | Path | Notes / spec |
|------|--------|------|--------------|
| P0 | GET | `/_matrix/client/v3/rooms/{roomId}/context/{eventId}` | Room context; Complement `apidoc_search_test.go` + clients (Element) rely on it |
| P0 | GET | `/_matrix/client/v1/rooms/{roomId}/timestamp_to_event` | **Wrong prefix**: Maelstrom only serves `/_matrix/client/v3/...`; Complement `room_timestamp_to_event_test.go` calls the `v1` path |
| P0 | GET | `/_matrix/client/v1/room_summary/{roomIdOrAlias}` | Complement `room_summary_test.go` calls the `v1` path; also needed by restricted rooms |
| P0 | GET/POST | `/_matrix/client/v3/rooms/{roomId}/report`, `/_matrix/client/v3/users/{userId}/report` | v1.13/v1.14 report content; Maelstrom only has `report/{eventId}` |
| P0 | GET/PUT/DELETE | `/_matrix/client/v3/profile/{userId}/{keyName}` | Generic profile key endpoints (v1.12/v1.16); Maelstrom hardcodes `displayname`/`avatar_url` |
| P0 | GET | `/_matrix/client/v3/pushrules/global/` | List all global rules of a kind — required by pushrules spec + clients |
| P0 | GET | `/_matrix/client/v1/mutual_rooms` | v1.19 (MSC2666) |
| P1 | GET | `/.well-known/matrix/support` | v1.10 support info |
| P1 | GET | `/.well-known/matrix/policy_server` | v1.18 |
| P1 | GET | `/_matrix/client/v3/admin/whois` | v1.19 (MSC4133-style admin API) — optional |
| P1 | GET/PUT | `/_matrix/client/v1/admin/{lock,suspend}/{userId}` | MSC3939 — **Matrix 2.0, out of scope for C1** |
| P1 | GET | `/_matrix/client/v1/auth_metadata` | OAuth 2.0 server metadata (MSC3861) — optional |
| P1 | POST | `/_matrix/client/v1/login/get_token` | MSC3861 — optional |
| P1 | POST | `/_matrix/client/v1/appservice/{appserviceId}/ping` | appservice ping (v1.7) |
| P1 | PUT | `/_matrix/client/v3/directory/list/appservice/{networkId}/{roomId}` | appservice room directory |
| P1 | POST | `/_matrix/client/v3/delete_devices` | bulk device delete (v1.11) |
| P1 | GET | `/_matrix/client/v1/register/m.login.registration_token/validity` | registration token validity |
| P1 | POST | `/_matrix/client/v3/register/{email,msisdn}/requestToken` | 3pid registration tokens (identity service) |
| P1 | POST | `/_matrix/client/v3/account/password/{email,msisdn}/requestToken` | 3pid password reset tokens |
| P2 | GET/POST/… | `/_matrix/client/v3/account/3pid*` | 3PID account API — deprecated in spec |
| P2 | GET | `/_matrix/client/v3/events`, `/events/{eventId}`, `/initialSync`, `/rooms/{roomId}/initialSync` | legacy r0-era sync; spec keeps them but no modern client needs them |
| P2 | POST | `/_matrix/client/v3/notifications` | MSC3903 — unstable, out of scope |
| P2 | POST | `/_matrix/media/v1/create`, PUT `/_matrix/media/v3/upload/{serverName}/{mediaId}` | legacy content-repo upload variants |

#### 3.1.2 Server-Server API — missing (12)

| Prio | Method | Path | Notes |
|------|--------|------|-------|
| P0 | GET | `/_matrix/federation/v1/event_auth/{roomId}/{eventId}` | Auth chain for an event — required by SS API; Complement `federation_event_auth_test.go` |
| P0 | GET | `/_matrix/federation/v1/user/devices/{userId}` | device list for a user (SS) — needed for device lists |
| P0 | POST | `/_matrix/federation/v1/user/keys/claim` | one-time key claim over federation |
| P0 | GET | `/_matrix/federation/v1/make_knock/{roomId}/{userId}` + PUT `/_matrix/federation/v1/send_knock/{roomId}/{eventId}` | knocking over federation (v1.1) — Complement `knocking_test.go` |
| P0 | GET | `/_matrix/federation/v1/hierarchy/{roomId}` | space hierarchy over federation (v1.2) |
| P0 | GET | `/_matrix/federation/v1/media/download/{mediaId}`, `.../media/thumbnail/{mediaId}` | federation media (v1.11) — Complement `federation_media_content_test.go` |
| P1 | GET | `/_matrix/federation/v1/query/{queryType}` | generic query endpoint (profile/directory are already routed separately) |
| P1 | PUT | `/_matrix/federation/v1/exchange_third_party_invite/{roomId}` | 3pid invite exchange |
| P1 | PUT | `/_matrix/federation/v1/3pid/onbind` | 3pid onbind callback |
| P2 | POST | `/_matrix/policy/v1/sign` | policy room signing (MSCxxxx) — optional |

#### 3.1.3 Method mismatches (6)

| Path | Maelstrom | Spec | Fix |
|------|----------|------|-----|
| `GET /_matrix/client/v3/directory/list/room/{roomId}` | only PUT | GET+PUT | add GET (public room listing per-room) |
| `GET/PUT/DELETE /_matrix/client/v3/room_keys/version/{version}` | only GET | GET+PUT+DELETE | implement backup version CRUD |
| `GET/PUT/DELETE /_matrix/client/v3/room_keys/keys[...]` | only GET+PUT | +DELETE | key backup is currently a **stub** — implement or 501 |

#### 3.1.4 Application Service API

`POST /_matrix/app/v1/transactions/{txnId}` (event push) and `POST /_matrix/app/v1/ping` are
registered under custom prefixes; the AS API paths themselves are missing (9 spec endpoints).
Low priority: AS API is only needed for bridges.

---

### 3.2 Behavioural / protocol gaps (C2–C4)

These matter more than endpoint presence — a wrong 200 is a failed endpoint.

The full code-traced audit (6 areas, **59 findings: 10 Blocker / 31 Major / 18 Minor**) is in
[`docs/compliance/behavior-audit.md`](docs/compliance/behavior-audit.md). The blockers:

| Area | Finding | Location | Severity |
|------|---------|----------|----------|
| Errors | No router fallback → unknown path returns empty-body 404, wrong method empty-body 405. Complement `unknown_endpoints_test.go` requires `{"errcode":"M_UNRECOGNIZED"}` for both | `api/src/router.rs` | **Blocker** (cheap fix) |
| Federation | Locally created events are **never signed** and lack `prev_events`, `depth`, `origin`, `hashes` → real servers reject every outbound PDU. `Pdu::sign()` has **no call sites** outside tests | `handlers/events.rs:187`, `rooms.rs:147`, `directory.rs:232`, `profile.rs:490`, `knock.rs:127` | **Blocker** |
| Auth rules | **Kick/ban/unban have no power-level check** — any joined member can ban anyone | `handlers/rooms.rs:1256-1427` | **Blocker** (security) |
| Auth rules | A user's **own `m.room.member`** event bypasses all auth checks (self-join / self-ban) | `handlers/events.rs:705-753` | **Blocker** |
| Auth rules | **No `m.room.power_levels` change rules** — a PL-50 user can rewrite PL to 100 | `handlers/events.rs:710-753` | **Blocker** |
| Auth rules | **Invite has no invite-level / target-ban check** | `handlers/rooms.rs:1093-1204` | **Blocker** |
| Auth rules | No PL check for non-state events (`events_default` unused) | `handlers/events.rs:148-233` | Major |
| Federation | Inbound PDU auth is **soft-fail**: invalid/missing signatures, unknown auth events and failed auth rules are accepted with a warning | `federation/receiver.rs:496,516,566` | **Blocker** |
| Federation | `X-Matrix` request signature failures are logged and ignored | `receiver.rs:177,214` | **Blocker** |
| Federation | **State resolution v2 is not implemented** — no power ordering, no mainline ordering, single conflicting key only | `core/matrix/state.rs:90`, `receiver.rs:606-670` | **Blocker** |
| Federation | `state`/`state_ids` return `event_id` = stream position instead of the real event ID | `federation/state.rs` | Major |
| Federation | `backfill` is a linear scan, not a DAG walk; `get_missing_events` uses a stream-range approximation | `federation/backfill.rs` | Major |
| Media | **Federation media endpoints missing** (`/_matrix/federation/v1/media/download\|thumbnail`) | `federation/router.rs` | **Blocker** |
| Media | Media is always served `inline`, including `text/html` / `image/svg+xml` (spec requires `attachment`) — stored-XSS risk | `handlers/media.rs:244-249` | Major |
| Media | `Content-Disposition` omitted when there is no filename (spec: required since v1.12) | `handlers/media.rs:238-249` | Major |
| Media | URL preview returns the **raw remote `og:image` URL** instead of caching it and serving an `mxc://` URI | `media/src/preview.rs:31-47` | Major |
| Sync | 88 sync tests at 40 % — incremental sync gaps, `limited`/`prev_batch`, filter handling, leave section, `device_lists` | `handlers/sync.rs` | Major |
| Sync | `full_state=true` is parsed and ignored | `sync.rs:266` | Major |
| Sync | Timeline `types`/`not_types` filter is never applied in incremental sync; `not_types`, `contains_url`, `senders`, `not_senders`, `rooms`, `not_rooms` unimplemented (6 of 9 filter fields) | `sync.rs:288-333` | Major |
| Sync | `limited` is forced `true` for every newly-joined room (spec: only when the filter truncated) | `sync.rs:1125` | Major |
| Sync | `unread_notifications` is always `{0,0}` — clients show 0 unread forever | `sync.rs:1012,1196` | Major |
| Sync | `device_one_time_keys_count` / `device_unused_fallback_key_types` absent from the response (the latter is spec-**Required**) | `sync.rs:128-140` | Major |
| Sync | `/sync`'s `m.push_rules` disagrees with `GET /pushrules/` (3 rules vs ~30) | `sync.rs:1683` vs `capabilities.rs:643` | Major |
| E2EE | `/keys/signatures/upload` **discards all signatures** → cross-signing is non-functional | `handlers/keys.rs:539-541` | **Blocker** |
| E2EE | `/keys/changes` `left` always empty; `fallback_keys` unsupported anywhere | `handlers/keys.rs:94-460` | Major |
| E2EE | One-time keys are claimed in `key_id DESC` order, not upload order (MSC4225) | `storage/surreal/keys.rs:226` | Major |
| E2EE | `/keys/claim` does not federate to remote users' servers | `handlers/keys.rs:391-412` | Major |
| E2EE | Key backup (`room_keys/*`) entirely stubbed and no storage table exists | `handlers/keys.rs:751-779`, `db/schema.surql` | Major |
| Push rules | `GET /pushrules/global/` missing; `.m.rule.is_user_mention` / `.m.rule.suppress_edits` absent; `.m.rule.is_room_mention` uses `event_match` so it never matches; **user rules are appended after the defaults so they never take priority** | `handlers/capabilities.rs:100,643-761,529-551` | Major |
| Discovery | `versions` lists v1.1–v1.18 (v1.19 released) and advertises no unstable features although several MSCs are implemented | `handlers/versions.rs` | Major |
| Discovery | `capabilities` advertises room v12/v13 as stable but `createRoom` rejects >v11; default is v10 (spec SHOULD be v12) | `capabilities.rs:151-175` vs `rooms.rs:235-245` | Major |
| Security | `POST/GET/DELETE /_maelstrom/admin/v1/appservice[s]` live in the **client** router with **no admin auth** | `handlers/appservice.rs:48-71` | **Blocker** (security) |
| Errors | 11 handlers use raw `axum::Json` → plain-text 400 instead of `M_BAD_JSON`; path/query extractor rejections likewise | `handlers/keys.rs:97,217,394,470,547,651,755,771` | Major |
| Third-party | `/thirdparty/protocols`, `/thirdparty/location`, `/thirdparty/user` are hardcoded empty | `handlers/appservice.rs` | Minor |
| Misc | `POST /_matrix/client/v3/refresh` always fails `M_UNKNOWN`; TURN server returns empty | `handlers/capabilities.rs` | Minor |


---

## 4. Roadmap

> The **executable** version of this roadmap — milestones M0–M7, per-task checkboxes,
> `file:line` references, acceptance gates, and a suggested parallel agent split — is
> [`docs/compliance/ROADMAP.md`](docs/compliance/ROADMAP.md). Read that one; this section is
> the summary.

### Phase A — Compliance hygiene (1–2 days, no risk) → *ROADMAP M0*

1. Router fallback: `Router::fallback` → `404 {"errcode":"M_UNRECOGNIZED","error":...}`; add
   `method_not_allowed_fallback` → `405` with the same body. (fixes C2 `TestUnknownEndpoints`)
2. Move the appservice admin routes behind `AdminUser` (or move them into `maelstrom-admin`).
3. `versions`: add `v1.19`; advertise the MSCs that are actually implemented via
   `unstable_features` (`org.matrix.msc3967` cross-signing UIA, `org.matrix.msc3706`
   partial state, `org.matrix.msc4102` threaded receipts, `org.matrix.msc4186` sliding sync,
   `org.matrix.msc3391` account-data deletion) — or drop those MSC implementations.
   `UnstableFeatures` in `handlers/versions.rs` is currently an **empty struct**; it must
   become `serde_json::Map<String, bool>` (the spec type is
   `object<string, boolean>` — `data/api/client-server/versions.yaml`).

4. `GET /_matrix/client/v3/pushrules/global/`, `GET /_matrix/client/v3/directory/list/room/{roomId}`.
5. Add the missing path aliases: `/_matrix/client/v1/rooms/{roomId}/timestamp_to_event`,
   `/_matrix/client/v1/room_summary/{roomIdOrAlias}`.
6. Generic profile endpoints `GET/PUT/DELETE /_matrix/client/v3/profile/{userId}/{keyName}`.

**Exit criteria:** `TestUnknownEndpoints` green; no unauthenticated admin route; endpoint
coverage ≥ 80 %.

Concrete sketch for A1 (axum 0.8 supports both fallbacks):

```rust
// crates/maelstrom-api/src/router.rs
use maelstrom_core::matrix::error::MatrixError;

Router::new()
    .merge(client_api)
    .fallback(|_req: axum::http::Request<axum::body::Body>| async {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "errcode": "M_UNRECOGNIZED",
            "error": "Unrecognized request"
        })))
    })
    .method_not_allowed_fallback(|_req: axum::http::Request<axum::body::Body>| async {
        (StatusCode::METHOD_NOT_ALLOWED, Json(serde_json::json!({
            "errcode": "M_UNRECOGNIZED",
            "error": "Unrecognized request"
        })))
    })
    // ... existing middleware layers
```

`MatrixError` in `crates/maelstrom-core/src/matrix/error.rs` already models
`M_UNRECOGNIZED` (error.rs:258) — reuse it instead of hand-rolled JSON if it serialises
to `{"errcode": ..., "error": ...}`.


### Phase B0 — Authorization rules (3–5 days) — *new, ROADMAP M2*

Maelstrom's **local** authorization model is largely absent — this is a security blocker,
independent of federation:

- Kick/ban/unban have **no power-level check** → any joined member can ban anyone
  (`handlers/rooms.rs:1256-1427`).
- A user's **own `m.room.member`** event bypasses all checks (self-join / self-ban)
  (`handlers/events.rs:705-753`).
- No **`m.room.power_levels` change rules** → a PL-50 user can rewrite PL to 100
  (`handlers/events.rs:710-753`).
- **Invite** has no invite-level / target-ban check (`handlers/rooms.rs:1093-1204`).
- `events_default` is never applied to non-state events (`handlers/events.rs:148-233`).

Deliverable: one shared, room-version-aware `auth_rules` module used by both the CS API
and the federation receiver.

### Phase B — Federation that actually federates (1–2 weeks) — *the real blocker* → *ROADMAP M3*

1. **Sign every locally created event** before storing: `hashes.sha256` + `signatures`,
   and set `prev_events` (from the room's current forward extremities), `depth` (max+1),
   `origin` (server_name). Wire `Pdu::sign()` into a single `create_event()` helper used by
   *all* creation paths (`events.rs`, `rooms.rs`, `directory.rs`, `profile.rs`, `knock.rs`).
2. **Enforce the authorization rules** on inbound PDUs: implement the room-version-aware
   auth rules (create / member / power_levels / join_rules / redact / third_party_invite /
   restrictions) and reject events that fail them (no more soft-fail). Reject missing
   `prev_events`/`depth`/`auth_events`.
3. **Harden `X-Matrix` request auth**: reject invalid signatures, not warn.
4. **State resolution**: implement state res v2 properly (auth events, iterative auth
   checks, mainline ordering) and use it on conflicting forks in the receiver and in
   `/state` + `/state_ids` federation responses.
5. **Federation endpoints**: `event_auth`, `user/devices`, `user/keys/claim`,
   `make_knock`/`send_knock`, `hierarchy`, `media/download`, `media/thumbnail`.
6. **Backfill/get_missing_events**: real DAG walk (BFS over `prev_events` with depth limit).

**Exit criteria:** Complement `tests/federation_*.go` green; a real Synapse/Dendrite can join
a Maelstrom room and vice-versa (manual interop test with 2 servers).

### Phase C — Stable-spec surface completion (1 week)

Fill the remaining P1/P0 CS endpoints: room context, reports (`/rooms/{id}/report`,
`/users/{id}/report`), mutual rooms, support/policy well-knowns, appservice ping,
`delete_devices`, registration-token validity, 3pid request-token endpoints, AS API paths.

### Phase D — Sync & E2EE conformance (1–2 weeks)

1. Sync: filters (all of them), `limited`/`prev_batch`, incremental `next_batch` monotonicity,
   leave/invite sections, `device_lists.changed`/`fallback_keys`, `device_one_time_keys_count`,
   `to_device` + presence EDU delivery, `unread_notifications`.
2. Keys: implement `/keys/signatures/upload` storage, `/keys/changes` `left`, key backup
   (`room_keys/version` + `room_keys/keys`) properly, device-list tracking per encrypted room.
   *Key backup needs new storage*: there is no backup table in `db/schema.surql` — the
   handlers currently answer with hard-coded `{"count":0,"etag":"0"}` /
   `{"sessions":{}}` (`handlers/keys.rs:751-780`), so the stable Complement test
   `TestE2EKeyBackupReplaceRoomKeyRules` cannot pass. Add a `key_backup` +
   `key_backup_session` table (or reuse `account_data` with a `m.key_backup` document, which
   is what MSC4287/v1.19 describes).
3. Push rules: the default set lives in `default_push_rules()`
   (`handlers/capabilities.rs:643`) and user rules are persisted as the
   `_maelstrom.push_rules` account-data blob — that part is fine; what is missing is
   `GET /_matrix/client/v3/pushrules/global/` and `GET /_matrix/client/v3/directory/list/room/{roomId}`.


### Phase E — Matrix 2.0 / optional surface (post-1.0, not required for C1–C4)

Sliding sync (MSC4186), partial state join (MSC3706), delayed events (MSC4140), OIDC
(MSC3861), account lock/suspend (MSC3939), admin API (MSC2716/4133), notifications (MSC3903),
async upload (MSC2246), policy rooms signing, `state_after` (MSC4222).

---

## 5. Verification loop

```bash
cargo fmt --all && cargo clippy --all-targets --all-features && cargo test --all
make complement                      # full suite (CS + federation + MSC)
make complement-filter FILTER=TestSync
# stable-spec only (the C2 gate):
cd ../complement && go test -json -count=1 ./tests/csapi/... | tee ../complement-results.json
```

CI gate to add: a job that runs **only** `tests/csapi/...` and fails the build if any test
fails (today CI is `continue-on-error: true`, so regressions are invisible). Add the
federation package as a second gate.

### 5.1 Stable-gate tests that cannot pass today (from the code audit)

The 107 stable test functions are listed in `docs/compliance/complement-csapi-tests.txt`.
These are expected to fail for reasons that are visible in the code, independent of the
remaining ~40 unknown failures:

| Stable test | Why it cannot pass | Fix |
|-------------|--------------------|-----|
| `TestUnknownEndpoints` | no router fallback → empty-body 404/405 instead of `M_UNRECOGNIZED` | Phase A.1 |
| `TestRoomSummary` | `GET /_matrix/client/v1/room_summary/{roomIdOrAlias}` not routed | Phase A.5 |
| `TestRoomTimestampToEvent` (root `room_timestamp_to_event_test.go`) | only the `v3` path is routed, Complement calls `v1` | Phase A.5 |
| `TestE2EKeyBackupReplaceRoomKeyRules` | key backup handlers are stubs, no storage | Phase D.2 |
| `TestKeyChangesLocal` (partly) | `/keys/changes` always returns `"left": []` (`handlers/keys.rs:460`) | Phase D.2 |
| `TestDeviceListUpdates` | device-list tracking is not per-encrypted-room | Phase D.2 |
| `TestPushSync` / `TestPushRuleRoomUpgrade` | push rules are defaults only, no `GET /pushrules/global/`; room-upgrade rule carry-over incomplete | Phase D.3 |
| `TestContent` / `TestContentCSAPIMediaV1` | `/_matrix/client/v1/media/*` and federation media endpoints only partially routed | Phase C |
| `TestMessagesOverFederation` | outbound PDUs are unsigned → remote server rejects | Phase B.1 |
| `TestMembershipOnEvents` | historical membership per event not modelled | Phase D.1 |
| `TestSyncTimelineGap`, `TestGappedSyncLeaveSection`, `TestLeaveEventVisibility` | sync gap / leave-section semantics | Phase D.1 |



---

## 6. Documentation hygiene (do this alongside Phase A)

The three status documents disagree, which makes any "compliance" claim unverifiable:

| Claim | README.md | VERIFICATION.md | PROJECT.md |
|-------|-----------|-----------------|------------|
| Complement pass rate | 343+/538 (63.8 %), dated 2026-04-10 | 356/539 (66.0 %), dated 2026-04-13 | 336/538 (62.5 %) and 322/437 (73.7 %) in two places |
| Test count | 538 | 539 | 437 / 538 |

Pick **one** source of truth (a generated `docs/compliance-status.md` from
`complement-results.json`), and have `README.md` / `PROJECT.md` / `VERIFICATION.md` link to
it instead of embedding numbers. `VERIFICATION.md` also refers to a `CLAUDE.md` that is not
in the repository — either add it or drop the reference.

---

## 7. Working artefacts (all inside the repository — no network needed)

* `vendor/matrix-spec-v1.19.tar.gz` — the v1.19 spec source (OpenAPI per endpoint);
  extract with `make compliance-vendor` → `vendor/matrix-spec-1.19/`
* `vendor/complement-main.tar.gz` — the Complement test suite (stable + MSC);
  extract with `make compliance-vendor` → `vendor/complement-main/`
* `docs/compliance/endpoints-v1.19.json` — 235 extracted endpoints (CS 166, SS 36, AS 9, ID 23, PG 1)
* `docs/compliance/endpoint-gap-v1.19.txt` — the diff report (missing / mismatched / extra)
* `docs/compliance/route-inventory.md` — route inventory with stub/partial classification
* `docs/compliance/complement-csapi-tests.txt` — the 107 stable Complement test functions
* `docs/compliance/ROADMAP.md` — the milestone roadmap for the agent executing this plan
* *(in progress)* `docs/compliance/behavior-audit.md` — behavioural conformance audit
  (sync, E2EE, state/auth rules, media, push rules, error semantics) — fold its findings
  into §3.2 when it lands


### Appendix — regenerating the gap analysis

```bash
make compliance-diff
# or manually:
python3 docs/compliance/extract_spec_endpoints.py --spec vendor/matrix-spec-1.19 \
    --out docs/compliance/endpoints-v1.19.json
python3 docs/compliance/diff_routes.py --endpoints docs/compliance/endpoints-v1.19.json
```

