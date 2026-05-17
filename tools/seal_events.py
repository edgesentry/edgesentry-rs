#!/usr/bin/env python3
"""Seal RiskEvent JSONL lines with eds audit sign-record (issue #404 demo)."""

from __future__ import annotations

import json
import subprocess
import sys


def main() -> int:
    if len(sys.argv) != 5:
        print(
            "usage: seal_events.py <events.jsonl> <eds-bin> <demo-key-hex> <out-dir>",
            file=sys.stderr,
        )
        return 1

    events_path, eds, demo_key, out_dir = sys.argv[1:5]
    prev = "0" * 64
    seq = 0
    for line in open(events_path, encoding="utf-8"):
        line = line.strip()
        if not line or "rule_id" not in line:
            continue
        ev = json.loads(line)
        seq += 1
        rec = f"{out_dir}/record_{seq}.json"
        payload = json.dumps(ev, sort_keys=True)
        subprocess.run(
            [
                eds,
                "audit",
                "sign-record",
                "--device-id",
                "rpi5-ais-demo",
                "--sequence",
                str(seq),
                "--timestamp-ms",
                str(ev["timestamp_ms"]),
                "--payload",
                payload,
                "--prev-hash-hex",
                prev,
                "--object-ref",
                f"ais-demo/events/{seq}.json",
                "--private-key-hex",
                demo_key,
                "--out",
                rec,
            ],
            check=True,
            stdout=subprocess.DEVNULL,
        )
    print(seq)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
