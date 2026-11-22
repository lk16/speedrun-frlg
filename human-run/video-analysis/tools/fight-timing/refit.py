"""Bot episodes: trainer_cost fit with and without a level-up term (the sim adds `level_up` per level on top)."""
import json, sys
sys.path.insert(0, __import__('os').path.dirname(__file__))
from fit import ols, boot  # noqa
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
b = [x for x in json.load(open(SP + 'botfights.json')) if x['ep_f'] and x['turns'] > 0]
y = [x['ep_f'] for x in b]
c3 = ols([[1, x['turns'], x['mons']] for x in b], y)
c4 = ols([[1, x['turns'], x['mons'], x['lvups']] for x in b], y)
print(f"n {len(b)}; level-ups {sum(x['lvups'] for x in b)} ({sum(x['lvups'] for x in b) / len(b):.2f}/fight)")
print('a, t, m       :', [round(v) for v in c3])
print('a, t, m, lvup :', [round(v) for v in c4])
# with the level-up term fixed at the sim's 240 f: fit a, t, m on ep - 240 * lvups
c5 = ols([[1, x['turns'], x['mons']] for x in b], [x['ep_f'] - 240 * x['lvups'] for x in b])
print('a, t, m | lvup fixed 240:', [round(v) for v in c5])
ci = [boot(b, lambda s, i=i: ols([[1, x['turns'], x['mons']] for x in s], [x['ep_f'] - 240 * x['lvups'] for x in s])[i], n=2000) for i in range(3)]
print('  95% CIs', [(round(l), round(h)) for l, h in ci])
