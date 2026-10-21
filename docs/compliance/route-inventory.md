# Maelstrom — HTTP Route Inventory

Repo: `/root/m-lstrom`  ·  Date: 2026-10-21

All routes registered by the three crates that are mounted in `src/main.rs`:

- `maelstrom_api::router::build` (client-server API) — `src/main.rs:496`
- `maelstrom_federation::router::build` (server-server API) — `src/main.rs:428`
- `maelstrom_admin::router::build` (admin API + SSR dashboard) — `src/main.rs:471`

Status legend:
- **implemented** — handler has a real body that reads/writes storage, performs the Matrix semantics, and returns a meaningful response.
- **partial** — handler runs and returns a well-formed response, but the semantics are approximations / hardcoded values / incomplete.
- **stub** — handler returns a constant or always-fails; no real work performed.

---

## 1. Route table

### 1.1 `maelstrom-api` — Client-Server API (`crates/maelstrom-api/src/handlers/`)

| Method | Path | File | Description | Status |
|---|---|---|---|---|
| GET | /_matrix/client/versions | versions.rs:29 | List supported spec versions + unstable feature flags | partial (flags always empty) |
| GET | /.well-known/matrix/client | wellknown.rs:35 | Client discovery: `m.homeserver` base URL | implemented |
| GET | /.well-known/matrix/server | wellknown.rs:36 | Federation discovery: `m.server` = server_name:8448 | implemented |
| GET | /_health/live | health.rs:35 | Liveness probe, always `{"status":"ok"}` | implemented |
| GET | /_health/ready | health.rs:36 | Readiness probe, 200 if DB healthy else 503 | implemented |
| POST | /_matrix/client/v3/register | register.rs:70 | Register a new user (m.login.dummy flow, Argon2 password hash) | implemented |
| GET | /_matrix/client/v3/register/available | register.rs:71 | Check username availability | implemented |
| GET | /_synapse/admin/v1/register | register.rs:73 | Synapse-compat: issue registration nonce | implemented |
| POST | /_synapse/admin/v1/register | register.rs:73 | Synapse-compat: register user with nonce | implemented |
| GET | /_matrix/client/v3/login | auth.rs:58 | List supported login flows (password only) | implemented |
| POST | /_matrix/client/v3/login | auth.rs:58 | Log in (password / dummy), returns access token + device id | implemented |
| GET | /_matrix/client/r0/login | auth.rs:59 | r0 alias of GET /login | implemented |
| POST | /_matrix/client/r0/login | auth.rs:59 | r0 alias of POST /login | implemented |
| POST | /_matrix/client/v3/logout | auth.rs:60 | Log out (delete current access token / device) | implemented |
| POST | /_matrix/client/v3/logout/all | auth.rs:61 | Log out of all sessions for the user | implemented |
| GET | /_matrix/client/v3/capabilities | capabilities.rs:74 | Server capabilities + room versions | implemented |
| POST | /_matrix/client/v3/user/{userId}/filter | capabilities.rs:75 | Create a sync filter (stored as account data) | implemented |
| GET | /_matrix/client/v3/user/{userId}/filter/{filterId} | capabilities.rs:79 | Get a previously created filter | implemented |
| GET | /_matrix/client/v3/user/{userId}/account_data/{type} | capabilities.rs:83 | Read global account data | implemented |
| PUT | /_matrix/client/v3/user/{userId}/account_data/{type} | capabilities.rs:83 | Write global account data | implemented |
| DELETE | /_matrix/client/v3/user/{userId}/account_data/{type} | capabilities.rs:83 | Delete global account data | implemented |
| GET | /_matrix/client/v3/user/{userId}/rooms/{roomId}/account_data/{type} | capabilities.rs:87 | Read per-room account data | implemented |
| PUT | /_matrix/client/v3/user/{userId}/rooms/{roomId}/account_data/{type} | capabilities.rs:87 | Write per-room account data | implemented |
| DELETE | /_matrix/client/v3/user/{userId}/rooms/{roomId}/account_data/{type} | capabilities.rs:87 | Delete per-room account data | implemented |
| DELETE | /_matrix/client/unstable/org.matrix.msc3391/user/{userId}/account_data/{type} | capabilities.rs:92 | MSC3391: delete global account data via sentinel marker | implemented |
| DELETE | /_matrix/client/unstable/org.matrix.msc3391/user/{userId}/rooms/{roomId}/account_data/{type} | capabilities.rs:96 | MSC3391: delete per-room account data via sentinel marker | implemented |
| GET | /_matrix/client/v3/pushrules/ | capabilities.rs:100 | Get push rules (server defaults + user overrides) | implemented |
| GET | /_matrix/client/v3/pushers | capabilities.rs:101 | List the user's pushers | implemented |
| POST | /_matrix/client/v3/pushers/set | capabilities.rs:102 | Create/delete a pusher | implemented |
| PUT | /_matrix/client/v3/pushrules/global/{kind}/{ruleId} | capabilities.rs:103 | Set a push rule | implemented |
| GET | /_matrix/client/v3/pushrules/global/{kind}/{ruleId} | capabilities.rs:103 | Get a push rule | implemented |
| DELETE | /_matrix/client/v3/pushrules/global/{kind}/{ruleId} | capabilities.rs:103 | Delete a push rule | implemented |
| PUT | /_matrix/client/v3/pushrules/global/{kind}/{ruleId}/enabled | capabilities.rs:109 | Enable/disable a push rule | implemented |
| GET | /_matrix/client/v3/pushrules/global/{kind}/{ruleId}/enabled | capabilities.rs:109 | Get push rule enabled flag | implemented |
| PUT | /_matrix/client/v3/pushrules/global/{kind}/{ruleId}/actions | capabilities.rs:113 | Set push rule actions | implemented |
| GET | /_matrix/client/v3/pushrules/global/{kind}/{ruleId}/actions | capabilities.rs:113 | Get push rule actions | implemented |
| GET | /_matrix/client/v3/voip/turnServer | capabilities.rs:117 | Return TURN server credentials | stub (hardcoded empty URIs) |
| GET | /_matrix/client/v3/devices | capabilities.rs:118 | List the user's devices | implemented |
| GET | /_matrix/client/v3/devices/{deviceId} | capabilities.rs:119 | Get one device | implemented |
| PUT | /_matrix/client/v3/devices/{deviceId} | capabilities.rs:119 | Rename/update a device | implemented |
| DELETE | /_matrix/client/v3/devices/{deviceId} | capabilities.rs:119 | Delete a device | implemented |
| GET | /_matrix/client/v3/rooms/{roomId}/members | capabilities.rs:123 | List room members with memberships | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/read_markers | capabilities.rs:127 | Set fully_read / read receipt markers | implemented |
| GET | /_matrix/client/v3/user/{userId}/rooms/{roomId}/tags | capabilities.rs:132 | Get room tags (`m.tag`) | implemented |
| PUT | /_matrix/client/v3/user/{userId}/rooms/{roomId}/tags/{tag} | capabilities.rs:136 | Add/update a room tag | implemented |
| DELETE | /_matrix/client/v3/user/{userId}/rooms/{roomId}/tags/{tag} | capabilities.rs:136 | Remove a room tag | implemented |
| POST | /_matrix/client/v3/user/{userId}/openid/request_token | capabilities.rs:141 | Mint an OpenID token for the user | implemented |
| POST | /_matrix/client/v3/refresh | capabilities.rs:146 | Refresh access token | stub (always returns M_UNKNOWN error) |
| GET | /_matrix/client/v3/account/whoami | account.rs:52 | Return the authenticated user id / device id / guest flag | implemented |
| POST | /_matrix/client/v3/account/deactivate | account.rs:53 | Deactivate the account (supports UIA) | implemented |
| POST | /_matrix/client/v3/account/password | account.rs:54 | Change password (supports UIA) | implemented |
| GET | /_matrix/client/v3/profile/{userId}/displayname | profile.rs:60 | Get displayname | implemented |
| PUT | /_matrix/client/v3/profile/{userId}/displayname | profile.rs:60 | Set displayname + re-emit member events | implemented |
| GET | /_matrix/client/v3/profile/{userId}/avatar_url | profile.rs:64 | Get avatar URL | implemented |
| PUT | /_matrix/client/v3/profile/{userId}/avatar_url | profile.rs:64 | Set avatar URL + re-emit member events | implemented |
| GET | /_matrix/client/v3/profile/{userId} | profile.rs:68 | Get full profile | implemented |
| POST | /_matrix/client/v3/user_directory/search | profile.rs:69 | Search the user directory (room/public-room visibility rules) | implemented |
| POST | /_matrix/client/v3/createRoom | rooms.rs:97 | Create a room (creator, name, topic, invite, power levels) | implemented |
| POST | /_matrix/client/v3/join/{roomIdOrAlias} | rooms.rs:98 | Join a room by id or alias (local or federated) | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/join | rooms.rs:102 | Join a room by id | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/leave | rooms.rs:103 | Leave a room | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/invite | rooms.rs:104 | Invite a user to a room | implemented |
| GET | /_matrix/client/v3/joined_rooms | rooms.rs:108 | List rooms the user has joined | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/kick | rooms.rs:109 | Kick a user | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/ban | rooms.rs:110 | Ban a user | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/unban | rooms.rs:111 | Unban a user | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/forget | rooms.rs:112 | Forget a room (leave + hide) | implemented |
| GET | /_matrix/client/v3/rooms/{roomId}/joined_members | rooms.rs:116 | List joined members | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/upgrade | rooms.rs:120 | Upgrade a room to a new version | implemented |
| GET | /_matrix/client/r0/rooms/{roomId}/joined_members | rooms.rs:125 | r0 alias of joined_members | implemented |
| PUT | /_matrix/client/{v3\|r0}/rooms/{roomId}/send/{eventType}/{txnId} | events.rs:96 | Send a message/state event (txn-id idempotency) | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/event/{eventId} | events.rs:100 | Get a single event | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/messages | events.rs:104 | Paginated room timeline (`/messages`) | implemented |
| PUT | /_matrix/client/{v3\|r0}/rooms/{roomId}/state/{eventType}/{stateKey} | events.rs:108 | Send a state event | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/state/{eventType}/{stateKey} | events.rs:108 | Get a state event | implemented |
| PUT | /_matrix/client/{v3\|r0}/rooms/{roomId}/state/{eventType} | events.rs:112 | Send state event with empty state key | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/state/{eventType} | events.rs:112 | Get state event with empty state key | implemented |
| PUT | /_matrix/client/{v3\|r0}/rooms/{roomId}/state/{eventType}/ | events.rs:117 | Trailing-slash variant of state PUT | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/state/{eventType}/ | events.rs:117 | Trailing-slash variant of state GET | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/state | events.rs:121 | Get full current room state | implemented |
| PUT | /_matrix/client/{v3\|r0}/rooms/{roomId}/redact/{eventId}/{txnId} | events.rs:125 | Redact an event | implemented |
| GET | /_matrix/client/{v3\|r0}/rooms/{roomId}/timestamp_to_event | events.rs:129 | Find closest event to a timestamp | implemented |
| PUT | /_matrix/client/v3/directory/room/{roomAlias} | directory.rs:48 | Create a room alias | implemented |
| GET | /_matrix/client/v3/directory/room/{roomAlias} | directory.rs:48 | Resolve a room alias | implemented |
| DELETE | /_matrix/client/v3/directory/room/{roomAlias} | directory.rs:48 | Delete a room alias | implemented |
| GET | /_matrix/client/v3/rooms/{roomId}/aliases | directory.rs:54 | List aliases for a room | implemented |
| PUT | /_matrix/client/v3/directory/list/room/{roomId} | directory.rs:58 | Set room directory visibility (public/private) | implemented |
| GET | /_matrix/client/v3/publicRooms | directory.rs:62 | List public rooms | implemented |
| POST | /_matrix/client/v3/publicRooms | directory.rs:62 | Search/filter public rooms | implemented |
| POST | /_matrix/client/v3/keys/upload | keys.rs:57 | Upload device keys / one-time keys | implemented |
| POST | /_matrix/client/v3/keys/query | keys.rs:58 | Query device + cross-signing keys | implemented |
| POST | /_matrix/client/v3/keys/claim | keys.rs:59 | Claim one-time keys | implemented |
| GET | /_matrix/client/v3/keys/changes | keys.rs:60 | List users whose device lists changed | partial (heuristic; `left` always empty) |
| POST | /_matrix/client/v3/keys/device_signing/upload | keys.rs:61 | Upload cross-signing keys (UIA / MSC3967) | implemented |
| POST | /_matrix/client/v3/keys/signatures/upload | keys.rs:65 | Upload key signatures | stub (accepts and returns `{"failures":{}}`, stores nothing) |
| POST | /_matrix/client/v3/room_keys/version | keys.rs:69 | Create a key backup version | implemented |
| GET | /_matrix/client/v3/room_keys/version | keys.rs:69 | Get current key backup version | implemented |
| GET | /_matrix/client/v3/room_keys/version/{version} | keys.rs:73 | Get a specific key backup version | partial (`count`/`etag` hardcoded to 0) |
| PUT | /_matrix/client/v3/room_keys/keys/{roomId}/{sessionId} | keys.rs:77 | Store one room key (with spec replacement rules) | implemented |
| GET | /_matrix/client/v3/room_keys/keys/{roomId}/{sessionId} | keys.rs:77 | Fetch one room key | implemented |
| PUT | /_matrix/client/v3/room_keys/keys/{roomId} | keys.rs:81 | Store all keys for a room | stub (returns `{"count":0,"etag":"0"}`, stores nothing) |
| GET | /_matrix/client/v3/room_keys/keys/{roomId} | keys.rs:81 | Fetch all keys for a room | stub (returns empty `sessions`) |
| PUT | /_matrix/client/v3/room_keys/keys | keys.rs:85 | Store all room keys | stub (returns `{"count":0,"etag":"0"}`, stores nothing) |
| GET | /_matrix/client/v3/room_keys/keys | keys.rs:85 | Fetch all room keys | stub (returns empty `rooms`) |
| POST | /_matrix/client/v3/knock/{roomIdOrAlias} | knock.rs:41 | Knock on a room (knock / knock_restricted only) | implemented |
| POST | /_matrix/client/v1/media/upload | media.rs:61 | Upload media (S3-backed) | implemented |
| GET | /_matrix/client/v1/media/download/{serverName}/{mediaId} | media.rs:62 | Download media | implemented |
| GET | /_matrix/client/v1/media/download/{serverName}/{mediaId}/{fileName} | media.rs:66 | Download media with filename | implemented |
| GET | /_matrix/client/v1/media/thumbnail/{serverName}/{mediaId} | media.rs:70 | Generate/scale a thumbnail (falls back to original) | implemented |
| GET | /_matrix/client/v1/media/config | media.rs:74 | Media config (`m.upload.size`) | implemented |
| GET | /_matrix/client/v1/media/preview_url | media.rs:75 | Fetch OG preview metadata for a URL | implemented |
| POST | /_matrix/media/v3/upload | media.rs:77 | v3 alias of media upload | implemented |
| GET | /_matrix/media/v3/download/{serverName}/{mediaId} | media.rs:78 | v3 alias of media download | implemented |
| GET | /_matrix/media/v3/download/{serverName}/{mediaId}/{fileName} | media.rs:82 | v3 alias of media download w/ filename | implemented |
| GET | /_matrix/media/v3/thumbnail/{serverName}/{mediaId} | media.rs:86 | v3 alias of thumbnail | implemented |
| GET | /_matrix/media/v3/config | media.rs:90 | v3 alias of media config | implemented |
| GET | /_matrix/media/v3/preview_url | media.rs:91 | v3 alias of preview_url | implemented |
| GET | /_matrix/client/v3/presence/{userId}/status | presence.rs:40 | Get presence state (defaults to offline) | implemented |
| PUT | /_matrix/client/v3/presence/{userId}/status | presence.rs:40 | Set presence state + federate `m.presence` EDU | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/receipt/{receiptType}/{eventId} | receipts.rs:41 | Send a read receipt (supports MSC4102 `thread_id`) | implemented |
| GET | /_matrix/client/v1/rooms/{roomId}/relations/{eventId} | relations.rs:45 | List all relations of an event | implemented |
| GET | /_matrix/client/v1/rooms/{roomId}/relations/{eventId}/{relType} | relations.rs:49 | List relations by rel type | implemented |
| GET | /_matrix/client/v1/rooms/{roomId}/relations/{eventId}/{relType}/{eventType} | relations.rs:53 | List relations by rel type + event type | implemented |
| POST | /_matrix/client/v3/rooms/{roomId}/report/{eventId} | reporting.rs:30 | Report an event for abuse (stored in `event_report`) | implemented |
| POST | /_matrix/client/v3/search | search.rs:36 | Search room events (filters, pagination, event context) | implemented |
| GET | /_matrix/client/v1/rooms/{roomId}/hierarchy | spaces.rs:40 | Traverse the space hierarchy (`m.space.child`) | implemented |
| GET | /_matrix/client/v3/sync | sync.rs:98 | Long-poll incremental/initial sync | implemented |
| POST | /_matrix/client/v3/sync | sync.rs:98 | Sliding sync (MSC4186-style) | implemented |
| GET | /_matrix/client/v1/rooms/{roomId}/threads | threads.rs:32 | List thread roots in a room | implemented |
| PUT | /_matrix/client/v3/sendToDevice/{eventType}/{txnId} | to_device.rs:39 | Send to-device messages (local + federated EDU) | implemented |
| PUT | /_matrix/client/v3/rooms/{roomId}/typing/{userId} | typing.rs:34 | Set typing notification | implemented |
| POST | /_maelstrom/admin/v1/appservice | appservice.rs:48 | Register an application service | implemented (⚠ no admin auth — see §2) |
| GET | /_maelstrom/admin/v1/appservices | appservice.rs:49 | List registered application services | implemented (⚠ no admin auth) |
| DELETE | /_maelstrom/admin/v1/appservice/{asId} | appservice.rs:50 | Unregister an application service | implemented (⚠ no admin auth) |
| GET | /_matrix/client/v3/thirdparty/protocols | appservice.rs:55 | List third-party protocols | partial (static empty metadata) |
| GET | /_matrix/client/v3/thirdparty/protocol/{protocol} | appservice.rs:59 | Get one third-party protocol | partial (static empty metadata) |
| GET | /_matrix/client/v3/thirdparty/location/{protocol} | appservice.rs:63 | Search third-party locations by protocol | stub (always `[]`) |
| GET | /_matrix/client/v3/thirdparty/user/{protocol} | appservice.rs:67 | Search third-party users by protocol | stub (always `[]`) |
| GET | /_matrix/client/v3/thirdparty/location | appservice.rs:71 | Search all third-party locations | stub (always `[]`) |
| GET | /_matrix/client/v3/thirdparty/user | appservice.rs:72 | Search all third-party users | stub (always `[]`) |

