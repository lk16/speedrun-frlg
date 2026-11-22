"""Sim (1280) per-fight player actions, mean over attempts reaching the League, by sequence position."""
import json, collections
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
att = [json.loads(l) for l in open(SP + 'simfights.jsonl')]
LEAGUE = {'lorelei', 'bruno', 'agatha', 'lance', 'champion'}
seqs = []
for a in att:
    s = [f for f in a['fights'] if f[0].split('-')[0] not in LEAGUE and not f[0].startswith(('lorelei', 'bruno', 'agatha', 'lance', 'champion'))]
    seqs.append(s)
L = collections.Counter(len(s) for s in seqs)
print('sequence lengths', L)
n = L.most_common(1)[0][0]
use = [s for s in seqs if len(s) == n]
out = []
for i in range(n):
    names = collections.Counter(s[i][0] for s in use)
    acts = [s[i][1] for s in use]
    out.append((names.most_common(1)[0][0], sum(acts) / len(acts), sum(s[i][3] for s in use) / len(use)))
json.dump(out, open(SP + 'simseq.json', 'w'))
for i, (nm, a, fr) in enumerate(out):
    print(i, nm, round(a, 2))
