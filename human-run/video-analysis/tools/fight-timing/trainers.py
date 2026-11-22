"""TRAINER id -> (constant, party list) from the decomp. Prints matches for argv name fragments."""
import re, sys, json
D = '/home/luuk/.cache/speedrun-frlg/decompiled/'
ids = {}
for l in open(D + 'include/constants/opponents.h'):
    m = re.match(r'#define (TRAINER_\w+)\s+(\d+)', l)
    if m:
        ids[m.group(1)] = int(m.group(2))
tr = open(D + 'src/data/trainers.h').read()
parties = {}
pt = open(D + 'src/data/trainer_parties.h').read()
for m in re.finditer(r'(sParty_\w+)\[\] = \{(.*?)\n\};', pt, re.S):
    mons = re.findall(r'\.lvl = (\d+),.*?\.species = (SPECIES_\w+)', m.group(2), re.S)
    parties[m.group(1)] = [s[8:] + 'L' + l for l, s in mons]
out = {}
for m in re.finditer(r'\[(TRAINER_\w+)\] = \{(.*?)\n    \},', tr, re.S):
    p = re.search(r'\((sParty_\w+)\)', m.group(2))
    if m.group(1) in ids and p:
        out[ids[m.group(1)]] = (m.group(1), parties.get(p.group(1), []))
json.dump(out, open(__import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/trainers.json', 'w'))
for k, (n, p) in sorted(out.items()):
    if any(a in n for a in sys.argv[1:]):
        print(k, n, p)