> The `{v3\|r0}` rows expand to **two** concrete paths each (`/_matrix/client/v3/...` and `/_matrix/client/r0/...`) — `events.rs` builds its router in a loop over both prefixes (events.rs:94).

### 1.2 `maelstrom-federation` — Server-Server API (`crates/maelstrom-federation/src/`)

| Method | Path | File | Description | Status |
|---|---|---|---|---|
| GET | /_matrix/federation/v1/version | key_server.rs:47 | Server name + version | implemented |
| GET | /_matrix/key/v2/server | key_server.rs:43 | This server's self-signed signing keys | implemented |
| GET | /_matrix/key/v2/server/{keyId} | key_server.rs:44 | This server's signing keys (same handler) | implemented |
| GET | /_matrix/key/v2/query/{serverName} | key_server.rs:45 | Notary query for one remote server (fetch + cache) | implemented |
| POST | /_matrix/key/v2/query | key_server.rs:46 | Batch notary key query | implemented |
| PUT | /_matrix/federation/v1/send/{txnId} | receiver.rs:94 | Receive an inbound transaction (PDUs + EDUs) | implemented (auth is a soft check) |
| GET | /_matrix/federation/v1/openid/userinfo | receiver.rs:98 | Resolve an OpenID token to a Matrix user id | implemented |
| GET | /_matrix/federation/v1/make_join/{roomId}/{userId} | joins.rs:64 | Produce a join event template for the remote server to sign | implemented |
| PUT | /_matrix/federation/v1/send_join/{roomId}/{eventId} | joins.rs:68 | Accept a signed join event (v1 response shape) | implemented |
| PUT | /_matrix/federation/v2/send_join/{roomId}/{eventId} | joins.rs:72 | Accept a signed join event (v2 response shape) | implemented |
| GET | /_matrix/federation/v1/make_leave/{roomId}/{userId} | joins.rs:76 | Produce a leave event template | implemented |
| PUT | /_matrix/federation/v1/send_leave/{roomId}/{eventId} | joins.rs:80 | Accept a signed leave event (v1 shape) | implemented |
| PUT | /_matrix/federation/v2/send_leave/{roomId}/{eventId} | joins.rs:84 | Accept a signed leave event (v2 shape) | implemented |
| GET | /_matrix/federation/v1/state/{roomId} | state.rs:42 | Room state + auth chain | partial (`event_id` filter is a stream-position approximation) |
| GET | /_matrix/federation/v1/state_ids/{roomId} | state.rs:43 | Room state event ids + auth chain | partial (same `event_id` approximation) |
| GET | /_matrix/federation/v1/event/{eventId} | state.rs:47 | Fetch a single event by id | implemented |
| GET | /_matrix/federation/v1/timestamp_to_event/{roomId} | state.rs:48 | Find event closest to a timestamp | implemented |
| GET | /_matrix/federation/v1/backfill/{roomId} | backfill.rs:43 | Backfill historical events | partial (linear stream scan, not a DAG walk) |
| POST | /_matrix/federation/v1/get_missing_events/{roomId} | backfill.rs:44 | Events between earliest/latest frontier | partial (stream-position range approximation) |
| POST | /_matrix/federation/v1/user/keys/query | user_keys.rs:52 | Serve device + cross-signing keys for local users | implemented |
| GET | /_matrix/federation/v1/user/devices/{userId} | user_keys.rs:56 | List all devices of a local user | implemented |
| GET | /_matrix/federation/v1/query/profile | queries.rs:40 | Serve a local user's profile to a remote server | implemented |
| GET | /_matrix/federation/v1/query/directory | queries.rs:41 | Resolve a room alias to a room id for a remote server | implemented |
| GET | /_matrix/federation/v1/publicRooms | queries.rs:45 | List public rooms for remote servers | implemented |
| POST | /_matrix/federation/v1/publicRooms | queries.rs:45 | Search public rooms for remote servers | implemented |
| PUT | /_matrix/federation/v1/invite/{roomId}/{eventId} | invite.rs:54 | Receive an invite (v1 body shape) | implemented |
| PUT | /_matrix/federation/v2/invite/{roomId}/{eventId} | invite.rs:50 | Receive an invite (v2 `{"event":...}` shape) | implemented |
| POST | /_matrix/federation/v2/exchange_third_party_invite/{roomId} | invite.rs:58 | Exchange a 3PI invite for a real member event | implemented |

