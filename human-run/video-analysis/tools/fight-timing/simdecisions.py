"""Usage: frlg-human fightdist <plan> --rung WR+160m --R 8000 --seed 7401 --from rival-lab > fd.jsonl; simdecisions.py fd.jsonl [...].
bos-cmp.py fightdist.jsonl [more.jsonl ...]: per segment, mean sim decisions (League-reaching attempts) vs WR."""
import json, sys, collections
REPO = __import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)), '..', '..', '..', '..') + '/'
src = open(REPO + 'human-run/video-analysis/tools/fight-timing/simseg.py').read()
SEG = eval(src[src.index('SEG = {') + 6:src.index('}\ntw') + 1])
wr = {r['name']: r for r in json.load(open(REPO + 'human-run/video-analysis/data/wr-fight-timing.json'))}
LEAGUE = ('lorelei', 'bruno', 'agatha', 'lance', 'champion')
BOSS = {'brock', 'misty', 'rival-cerulean', 'rival-ss-anne', 'hideout-giovanni', 'rival-tower', 'surge', 'koga',
        'blaine', 'rival-silph', 'sabrina', 'erika', 'giovanni', 'rival-route22'}
cols = []
for fn in sys.argv[1:]:
    # per segment: mean over attempts that fought the segment and a later one (completed it)
    acc = collections.defaultdict(float); fr = collections.defaultdict(float); lvd = collections.defaultdict(float); cnt = collections.Counter(); n = 0
    for l in open(fn):
        a = json.loads(l); n += 1
        fs = a['fights']
        last = fs[-1][0] if fs else None
        per = collections.defaultdict(float); perf = collections.defaultdict(float); perl = collections.defaultdict(float)
        for f in fs:
            per[f[0]] += f[1]; perf[f[0]] += f[3]; perl[f[0]] += f[4] if len(f) > 4 else 0
        for k in per:
            if k == last and not a.get('finished'):
                continue
            acc[k] += per[k]; fr[k] += perf[k]; lvd[k] += perl[k]; cnt[k] += 1
    cols.append(({k: v / cnt[k] for k, v in acc.items()}, {k: v / cnt[k] for k, v in fr.items()}, n, {k: v / cnt[k] for k, v in lvd.items()}))
print(f"{'segment':24s}  WR " + ' '.join(f'{fn.split("/")[-1][:9]:>9s}' for fn in sys.argv[1:]))
tot = [0.0] * len(cols); tw = 0; bt = [0.0] * len(cols); bw = 0
ftot = [0.0] * len(cols); ltot = [0.0] * len(cols)
for seg, names in SEG.items():
    w = sum(wr[n]['wr_menus'] for n in names); tw += w
    if seg in BOSS:
        bw += w
    vals = [c[0].get(seg, 0.0) for c in cols]
    for i, v in enumerate(vals):
        tot[i] += v
        ftot[i] += cols[i][1].get(seg, 0.0)
        ltot[i] += cols[i][3].get(seg, 0.0)
        if seg in BOSS:
            bt[i] += v
    print(f"{seg:24s} {w:3d} " + ' '.join(f'{v:9.2f}' for v in vals))
print(f"{'TOTAL':24s} {tw:3d} " + ' '.join(f'{v:9.1f}' for v in tot))
print(f"{'bosses':24s} {bw:3d} " + ' '.join(f'{v:9.1f}' for v in bt))
print(f"{'route':24s} {tw - bw:3d} " + ' '.join(f'{a - b:9.1f}' for a, b in zip(tot, bt)))
print(f"{'fight s (WR list)':24s}     " + ' '.join(f'{v / 59.7275:9.0f}' for v in ftot))
print(f"{'level-ups (WR list)':24s}     " + ' '.join(f'{v:9.1f}' for v in ltot))
nf = sum(len(v) for v in SEG.values())
print(f"{'total s (+outro+lvup)':24s}     " + ' '.join(f'{(f + 240 * (nf + l)) / 59.7275:9.0f}' for f, l in zip(ftot, ltot)) + '   WR 3684')
print('attempts', [c[2] for c in cols])
