#!/usr/bin/env python3
"""Diff the routes registered by Maelstrom against the stable Matrix spec.

Extracts every `Router::route(...)` registration from the Rust sources, then
compares it with the endpoint inventory produced by
`docs/compliance/extract_spec_endpoints.py`.

Usage:
    python3 docs/compliance/extract_spec_endpoints.py --spec matrix-spec-1.19 --out /tmp/endpoints119.json
    python3 docs/compliance/diff_routes.py --endpoints /tmp/endpoints119.json

Exit code 0 always; the report is printed to stdout and written to --out.
"""

import argparse
import glob
import json
import os
import re

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CRATES = os.path.join(REPO, "crates")


def split_top(s):
    """Split an argument blob on top-level commas."""
    parts, depth, cur = [], 0, ""
    instr = False
    esc = False
    for c in s:
        if instr:
            cur += c
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                instr = False
            continue
        if c == '"':
            instr = True
            cur += c
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append(cur)
            cur = ""
            continue
        cur += c
    if cur.strip():
        parts.append(cur)
    return parts


def top_methods(expr):
    """HTTP verbs chained at the top level of a method-router expression."""
    out = []
    depth = 0
    i = 0
    while i < len(expr):
        c = expr[i]
        if c == '"':
            i += 1
            while i < len(expr) and expr[i] != '"':
                if expr[i] == "\\":
                    i += 1
                i += 1
            i += 1
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif depth == 0:
            m = re.match(r"(get|post|put|delete|patch|head)\s*\(", expr[i:])
            if m:
                out.append(m.group(1).upper())
                i += m.end() - 1
                continue
        i += 1
    return out or ["ANY"]


def extract_routes(crates):
    routes = []
    for path in glob.glob(os.path.join(crates, "**", "*.rs"), recursive=True):
        src = open(path, encoding="utf-8").read()
        for m in re.finditer(r"\.route\(", src):
            i = m.end()
            depth = 1
            blob = ""
            while i < len(src) and depth > 0:
                c = src[i]
                if c == '"':
                    blob += c
                    i += 1
                    while i < len(src) and src[i] != '"':
                        if src[i] == "\\":
                            blob += src[i]
                            i += 1
                        blob += src[i]
                        i += 1
                    blob += '"'
                    i += 1
                    continue
                if c == "(":
                    depth += 1
                elif c == ")":
                    depth -= 1
                    if depth == 0:
                        break
                blob += c
                i += 1
            parts = split_top(blob)
            if not parts:
                continue
            lit = re.findall(r'"((?:[^"\\]|\\.)*)"', parts[0])
            if not lit:
                continue
            p = lit[0]
            methods = top_methods(parts[1]) if len(parts) > 1 else ["ANY"]
            line = src[: m.start()].count("\n") + 1
            variants = [p]
            if "{prefix}" in p:
                # events.rs registers the same handlers under v3 and r0
                variants = [p.replace("{prefix}", pre) for pre in ("/_matrix/client/v3", "/_matrix/client/r0")]
            for v in variants:
                for meth in methods:
                    routes.append(
                        {
                            "method": meth,
                            "path": v,
                            "file": os.path.relpath(path, REPO),
                            "line": line,
                        }
                    )

    seen, uniq = set(), []
    for r in routes:
        k = (r["method"], r["path"])
        if k in seen:
            continue
        seen.add(k)
        uniq.append(r)
    return uniq


def norm(p):
    p = p.strip().replace("{{", "{").replace("}}", "}")
    p = re.sub(r"\{[^}]+\}", "{}", p)
    return p.rstrip("/") or "/"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--endpoints", default="/tmp/endpoints119.json")
    ap.add_argument("--out", default=os.path.join(REPO, "docs", "compliance", "endpoint-gap-v1.19.txt"))
    ap.add_argument("--areas", default="CS,SS", help="comma separated spec areas to check")
    args = ap.parse_args()

    impl = extract_routes(CRATES)
    spec = json.load(open(args.endpoints, encoding="utf-8"))
    wanted = set(args.areas.split(","))
    spec = [s for s in spec if s["area"] in wanted]

    by_path = {}
    for r in impl:
        by_path.setdefault(norm(r["path"]), []).append(r)

    missing, mism, covered = [], [], []
    for s in spec:
        n = norm(s["path"])
        cands = by_path.get(n)
        if not cands:
            missing.append(s)
        elif any(c["method"] == s["method"] for c in cands):
            covered.append(s)
        else:
            mism.append((s, cands))

    lines = []
    lines.append(f"impl route entries: {len(impl)}")
    lines.append("")
    lines.append("=== SPEC COVERAGE SUMMARY")
    lines.append(
        f"spec {args.areas} endpoints: {len(spec)}  covered: {len(covered)}  "
        f"missing: {len(missing)}  method-mismatch: {len(mism)}"
    )
    lines.append("")
    lines.append(f"=== MISSING ({len(missing)})")
    for s in sorted(missing, key=lambda x: (x["area"], x["path"])):
        lines.append(f"  [{s['area']}] {s['method']:<6} {s['path']}   ({s['file']})")
    lines.append("")
    lines.append(f"=== METHOD MISMATCH ({len(mism)})")
    for s, cands in mism:
        lines.append(
            f"  {s['method']:<6} {s['path']}   impl has: {[c['method'] for c in cands]}   ({s['file']})"
        )
    lines.append("")
    spec_keys = {(s["method"], norm(s["path"])) for s in spec}
    extra = [
        r
        for r in impl
        if (r["method"], norm(r["path"])) not in spec_keys
        and norm(r["path"]).startswith(("/_matrix", "/.well-known"))
    ]
    lines.append(f"=== IMPLEMENTED, NOT IN STABLE SPEC ({len(extra)})")
    for r in extra:
        lines.append(f"  {r['method']:<6} {r['path']}   ({r['file']}:{r['line']})")

    report = "\n".join(lines)
    print(report)
    with open(args.out, "w", encoding="utf-8") as f:
        f.write(report + "\n")
    print(f"\nreport -> {args.out}")


if __name__ == "__main__":
    main()
