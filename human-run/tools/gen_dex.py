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
    sf = re.search(r'\.safariZoneFleeRate\s*=\s*(\d+)', body)
    gi = lambda k: int(re.search(r'\.' + k + r'\s*=\s*(\d+)', body).group(1)) if re.search(r'\.' + k + r'\s*=\s*(\d+)', body) else 0
    base = [gi('baseHP'), gi('baseAttack'), gi('baseDefense'), gi('baseSpeed'), gi('baseSpAttack'), gi('baseSpDefense')]
    ev = [gi('evYield_HP'), gi('evYield_Attack'), gi('evYield_Defense'), gi('evYield_Speed'), gi('evYield_SpAttack'), gi('evYield_SpDefense')]
    species[name] = dict(base=base, ev=ev, safari_flee=int(sf.group(1)) if sf else 0, types=[tm.group(1), tm.group(2)] if tm else ['TYPE_NORMAL', 'TYPE_NORMAL'], catch=int(g('catchRate').group(1)), yield_=int(g('expYield').group(1)),
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
parties_full = {}
for m in re.finditer(r'static const struct TrainerMon\w+ (sParty_\w+)\[\] = \{((?:[^\n]*\};)|(?:\n.*?\n\};))', tp, re.S):
    parties[m.group(1)] = [[int(l), sp] for l, sp in re.findall(r'\.lvl = (\d+),\s*\.species = (SPECIES_[A-Z0-9_]+)', m.group(2))]
    full = []
    for mm in re.finditer(r'\{\s*\.iv = (\d+),\s*\.lvl = (\d+),\s*\.species = (SPECIES_[A-Z0-9_]+),(.*?)\n    \}', m.group(2), re.S):
        mv = re.search(r'\.moves = \{([^}]*)\}', mm.group(4))
        full.append([int(mm.group(2)), mm.group(3), int(mm.group(1)), re.findall(r'MOVE_[A-Z0-9_]+', mv.group(1)) if mv else None])
    parties_full[m.group(1)] = full
tr = open(os.path.join(root, 'trainers.h')).read()
bm0 = open(os.path.join(root, '..', 'battle_main.c')).read()
mt_src = bm0.split('const struct TrainerMoney gTrainerMoneyTable[] =')[1].split('};')[0]
money_table = {c: int(v) for c, v in re.findall(r'\{(TRAINER_CLASS_[A-Z0-9_]+),\s*(\d+)\}', mt_src)}
trainers = {}
for m in re.finditer(r'\[(TRAINER_[A-Z0-9_]+)\] = \{(.*?)\n    \},', tr, re.S):
    pm = re.search(r'\.party = \w+\((sParty_\w+)\)', m.group(2))
    dbl = 'doubleBattle = TRUE' in m.group(2)
    if pm and pm.group(1) in parties:
        cls = re.search(r'\.trainerClass = (TRAINER_CLASS_[A-Z0-9_]+)', m.group(2))
        # decompiled/src/battle_script_commands.c:5373: 4 * lastMonLevel * moneyMultiplier(1) * (double ? 2 : 1) * table value
        val = money_table.get(cls.group(1), 0) if cls else 0
        last = parties[pm.group(1)][-1][0] if parties[pm.group(1)] else 0
        trainers[m.group(1)] = dict(party=parties[pm.group(1)], party_full=parties_full.get(pm.group(1), []), double=dbl, money=4 * last * (2 if dbl else 1) * val)
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
bmv = open(os.path.join(root, 'battle_moves.h')).read()
moves = {}
for m in re.finditer(r'\[(MOVE_[A-Z0-9_]+)\]\s*=\s*\{(.*?)\n    \},', bmv, re.S):
    mn, body = m.groups()
    def f(k, default=None):
        r = re.search(r'\.' + k + r'\s*=\s*([A-Za-z0-9_]+)', body)
        return r.group(1) if r else default
    moves[mn] = dict(power=int(f('power', '0')), type=f('type', 'TYPE_NORMAL'), acc=int(f('accuracy', '0')),
                     effect=f('effect', 'EFFECT_HIT'), prio=int(f('priority', '0')))
lu = open(os.path.join(root, 'pokemon', 'level_up_learnsets.h')).read()
sets = {}
for m in re.finditer(r'static const u16 (s\w+LevelUpLearnset)\[\] = \{(.*?)LEVEL_UP_END', lu, re.S):
    sets[m.group(1)] = [[int(l), mv] for l, mv in re.findall(r'LEVEL_UP_MOVE\(\s*(\d+),\s*(MOVE_[A-Z0-9_]+)\)', m.group(2))]
ptr = open(os.path.join(root, 'pokemon', 'level_up_learnset_pointers.h')).read()
for sp, sn in re.findall(r'\[(SPECIES_[A-Z0-9_]+)\]\s*=\s*(s\w+LevelUpLearnset)', ptr):
    if sp in species and sn in sets and sp != 'SPECIES_NONE':
        species[sp]['learnset'] = sets[sn]
scripts = {k: v for k, v in label_trainer.items() if v in trainers}
json.dump(dict(moves=moves, scripts=scripts, type_chart=chart, kanto_count=len(kanto), species=species, sites=sites, sites_lg=sites_lg, trainers=trainers, map_trainers=map_trainers), open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'data', 'dex.json'), 'w'), separators=(',', ':'))
print(len(kanto), len(species), len(sites), len(trainers))
