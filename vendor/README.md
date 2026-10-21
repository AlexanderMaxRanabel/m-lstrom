# Vendored upstream references

These archives make the compliance tooling work **offline** (no network, no `git clone`).

| Archive | Source | Version | Extracted dir | License |
|---------|--------|---------|---------------|---------|
| `matrix-spec-v1.19.tar.gz` | https://github.com/matrix-org/matrix-spec (tag `v1.19`) | v1.19 — latest stable Matrix spec (2026-07-08) | `vendor/matrix-spec-1.19/` | Apache-2.0 |
| `complement-main.tar.gz` | https://github.com/matrix-org/complement (branch `main`) | snapshot taken 2026-10-21 | `vendor/complement-main/` | Apache-2.0 |

Extract them with:

```bash
make compliance-vendor      # -> vendor/matrix-spec-1.19/ and vendor/complement-main/
```

The extracted directories are **not** committed (see `.gitignore`); only the archives are.
Re-extract whenever you need to re-read the spec or run the test suite.

## What is used from each

**matrix-spec v1.19**

* `data/api/client-server/*.yaml` — OpenAPI definition of every stable CS API endpoint
  (request/response schemas, error codes, `x-addedInMatrixVersion`).
* `data/api/server-server/*.yaml` — the same for the federation API.
* `data/api/application-service/*.yaml` — Application Service API.
* `content/client-server-api/`, `content/server-server-api.md` — the narrative spec
  (authorization rules, state resolution, error semantics).
* `content/rooms/fragments/v*-auth-rules.md`, `v2-state-res.md` — the algorithms M2.2/M2.4
  must implement.
* `content/changelog/v1.19.md` — what changed in the current release.

**complement**

* `tests/csapi/` — the **stable** spec test suite (107 test functions). This is the G2 gate.
* `tests/mscXXXX/` — unstable MSC suites (informational only, never a release gate).
* `tests/federation_*.go` — the federation test suite (G3 gate).

## Refreshing the archives

```bash
# spec
curl -sL https://github.com/matrix-org/matrix-spec/archive/refs/tags/v1.19.tar.gz \
  -o vendor/matrix-spec-v1.19.tar.gz
# complement
curl -sL https://github.com/matrix-org/complement/archive/refs/heads/main.tar.gz \
  -o vendor/complement-main.tar.gz
```

After refreshing, re-run `make compliance-diff` and update
`docs/compliance/complement-csapi-tests.txt`:

```bash
grep -rho "^func Test[A-Za-z0-9_]*" vendor/complement-main/tests/csapi/*_test.go \
  | sed 's/func //' | sort > docs/compliance/complement-csapi-tests.txt
```
