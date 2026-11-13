#!/usr/bin/env python3
"""Add each trainer's party natures (and its name) to human-run/data/dex.json.

A trainer's mons are not neutral: `CreateNPCTrainerParty`
(`decompiled/src/battle_main.c:1539-1620`) gives mon i the personality

    base + (nameHash_i << 8),   base = 0x88 (0x78 for a female trainer class,
                                0x80 for a double battle),
    nameHash_i = (i + 1) * sum(trainer name bytes) + sum(species name bytes of mons 0..=i)

(`nameHash` is declared once and the trainer-name loop sits inside the party loop, so it
accumulates). The nature is `personality % 25` (`GetNature`) and the ability slot is the
personality's lowest bit, 0 for all three bases. Bytes are the charmap encoding
(`decompiled/charmap.txt`).

Usage: add_natures.py [DECOMP_ROOT]  (defaults to $FRLG_DECOMP_RO). Idempotent.
"""
import json
import os
import re
import sys

here = os.path.dirname(os.path.abspath(__file__))
root = sys.argv[1] if len(sys.argv) > 1 else os.environ.get('FRLG_DECOMP_RO')
if not root:
    sys.exit('set FRLG_DECOMP_RO or pass the decomp root')

charmap = {}
for line in open(os.path.join(root, 'charmap.txt'), encoding='utf-8'):
    m = re.match(r"^'(.+)'\s*=\s*([0-9A-Fa-f ]+)$", line.rstrip('\n'))
    if m:
        key = m.group(1)
        charmap["'" if key == "\\'" else key] = int(m.group(2).split()[0], 16)


def enc_sum(text):
    total = 0
    for ch in text:
        total += charmap[ch]
    return total


species_names = {}
sn = open(os.path.join(root, 'src', 'data', 'text', 'species_names.h'), encoding='utf-8').read()
for m in re.finditer(r'\[(SPECIES_[A-Z0-9_]+)\] = _\("([^"]*)"\)', sn):
    species_names[m.group(1)] = m.group(2)

trainers_h = open(os.path.join(root, 'src', 'data', 'trainers.h'), encoding='utf-8').read()
info = {}
for m in re.finditer(r'\[(TRAINER_[A-Z0-9_]+)\] = \{(.*?)\n    \},', trainers_h, re.S):
    body = m.group(2)
    name = re.search(r'\.trainerName = _\("([^"]*)"\)', body)
    if not name:
        continue
    female = 'F_TRAINER_FEMALE' in body
    double = 'doubleBattle = TRUE' in body
    info[m.group(1)] = (name.group(1), female, double)

path = os.path.join(here, '..', 'data', 'dex.json')
dex = json.load(open(path))
n = 0
for tid, t in dex['trainers'].items():
    if tid not in info:
        continue
    name, female, double = info[tid]
    try:
        tsum = enc_sum(name)
    except KeyError:
        continue
    base = 0x80 if double else (0x78 if female else 0x88)
    h = 0
    natures = []
    ok = True
    for mon in t['party_full']:
        sp = species_names.get(mon[1])
        if sp is None:
            ok = False
            break
        h += tsum + enc_sum(sp)
        pers = (base + (h << 8)) & 0xFFFFFFFF
        natures.append(pers % 25)
    if ok:
        t['name'] = name
        t['natures'] = natures
        n += 1
json.dump(dex, open(path, 'w'), separators=(',', ':'))
print('natures for', n, 'trainers')
