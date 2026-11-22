"""Per trainer fight of the played s11 (+ fwd-main) run: RAM markers, turns, mons."""
import csv, json, re
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
idx = json.load(open(SP + 'wft/csvidx.json'))
bat = []
for l in open(SP + 'exp-s11-battles.txt'):
    p = l.split()
    if len(p) > 3 and p[2] == 'trainer':
        foes = l.split('foes ')[1].strip().split(',')
        lead = int(re.search(r'lead L(\d+)', l).group(1))
        bat.append(dict(s=int(p[0]), e=int(p[1]), tid=int(p[3]), foes=foes, lead=lead))
for i, b in enumerate(bat):
    b['lvups'] = max(0, (bat[i + 1]['lead'] - b['lead'])) if i + 1 < len(bat) else 0
ev = list({(x['frame'], x['text']): x for x in [json.loads(l) for l in open(SP + 'cal-events.jsonl')]
           + [json.loads(l) for l in open(SP + 'cal-fwd-events.jsonl')]}.values())
eps = []
for f in ['cal-pdecomp.json', 'cal-decomp.json', 'cal-fdecomp.json']:
    d = json.load(open(SP + f))
    for st, v in d.items():
        for kind, s, e in v.get('episodes', []):
            eps.append((kind, s, e, st))


def rows(a, b):
    for s, e, f in idx:
        if e < a or s > b or f == 'cal-t-hideout-out.csv':
            continue
        for r in csv.DictReader(open(SP + f)):
            fr = int(r['frame'])
            if fr < a:
                continue
            if fr > b:
                return
            yield fr, r


out = []
for b in bat:
    if b['tid'] == 0:
        continue
    ep = [x for x in eps if x[1] - 5 <= b['s'] <= x[2] and x[0] == 'trainer']
    hs, he, stage = (ep[0][1], ep[0][2], ep[0][3]) if ep else (None, None, None)
    inside = [x for x in ev if b['s'] <= x['frame'] <= b['e'] + 2]
    turns = sum(1 for x in inside if x['kind'] == 'battle' and re.match(r'turn \d+:', x['text']))
    a = hs if hs else b['s'] - 1500
    trans = None
    speech = None
    prev_cb = None
    for fr, r in rows(a, b['s']):
        if r['cb2'] == 'CB2_OverworldBasic' and prev_cb != 'CB2_OverworldBasic' and trans is None:
            trans = fr
        if r['msg'] != '0' and speech is None:
            speech = fr
        prev_cb = r['cb2']
    menu_f = 0
    for fr, r in rows(b['s'], b['e']):
        if r['in_battle'] == '1' and r['cb2'] not in ('BattleMainCB2', 'CB2_HandleStartBattle'):
            menu_f += 1
    out.append(dict(stage=stage, tid=b['tid'], foes=[f[8:] for f in b['foes']], mons=len(b['foes']), turns=turns,
                    lvups=b['lvups'], lead=b['lead'], held=hs, field=he, speech=speech, trans=trans, bs=b['s'], be=b['e'],
                    battle_f=b['e'] - b['s'] - 50, ep_f=(he - hs + 1) if hs else None, menu_f=menu_f))
json.dump(out, open(SP + 'wft/botfights.json', 'w'), indent=0)
for o in out:
    print(o['stage'], o['tid'], o['mons'], o['turns'], o['lvups'], o['battle_f'], o['ep_f'],
          (o['trans'] - o['speech']) if o['trans'] and o['speech'] else None,
          (o['bs'] - o['trans']) if o['trans'] else None, (o['field'] - o['be']) if o['field'] else None, o['menu_f'])
