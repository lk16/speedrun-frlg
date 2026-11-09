#!/usr/bin/env python3
"""Extract species and FireRed wild data from decompiled/ into human-run/data/dex.json.

Sources (all under decompiled/src/data): pokemon/species_info.h (catchRate, expYield, growthRate),
pokemon/evolution.h, wild_encounters.json (labels ending _FireRed). stdlib only.
"""
import json, re, sys, os
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'decompiled', 'src', 'data')
si = open(os.path.join(root, 'pokemon', 'species_info.h')).read()
species = {}
for m in re.finditer(r'\[(SPECIES_[A-Z0-9_]+)\]\s*=\s*\{(.*?)\n    \},', si, re.S):
    name, body = m.groups()
    g = lambda k: re.search(r'\.' + k + r'\s*=\s*([A-Z0-9_]+)', body)
    if not g('catchRate'):
        continue
    species[name] = dict(catch=int(g('catchRate').group(1)), yield_=int(g('expYield').group(1)),
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
for e in wild:
    if not e['base_label'].endswith('_FireRed'):
        continue
    d = {}
    for key in ('land_mons', 'water_mons', 'rock_smash_mons', 'fishing_mons'):
        if key in e and e[key]:
            d[key] = dict(rate=e[key]['encounter_rate'],
                          slots=[[m['min_level'], m['max_level'], m['species']] for m in e[key]['mons']])
    sites[e['map']] = d
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
json.dump(dict(kanto_count=len(kanto), species=species, sites=sites, trainers=trainers), open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'data', 'dex.json'), 'w'), separators=(',', ':'))
print(len(kanto), len(species), len(sites), len(trainers))
