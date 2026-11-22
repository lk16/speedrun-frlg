"""For each identified WR fight: the start of the last segment of the chain if it has no menu (evolution candidate)."""
import json, importlib.util
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
spec = importlib.util.spec_from_file_location('bt', __import__('os').path.join(__import__('os').path.dirname(__file__), '..', 'battle-timeline.py'))
bt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bt)
rows = json.load(open(SP + 'pairs.json'))
wr = {round(json.loads(l)['t0'], 3): json.loads(l) for l in open(SP + 'wr-full.jsonl')}
d = open(SP + 'wr-full.raw', 'rb').read()
pal = bt.PAL['wr']
out = {}
for r in rows:
    w = wr[r['wr_t0']]
    a, b = int(w['t0'] * 60), int(w['t1'] * 60)
    st = [bt.classify(d[i * 39:(i + 1) * 39], pal)[0] for i in range(a, b)]
    ks = [(x, y) for s, x, y in bt.runs(st) if s == 'K' and y - x >= 4]
    if not ks:
        continue
    x, y = ks[-1]
    last_menu = max(w['menu_at']) if w['menu_at'] else w['t0']
    start = (a + y) / 60
    if start > last_menu:
        out[r['name']] = [round(start, 2), round((y - x) / 60, 2), round(w['t1'] - start, 1)]
        print(r['name'], out[r['name']])
json.dump(out, open(SP + 'lastseg.json', 'w'))
