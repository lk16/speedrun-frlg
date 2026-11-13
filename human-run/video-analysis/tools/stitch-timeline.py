#!/usr/bin/env python3
"""Stitch timer-sheet transcriptions into an attempt timeline.

Input: JSONL files with {"t": "H:MM:SS" video time, "timer": "<LiveSplit big
timer as shown>"} sampled every minute. The timer only ever counts up within an
attempt, so any decrease (or a 0.0 after a positive value) is a reset; a
monotone continuation belongs to the same attempt. Samples the transcriber
could not read ("unreadable", "?") are skipped.

Output: one line per inferred attempt: video time of the first/last sample,
first/last timer value seen, and a lower bound on the attempt's length (the
last sampled timer; the attempt may have run on up to ~1 min past it).
Attempts shorter than one sampling interval can be missed entirely between
samples — the count is a floor, not an exact number.
"""
import json
import sys


def parse_ts(s):
    parts = s.split(":")
    try:
        parts = [float(p) for p in parts]
    except ValueError:
        return None
    if len(parts) == 3:
        return parts[0] * 3600 + parts[1] * 60 + parts[2]
    if len(parts) == 2:
        return parts[0] * 60 + parts[1]
    if len(parts) == 1:
        return parts[0]
    return None


def fmt(sec):
    sec = int(sec)
    return f"{sec // 3600}:{sec % 3600 // 60:02d}:{sec % 60:02d}"


def main(paths):
    rows = []
    for p in paths:
        with open(p) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                r = json.loads(line)
                vt = parse_ts(r["t"])
                tm = parse_ts(str(r.get("timer", "")))
                if vt is not None:
                    rows.append((vt, tm))
    rows.sort()
    attempts = []
    cur = None  # [start_vt, start_timer, last_vt, last_timer]
    for vt, tm in rows:
        if tm is None:
            continue
        if tm == 0:
            if cur:
                attempts.append(cur)
                cur = None
            continue
        if cur and tm >= cur[3] - 5 and (tm - cur[3]) <= (vt - cur[2]) + 90:
            cur[2], cur[3] = vt, tm
        else:
            if cur:
                attempts.append(cur)
            cur = [vt, tm, vt, tm]
    if cur:
        attempts.append(cur)
    print(f"{len(attempts)} attempts visible at this sampling (floor):")
    for a in attempts:
        print(
            f"  video {fmt(a[0])} .. {fmt(a[2])}  timer {fmt(a[1])} -> {fmt(a[3])}"
            f"  (ran >= {fmt(a[3])})"
        )


if __name__ == "__main__":
    main(sys.argv[1:])
