#!/usr/bin/env python3
"""The scores ledger: archived rows in human-run/scores-archive/*.jsonl.gz, then the live human-run/scores.jsonl.

Writers keep appending to the live file. Readers use `lines()`, which yields every row oldest first (archives in
name order, then the live file), so "last matching row wins" still means the newest.

usage: scores.py archive   move every live row into scores-archive/<YYYY-MM-DD-HHMM>.jsonl.gz and empty the live
                           file. Run it at wrap-up with no agent branches outstanding (see README.md). stdlib only.
"""
import glob, gzip, os, sys, time

HR = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
LIVE = os.path.join(HR, 'scores.jsonl')
ARCHIVE = os.path.join(HR, 'scores-archive')


def lines():
    for path in sorted(glob.glob(os.path.join(ARCHIVE, '*.jsonl.gz'))):
        with gzip.open(path, 'rt') as f:
            yield from f
    if os.path.exists(LIVE):
        with open(LIVE) as f:
            yield from f


def archive():
    data = open(LIVE, 'rb').read()
    if not data.strip():
        sys.exit('scores.jsonl is empty; nothing to archive')
    os.makedirs(ARCHIVE, exist_ok=True)
    out = os.path.join(ARCHIVE, time.strftime('%Y-%m-%d-%H%M') + '.jsonl.gz')
    if os.path.exists(out):
        sys.exit(f'{out} exists; wait a minute')
    before = list(lines())
    with gzip.GzipFile(out, 'wb', mtime=0) as f:  # mtime=0: same rows, same bytes
        f.write(data)
    assert gzip.open(out, 'rb').read() == data
    open(LIVE, 'w').close()
    assert list(lines()) == before
    rows = data.count(b'\n')
    print(f'{out}: {rows} rows, {len(data)} -> {os.path.getsize(out)} bytes; scores.jsonl emptied')


if __name__ == '__main__':
    if sys.argv[1:] != ['archive']:
        sys.exit(__doc__)
    archive()
