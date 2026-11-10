#!/usr/bin/env python3
"""Extract species and FireRed wild data from decompiled/ into human-run/data/dex.json.

Sources (all under decompiled/src/data): pokemon/species_info.h (catchRate, expYield, growthRate),
pokemon/evolution.h, wild_encounters.json (labels ending _FireRed). stdlib only.
"""
import json, re, sys, os
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'decompiled', 'src', 'data')
si = open(os.path.join(root, 'pokemon', 'species_info.h')).read().replace('[SPECIES_NONE] = {0},', '')
species = {}
for m in re.finditer(r'\[(SPECIES_[A-Z0-9_]+)\]\s*=\s*\{(.*?)\n    \},', si, re.S):
    name, body = m.groups()
    g = lambda k: re.search(r'\.' + k + r'\s*=\s*([A-Z0-9_]+)', body)
    if not g('catchRate'):
        continue
    tm = re.search(r'\.types = \{(TYPE_[A-Z]+),\s*(TYPE_[A-Z]+)\}', body)
    species[name] = dict(types=[tm.group(1), tm.group(2)] if tm else ['TYPE_NORMAL', 'TYPE_NORMAL'], catch=int(g('catchRate').group(1)), yield_=int(g('expYield').group(1)),
                         growth=g('growthRate').group(1), evo=[])
ev = open(os.path.join(root, 'pokemon', 'evolution.h')).read()
for m in re.finditer(r'\[(SPECIES_[A-Z0-9_]+)\]\s*=\s*\{(.*?)\},\n', ev, re.S):
    name, body = m.groups()
    for e in re.finditer(r'\{(EVO_[A-Z_]+),\s*([A-Z0-9_]+|\d+),\s*(SPECIES_[A-Z0-9_]+)\}', body):
        if name in species:
            species[name]['evo'].append([e.group(1), e.group(2), e.group(3)])
# Kanto dex = national numbers 1..151 = enum order up to SPECIES_MEW
order = list(re.findall(r'\[(SPECIES_[A-Z0-9_]+)\]\s*=\s*\{', si))
kanto = []
for n in order:
    if n == 'SPECIES_NONE':
        continue
    kanto.append(n)
    if n == 'SPECIES_MEW':
        break
for n, s in species.items():
    s['kanto'] = n in kanto
    s['dex'] = kanto.index(n) + 1 if n in kanto else 0
wild = json.load(open(os.path.join(root, 'wild_encounters.json')))['wild_encounter_groups'][0]['encounters']
sites = {}
sites_lg = {}
for e in wild:
    suffix = e['base_label'].rsplit('_', 1)[-1]
    if suffix not in ('FireRed', 'LeafGreen'):
        continue
    d = {}
    for key in ('land_mons', 'water_mons', 'rock_smash_mons', 'fishing_mons'):
        if key in e and e[key]:
            d[key] = dict(rate=e[key]['encounter_rate'],
                          slots=[[m['min_level'], m['max_level'], m['species']] for m in e[key]['mons']])
    (sites if suffix == 'FireRed' else sites_lg)[e['map']] = d
tp = open(os.path.join(root, 'trainer_parties.h')).read()
parties = {}
for m in re.finditer(r'static const struct TrainerMon\w+ (sParty_\w+)\[\] = \{(.*?)\n\};', tp, re.S):
    parties[m.group(1)] = [[int(l), sp] for l, sp in re.findall(r'\.lvl = (\d+),\s*\.species = (SPECIES_[A-Z0-9_]+)', m.group(2))]
tr = open(os.path.join(root, 'trainers.h')).read()
trainers = {}
for m in re.finditer(r'\[(TRAINER_[A-Z0-9_]+)\] = \{(.*?)\n    \},', tr, re.S):
    pm = re.search(r'\.party = \w+\((sParty_\w+)\)', m.group(2))
    dbl = 'doubleBattle = TRUE' in m.group(2)
    if pm and pm.group(1) in parties:
        trainers[m.group(1)] = dict(party=parties[pm.group(1)], double=dbl)
maps_dir = os.path.join(root, '..', '..', 'data')
label_trainer = {}
def scan(path):
    cur = None
    for line in open(path, errors='replace'):
        m = re.match(r'^([A-Za-z0-9_]+)::', line)
        if m:
            cur = m.group(1)
        t = re.search(r'trainerbattle_\w+\s+(TRAINER_[A-Z0-9_]+)', line)
        if t and cur and cur not in label_trainer:
            label_trainer[cur] = t.group(1)
for dp, _, fs in os.walk(os.path.join(maps_dir, 'maps')):
    for f in fs:
        if f == 'scripts.inc':
            scan(os.path.join(dp, f))
for f in os.listdir(os.path.join(maps_dir, 'scripts')):
    if f.endswith('.inc'):
        scan(os.path.join(maps_dir, 'scripts', f))
map_trainers = {}
for d in sorted(os.listdir(os.path.join(maps_dir, 'maps'))):
    mj = os.path.join(maps_dir, 'maps', d, 'map.json')
    if not os.path.exists(mj):
        continue
    ids = []
    for o in json.load(open(mj)).get('object_events', []):
        t = label_trainer.get(o.get('script', ''))
        if t and t in trainers and t not in ids:
            ids.append(t)
    if ids:
        map_trainers[d] = ids
bm = open(os.path.join(root, '..', 'battle_main.c')).read()
chart_src = bm.split('const u8 gTypeEffectiveness[336] =')[1].split('};')[0]
mul = dict(TYPE_MUL_SUPER_EFFECTIVE=20, TYPE_MUL_NOT_EFFECTIVE=5, TYPE_MUL_NO_EFFECT=0, TYPE_MUL_NORMAL=10)
chart = [[a, d, mul[m]] for a, d, m in re.findall(r'(TYPE_[A-Z]+),\s*(TYPE_[A-Z]+),\s*(TYPE_MUL_[A-Z_]+)', chart_src) if a not in ('TYPE_FORESIGHT', 'TYPE_ENDTABLE')]
json.dump(dict(type_chart=chart, kanto_count=len(kanto), species=species, sites=sites, sites_lg=sites_lg, trainers=trainers, map_trainers=map_trainers), open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'data', 'dex.json'), 'w'), separators=(',', ':'))
print(len(kanto), len(species), len(sites), len(trainers))
