#!/usr/bin/env python3
"""Extract the stable Matrix HTTP endpoint inventory from a matrix-spec checkout.

The matrix-spec repository stores one OpenAPI YAML per API area under
`data/api/<area>/*.yaml`.  Each YAML declares its own `basePath` and lists
`paths:` with one HTTP method per operation.

Usage:
    curl -sL https://github.com/matrix-org/matrix-spec/archive/refs/tags/v1.19.tar.gz | tar xz
    python3 docs/compliance/extract_spec_endpoints.py \
        --spec matrix-spec-1.19 \
        --out /tmp/endpoints119.json

Outputs JSON: [{area, api, file, method, path, added, deprecated, summary}, ...]
  area     CS | SS | AS | ID | PG
  added    Matrix version the endpoint first appeared in ("" = unknown)
  deprecated  true if the spec marks the operation deprecated
"""

import argparse
import json
import os
import re

AREAS = {
    "client-server": "CS",
    "server-server": "SS",
    "application-service": "AS",
    "identity": "ID",
    "push-gateway": "PG",
}

DEFAULT_BASE = {
    "client-server": "/_matrix/client/v3",
    "server-server": "/_matrix/federation/v1",
    "application-service": "/_matrix/app/v1",
    "identity": "/_matrix/identity/v2",
    "push-gateway": "/_matrix/push/v1",
}


def extract(root):
    out = []
    for area, tag in AREAS.items():
        d = os.path.join(root, area)
        if not os.path.isdir(d):
            continue
        for fn in sorted(os.listdir(d)):
            if not fn.endswith(".yaml"):
                continue
            lines = open(os.path.join(d, fn), encoding="utf-8").read().splitlines()

            base = None
            for i, line in enumerate(lines):
                if line.strip() == "basePath:":
                    for j in range(i + 1, min(i + 5, len(lines))):
                        m = re.search(r"default:\s*(\S+)", lines[j])
                        if m:
                            base = m.group(1)
                            break
                    break
            base = base or DEFAULT_BASE.get(area, "")

            in_paths = False
            cur = None
            for i, line in enumerate(lines):
                if line.startswith("paths:"):
                    in_paths = True
                    continue
                if in_paths and line and not line.startswith(" ") and not line.startswith("#"):
                    in_paths = False
                if not in_paths:
                    continue

                m = re.match(r'^  "([^"]+)":\s*$', line) or re.match(r"^  (/\S+):\s*$", line)
                if m:
                    cur = m.group(1)
                    continue

                m = re.match(r"^    (get|post|put|delete|head|options):\s*$", line)
                if not (m and cur):
                    continue

                method = m.group(1).upper()
                op = []
                for j in range(i + 1, min(i + 120, len(lines))):
                    lj = lines[j]
                    if re.match(r"^    (get|post|put|delete|head|options):", lj) or re.match(r'^  "', lj):
                        break
                    op.append(lj)
                ops = "\n".join(op)

                added = re.search(r'x-addedInMatrixVersion:\s*"?([0-9.]+)', ops)
                summary = ""
                for lj in op:
                    ms = re.match(r"^      summary:\s*(.+)$", lj)
                    if ms:
                        summary = ms.group(1).strip()
                        break

                out.append(
                    {
                        "area": tag,
                        "api": area,
                        "file": fn,
                        "method": method,
                        "path": (base + cur).strip(),
                        "added": added.group(1) if added else "",
                        "deprecated": "deprecated: true" in ops,
                        "summary": summary,
                    }
                )
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--spec", default="matrix-spec-1.19", help="path to the matrix-spec checkout")
    ap.add_argument("--out", default="/tmp/endpoints119.json")
    args = ap.parse_args()

    root = os.path.join(args.spec, "data", "api")
    if not os.path.isdir(root):
        raise SystemExit(f"spec data dir not found: {root}")

    out = extract(root)
    json.dump(out, open(args.out, "w"), indent=1)
    print(f"{len(out)} endpoints -> {args.out}")
    for tag in AREAS.values():
        print(f"  {tag}: {sum(1 for e in out if e['area'] == tag)}")


if __name__ == "__main__":
    main()