### 1.3 `maelstrom-admin` — Admin API + SSR dashboard (`crates/maelstrom-admin/src/`)

All admin handlers require the `AdminUser` extractor (Bearer token → device → `is_admin` check, `crates/maelstrom-admin/src/auth.rs`).

| Method | Path | File | Description | Status |
|---|---|---|---|---|
| GET | /_maelstrom/admin/ | dashboard.rs:33 | SSR dashboard (uptime, memory, DB status) | implemented |
| GET | /_maelstrom/admin/users | dashboard.rs:34 | SSR user-management page | partial (static page; says "use the API endpoint") |
| GET | /_maelstrom/admin/rooms | dashboard.rs:35 | SSR room-management page | partial (static page; says "use the API endpoint") |
| GET | /_maelstrom/admin/federation | dashboard.rs:36 | SSR federation status page | implemented |
| GET | /_maelstrom/admin/v1/users | users.rs:32 | List users | stub (always `{"users":[],"total":0}`) |
| GET | /_maelstrom/admin/v1/users/{userId} | users.rs:33 | Get one user (profile, device/room counts) | implemented |
| POST | /_maelstrom/admin/v1/users/{userId}/deactivate | users.rs:34 | Deactivate a user | implemented |
| POST | /_maelstrom/admin/v1/users/{userId}/reactivate | users.rs:38 | Reactivate a user | implemented |
| PUT | /_maelstrom/admin/v1/users/{userId}/admin | users.rs:42 | Grant admin | implemented |
| DELETE | /_maelstrom/admin/v1/users/{userId}/admin | users.rs:43 | Revoke admin | implemented |
| POST | /_maelstrom/admin/v1/users/{userId}/reset-password | users.rs:47 | Set a new password hash (Argon2) | implemented |
| GET | /_maelstrom/admin/v1/users/{userId}/devices | users.rs:51 | List a user's devices | implemented |
| GET | /_maelstrom/admin/v1/rooms | rooms.rs:29 | List public rooms | implemented |
| GET | /_maelstrom/admin/v1/rooms/{roomId} | rooms.rs:30 | Room detail (name, topic, members, state count) | implemented |
| POST | /_maelstrom/admin/v1/rooms/{roomId}/shutdown | rooms.rs:31 | Force all joined members to leave | implemented |
| GET | /_maelstrom/admin/v1/media/user/{userId} | media.rs:33 | List media uploaded by a user | implemented |
| POST | /_maelstrom/admin/v1/media/{serverName}/{mediaId}/quarantine | media.rs:34 | Quarantine a media item | implemented |
| POST | /_maelstrom/admin/v1/media/{serverName}/{mediaId}/unquarantine | media.rs:38 | Un-quarantine a media item | implemented |
| GET | /_maelstrom/admin/v1/media/retention | media.rs:42 | Get media retention policy | implemented |
| PUT | /_maelstrom/admin/v1/media/retention | media.rs:42 | Set media retention policy | implemented |
| POST | /_maelstrom/admin/v1/media/retention/sweep | media.rs:46 | Run a retention sweep now | partial (deletes DB rows only; S3 objects are not deleted — see media.rs:192) |
| GET | /_maelstrom/admin/v1/federation/stats | federation.rs:25 | Signing-key count + server status | implemented |
| GET | /_maelstrom/admin/v1/server/info | server.rs:29 | Server name, version, uptime, memory, CPU | implemented |
| GET | /_maelstrom/admin/v1/server/health | server.rs:30 | Detailed health (DB up/down) | implemented |
| GET | /_maelstrom/admin/v1/metrics | server.rs:31 | Prometheus text metrics (uptime, memory, DB up) | implemented (minimal metric set) |
| GET | /_maelstrom/admin/v1/reports | reports.rs:27 | List content reports | stub (always `{"reports":[],"total":0}`) |

Non-route services: `GET /_maelstrom/admin/static/*` is served by `tower_http::services::ServeDir` (`crates/maelstrom-admin/src/router.rs:44`), not a handler.

---

## 2. Known TODOs / stubs

### 2.1 Repo-wide grep for `todo!` / `unimplemented!` / `TODO` / `FIXME`

There are **no** `todo!()` or `unimplemented!()` macros anywhere in the workspace (`crates/`, `src/`, `tests/`). The complete list of `TODO`/`FIXME` hits is:

| file:line | Description |
|---|---|
| `crates/maelstrom-api/src/handlers/rooms.rs:717` | `// TODO: Verify signatures on all state events and auth_chain events from their origin servers.` — inbound federation state events are trusted, not signature-verified. |
| `crates/maelstrom-api/src/handlers/sync.rs:922` | `// TODO(M9): Use futures::future::join_all to fetch room data in parallel` — sync fetches room data sequentially (perf, not correctness). |
| `VERIFICATION.md:169` | Documentation note comparing against tuwunel (not code). |

### 2.2 Handler-level "not implemented" / stub markers

| file:line | Description | Affected route |
|---|---|---|
| `crates/maelstrom-api/src/handlers/capabilities.rs:1556-1563` | `refresh_token` — "Refresh an access token. Not supported -- returns M_UNKNOWN." Always returns HTTP 400 `M_UNKNOWN`. | `POST /_matrix/client/v3/refresh` |
| `crates/maelstrom-api/src/handlers/capabilities.rs:959-966` | `get_turn_server` — returns a hardcoded empty TURN config (`uris: []`, empty username/password). | `GET /_matrix/client/v3/voip/turnServer` |
| `crates/maelstrom-api/src/handlers/keys.rs:416` | `keys_changes` — "Get users whose device lists have changed (stub for now)". Uses an account-data heuristic (`_maelstrom.device_change_pos`); `left` is always `[]`. | `GET /_matrix/client/v3/keys/changes` |
| `crates/maelstrom-api/src/handlers/keys.rs:538-541` | `keys_signatures_upload` — "Upload key signatures (stub -- accepts and returns success)". Body is ignored, nothing persisted. | `POST /_matrix/client/v3/keys/signatures/upload` |
| `crates/maelstrom-api/src/handlers/keys.rs:751-759` | `put_room_keys` — ignores body, returns `{"count":0,"etag":"0"}`. | `PUT /_matrix/client/v3/room_keys/keys/{roomId}` |
| `crates/maelstrom-api/src/handlers/keys.rs:761-767` | `get_room_keys` — returns `{"sessions":{}}`. | `GET /_matrix/client/v3/room_keys/keys/{roomId}` |
| `crates/maelstrom-api/src/handlers/keys.rs:769-775` | `put_all_room_keys` — ignores body, returns `{"count":0,"etag":"0"}`. | `PUT /_matrix/client/v3/room_keys/keys` |
| `crates/maelstrom-api/src/handlers/keys.rs:777-779` | `get_all_room_keys` — returns `{"rooms":{}}`. | `GET /_matrix/client/v3/room_keys/keys` |
| `crates/maelstrom-api/src/handlers/keys.rs:641-644` | `get_key_backup_version` — `count` and `etag` are hardcoded to `0`. | `GET /_matrix/client/v3/room_keys/version/{version}` |
| `crates/maelstrom-api/src/handlers/versions.rs:40` | "Placeholder for MSC flags — populated as features are implemented"; `UnstableFeatures {}` is always empty. | `GET /_matrix/client/versions` |
| `crates/maelstrom-api/src/handlers/appservice.rs:179` | "Full implementation would query the AS via HTTP." | `GET /_matrix/client/v3/thirdparty/location/{protocol}` |
| `crates/maelstrom-api/src/handlers/appservice.rs:184-190` | `get_user_by_protocol` — returns `[]`. | `GET /_matrix/client/v3/thirdparty/user/{protocol}` |
| `crates/maelstrom-api/src/handlers/appservice.rs:192-199` | `get_locations` / `get_users` — both return `[]`. | `GET /_matrix/client/v3/thirdparty/location`, `GET /_matrix/client/v3/thirdparty/user` |
| `crates/maelstrom-api/src/handlers/appservice.rs:133-152` | `get_protocols` / `get_protocol` — "metadata is minimal (empty fields/instances); full delegation to the AS would require an HTTP call to the AS's URL." | `GET /_matrix/client/v3/thirdparty/protocols`, `GET /_matrix/client/v3/thirdparty/protocol/{protocol}` |
| `crates/maelstrom-api/src/handlers/appservice.rs:210-215` | `notify_appservices` — "Called synchronously after storing events. A production implementation would use a background queue." (not a route) | — |
| `crates/maelstrom-admin/src/handlers/reports.rs:13-14, 35` | "This is currently a placeholder that returns an empty list. The full implementation will query the `event_report` table with pagination and filtering support." | `GET /_maelstrom/admin/v1/reports` |
| `crates/maelstrom-admin/src/handlers/users.rs:57-65` | `list_users` — returns `{"users":[],"total":0}` unconditionally. | `GET /_maelstrom/admin/v1/users` |
| `crates/maelstrom-admin/src/handlers/mod.rs:12` | Doc says "Prometheus metrics stub" (the handler does emit 4 real gauges — treat as implemented but minimal). | `GET /_maelstrom/admin/v1/metrics` |
| `crates/maelstrom-admin/src/handlers/media.rs:192` | "Delete metadata (S3 deletion would need the MediaClient, which is in the API crate)" — orphaned S3 objects after a sweep. | `POST /_maelstrom/admin/v1/media/retention/sweep` |
| `crates/maelstrom-admin/src/handlers/dashboard.rs:67-79` + `templates/pages/users.html`, `templates/pages/rooms.html` | SSR pages render static text pointing the user at the JSON API — no data binding. | `GET /_maelstrom/admin/users`, `GET /_maelstrom/admin/rooms` |
| `crates/maelstrom-federation/src/backfill.rs:26-27` | "The current implementation is simplified -- it returns recent events up to the limit rather than performing a full DAG walk." | `GET /_matrix/federation/v1/backfill/{roomId}` |
| `crates/maelstrom-federation/src/backfill.rs:123-127` | "The implementation approximates DAG walking by using stream_position ranges." | `POST /_matrix/federation/v1/get_missing_events/{roomId}` |
| `crates/maelstrom-federation/src/state.rs:56-57` | `event_id` query param "not yet implemented -- currently returns current state" (the handler does a stream_position filter, which is an approximation of historical state). | `GET /_matrix/federation/v1/state/{roomId}`, `GET /_matrix/federation/v1/state_ids/{roomId}` |
| `crates/maelstrom-federation/src/joins.rs:537-539` | "In a full implementation, these would be the true DAG leaf nodes. Currently simplified to return the last 2 events by stream position." (affects `prev_events` in make_join/make_leave templates) | `GET /_matrix/federation/v1/make_join/...`, `GET /_matrix/federation/v1/make_leave/...` |
| `crates/maelstrom-federation/src/joins.rs:137` | `"depth": 100, // Simplified; real depth would be max(prev_events depths) + 1` | make_join / make_leave templates |
| `crates/maelstrom-federation/src/receiver.rs:176`, `:500`, `:1026` | "Soft check — warn but allow for now": X-Matrix signature verification is soft-failed when the origin key cannot be fetched, and sender-signature checks warn instead of rejecting. | `PUT /_matrix/federation/v1/send/{txnId}` and all federation inbound routes |
| `crates/maelstrom-api/src/handlers/rooms.rs:673` | "For now, generate event_id from content" (event id derivation is content-hash based, not the spec's reference hash) | send/state/redact event routes |
| `crates/maelstrom-api/src/handlers/rooms.rs:717-719` | "TODO: Verify signatures on all state events and auth_chain events from their origin servers. For now we trust the..." | federation join/state handling |
| `crates/maelstrom-api/src/handlers/capabilities.rs:682-681` | `msc3381` / `msc3930` feature-flag constants exist but are not advertised via `/versions`. | `GET /_matrix/client/versions` |

### 2.3 Auth gap worth flagging (not a stub, but a known hole)

`crates/maelstrom-api/src/handlers/appservice.rs:48-52` registers three endpoints under the `/_maelstrom/admin/v1/` namespace (`POST /appservice`, `GET /appservices`, `DELETE /appservice/{asId}`) inside the **client-API** router, and their handlers take only `State`/`Json` — **no `AdminUser` extractor**. They are therefore reachable without admin credentials, unlike every other `/_maelstrom/admin/v1/*` route (which lives in the `maelstrom-admin` crate and uses `AdminUser`).

---

## 3. Unstable-prefixed routes

### 3.1 `/_matrix/client/unstable/` (MSC unstable)

| Method | Path | File | Description |
|---|---|---|---|
| DELETE | /_matrix/client/unstable/org.matrix.msc3391/user/{userId}/account_data/{type} | capabilities.rs:92 | MSC3391 — delete global account data (implemented via a `_msc3391_deleted` sentinel; handler `delete_account_data`) |
| DELETE | /_matrix/client/unstable/org.matrix.msc3391/user/{userId}/rooms/{roomId}/account_data/{type} | capabilities.rs:96 | MSC3391 — delete per-room account data (sentinel marker; handler `delete_room_account_data`) |

These are the **only** two `/_matrix/client/unstable/...` routes in the codebase. There are **no** `/_matrix/federation/unstable/...` routes.

MSC behaviour implemented behind *stable* paths (worth knowing, since the flags are not advertised):
- **MSC3391** — account-data deletion (also exposed on the stable v3 paths).
- **MSC3967** — `POST /_matrix/client/v3/keys/device_signing/upload` requires UIA (`keys.rs:467`).
- **MSC3706** — partial-state joins; background resync task `resync_room_state` (`rooms.rs:895`), `servers_in_room` in the federation crate (`joins.rs:446`).
- **MSC4102** — `thread_id` in receipts (`receipts.rs:41`).
- **MSC4186** — sliding sync on `POST /_matrix/client/v3/sync` (`sync.rs:1806`).
- **MSC3874 / MSC3381 / MSC3930** — referenced in code but not advertised via `/versions` (`versions.rs:40`).

### 3.2 Custom `/_maelstrom/` prefix

**Admin JSON API** (`maelstrom-admin` crate — all require `AdminUser`):

| Method | Path | File |
|---|---|---|
| GET | /_maelstrom/admin/ | dashboard.rs:33 |
| GET | /_maelstrom/admin/users | dashboard.rs:34 |
| GET | /_maelstrom/admin/rooms | dashboard.rs:35 |
| GET | /_maelstrom/admin/federation | dashboard.rs:36 |
| GET | /_maelstrom/admin/v1/users | users.rs:32 |
| GET | /_maelstrom/admin/v1/users/{userId} | users.rs:33 |
| POST | /_maelstrom/admin/v1/users/{userId}/deactivate | users.rs:34 |
| POST | /_maelstrom/admin/v1/users/{userId}/reactivate | users.rs:38 |
| PUT | /_maelstrom/admin/v1/users/{userId}/admin | users.rs:42 |
| DELETE | /_maelstrom/admin/v1/users/{userId}/admin | users.rs:43 |
| POST | /_maelstrom/admin/v1/users/{userId}/reset-password | users.rs:47 |
| GET | /_maelstrom/admin/v1/users/{userId}/devices | users.rs:51 |
| GET | /_maelstrom/admin/v1/rooms | rooms.rs:29 |
| GET | /_maelstrom/admin/v1/rooms/{roomId} | rooms.rs:30 |
| POST | /_maelstrom/admin/v1/rooms/{roomId}/shutdown | rooms.rs:31 |
| GET | /_maelstrom/admin/v1/media/user/{userId} | media.rs:33 |
| POST | /_maelstrom/admin/v1/media/{serverName}/{mediaId}/quarantine | media.rs:34 |
| POST | /_maelstrom/admin/v1/media/{serverName}/{mediaId}/unquarantine | media.rs:38 |
| GET | /_maelstrom/admin/v1/media/retention | media.rs:42 |
| PUT | /_maelstrom/admin/v1/media/retention | media.rs:42 |
| POST | /_maelstrom/admin/v1/media/retention/sweep | media.rs:46 |
| GET | /_maelstrom/admin/v1/federation/stats | federation.rs:25 |
| GET | /_maelstrom/admin/v1/server/info | server.rs:29 |
| GET | /_maelstrom/admin/v1/server/health | server.rs:30 |
| GET | /_maelstrom/admin/v1/metrics | server.rs:31 |
| GET | /_maelstrom/admin/v1/reports | reports.rs:27 |

**Application-service admin** (registered in the **client-API** crate — ⚠ no admin auth, see §2.3):

| Method | Path | File |
|---|---|---|
| POST | /_maelstrom/admin/v1/appservice | appservice.rs:48 |
| GET | /_maelstrom/admin/v1/appservices | appservice.rs:49 |
| DELETE | /_maelstrom/admin/v1/appservice/{asId} | appservice.rs:50 |

**Static assets:** `GET /_maelstrom/admin/static/*` — `ServeDir`, `crates/maelstrom-admin/src/router.rs:44`.

### 3.3 Other non-standard prefixes

| Method | Path | File | Note |
|---|---|---|---|
| GET | /_synapse/admin/v1/register | register.rs:73 | Synapse-compat registration nonce (custom prefix) |
| POST | /_synapse/admin/v1/register | register.rs:73 | Synapse-compat registration (custom prefix) |
| GET | /_health/live | health.rs:35 | Kubernetes-style health prefix |
| GET | /_health/ready | health.rs:36 | Kubernetes-style health prefix |

---

## 4. Summary

- **Total routes (method + path pairs): 206**
  - `maelstrom-api` (client-server + AS admin): **152**
  - `maelstrom-federation` (server-server): **28**
  - `maelstrom-admin` (admin API + SSR): **26**
  - (Plus `GET /_maelstrom/admin/static/*`, served by `ServeDir`, not counted.)
- **Fully implemented: 181**
- **Stub or partial: 25**

### Stub / partial route paths (25)

Client-Server API (16):
1. `GET /_matrix/client/versions` — partial (unstable feature flags always empty)
2. `POST /_matrix/client/v3/refresh` — stub (always errors)
3. `GET /_matrix/client/v3/voip/turnServer` — stub (hardcoded empty TURN)
4. `GET /_matrix/client/v3/keys/changes` — partial (heuristic; `left` always empty)
5. `POST /_matrix/client/v3/keys/signatures/upload` — stub (stores nothing)
6. `GET /_matrix/client/v3/room_keys/version/{version}` — partial (count/etag hardcoded)
7. `PUT /_matrix/client/v3/room_keys/keys/{roomId}` — stub
8. `GET /_matrix/client/v3/room_keys/keys/{roomId}` — stub
9. `PUT /_matrix/client/v3/room_keys/keys` — stub
10. `GET /_matrix/client/v3/room_keys/keys` — stub
11. `GET /_matrix/client/v3/thirdparty/protocols` — partial (empty metadata)
12. `GET /_matrix/client/v3/thirdparty/protocol/{protocol}` — partial (empty metadata)
13. `GET /_matrix/client/v3/thirdparty/location/{protocol}` — stub
14. `GET /_matrix/client/v3/thirdparty/user/{protocol}` — stub
15. `GET /_matrix/client/v3/thirdparty/location` — stub
16. `GET /_matrix/client/v3/thirdparty/user` — stub

Federation (4):
17. `GET /_matrix/federation/v1/state/{roomId}` — partial (`event_id` approximation)
18. `GET /_matrix/federation/v1/state_ids/{roomId}` — partial (`event_id` approximation)
19. `GET /_matrix/federation/v1/backfill/{roomId}` — partial (no DAG walk)
20. `POST /_matrix/federation/v1/get_missing_events/{roomId}` — partial (stream-position approximation)

    *(Additionally, `make_join` / `make_leave` produce simplified `prev_events`/`depth` — counted as implemented, flagged in §2.2.)*

Admin (5):
21. `GET /_maelstrom/admin/v1/reports` — stub (empty list)
22. `GET /_maelstrom/admin/v1/users` — stub (empty list)
23. `GET /_maelstrom/admin/users` — partial (static SSR page)
24. `GET /_maelstrom/admin/rooms` — partial (static SSR page)
25. `POST /_maelstrom/admin/v1/media/retention/sweep` — partial (S3 objects not deleted)
