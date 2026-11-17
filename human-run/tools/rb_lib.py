#!/usr/bin/env python3
"""Library for the evidence-weighted EHT interval (ROBUSTNESS.md section 14). stdlib only.

Patches a plan dict with named constants (lane scale, door survival rows, league/mewtwo/carry power, trainer_fit,
mistake_rate, win_cv, slow_scale, hidden-item frames, museum frames, ...), runs `frlg-human eval` / `report` on it
and parses the result. Scratch files go under $RB_SCRATCH (default /tmp/frlg-rb) with the prefix `rb-`.
"""
import json, math, os, re, subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('RB_SCRATCH', '/tmp/frlg-rb')
os.makedirs(SCRATCH, exist_ok=True)

POST = ['lorelei-2', 'bruno-2', 'agatha-2', 'lance-2', 'champion-2', 'icefall-grunt', 'warehouse-admin-1', 'warehouse-admin-2',
        'warehouse-gideon', 'lorelei', 'bruno', 'agatha', 'lance', 'champion']
# the pickups plan 1058 added: key -> (list, after, kind). Frames the plan assumes: 300 / 450 / 450 / 450 / 350.
NEW_PICKS = {'r9': ('elixirs', 'route9', 'candy'), 'r17': ('elixirs', 'cycling-road', 'candy'),
             'hideout': ('elixirs', 'hideout-giovanni', 'calcium'), 'silph': ('elixirs', 'rival-silph', 'calcium'),
             'sell': ('specials', 'celadon-tea', 'sell')}
# map-geometry estimates (journal 2026-12-05): median frames of each new pickup
GEO_FRAMES = {'r9': 170, 'r17': 1030, 'hideout': 400, 'silph': 1170, 'sell': 300}
LANE_TACTICS = ['rival-ss-anne', 'rival-cerulean', 'misty']
HIDDEN_KINDS = ('max', 'ether', 'maxether', 'elixir')   # `hid`; PP Ups have their own knob `ppup_f`


def load_plan(name):
    t = open(os.path.join(ROOT, 'human-run', 'plans', name + '.md')).read()
    return json.loads(t.split('```json')[1].split('```')[0])


def flat_logsurv(p):
    s = 0.0
    for k in POST:
        f = p.get('fatal', {}).get(k, 0.0)
        s += math.log(1 - f) if f < 1 else -50
    return s


def patch(p, **kw):
    """Apply named constants. Absolute values unless noted. Returns a new dict."""
    q = json.loads(json.dumps(p))
    m = q.setdefault('manip', {})
    for k, v in kw.items():
        if v is None:
            continue
        if k == 'lane':            # multiplier on every risk-lane carry power
            for w in q.get('risk_windows', []):
                w['power'] = round(w['power'] * v, 5)
            for t in LANE_TACTICS:
                ft = q.get('fight_tactics', {}).get(t)
                if ft and ft.get('risk_power'):
                    ft['risk_power'] = round(ft['risk_power'] * v, 5)
        elif k == 'flat':          # multiplier on the log-survival of the flat post-door rows
            q['fatal'] = {r: (min(0.999, 1 - (1 - f) ** v) if r in POST else f) for r, f in q.get('fatal', {}).items()}
        elif k == 'lp':
            q['league_power'] = v
        elif k == 'mp':
            q['mewtwo_power'] = v
        elif k == 'cp':
            q['carry_power'] = v
        elif k == 'tf':
            q['trainer_fit'] = v
        elif k == 'mr':
            q['mistake_rate'] = v
        elif k == 'win_cv':
            q['win_cv'] = v
        elif k == 'slow':
            q['slow_scale'] = v
        elif k == 'phit':
            m['p_hit'] = v
        elif k == 'hid':           # multiplier on the Elixir / Max Elixir / Ether pickup detours
            for e in q.get('elixirs', []):
                if e.get('kind') in HIDDEN_KINDS:
                    e['frames'] = int(round(e['frames'] * v))
        elif k == 'ppup_f':        # absolute frames of each PP Up pickup
            for e in q.get('elixirs', []):
                if e.get('kind') in ('ppup', 'ppmax'):
                    e['frames'] = int(round(v))
        elif k == 'museum':        # multiplier on the Aerodactyl / Old Amber special
            for e in q.get('specials', []):
                if 'SPECIES_AERODACTYL' in e.get('species', []):
                    e['frames'] = int(round(e['frames'] * v))
        elif k in ('pk', 'pkg'):   # pk: multiplier on the plan's frames of the 1058 pickups; pkg: multiplier on GEO_FRAMES (absolute)
            for key, (lst, after, kind) in NEW_PICKS.items():
                for e in q.get(lst, []):
                    if e.get('after') == after and e.get('kind') == kind:
                        e['frames'] = int(round(e['frames'] * v)) if k == 'pk' else int(round(GEO_FRAMES[key] * v))
        elif k == 'persim_frames':
            q['persim_frames'] = int(v)
        elif k == 'persim':
            q['persim'] = bool(v)
        elif k == 'set':           # dotted-key raw sets: {"a.b": value}
            for dk, dv in v.items():
                d = q
                parts = dk.split('.')
                for part in parts[:-1]:
                    d = d.setdefault(part, {})
                d[parts[-1]] = dv
        else:
            raise SystemExit('unknown knob ' + k)
    return q


def write(q, tag):
    f = os.path.join(SCRATCH, 'rb-' + tag + '.md')
    q = dict(q)
    q['name'] = 'rb-' + tag
    open(f, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
    return f


def run_eval(q, tag, rung, R, seed):
    f = write(q, tag)
    out = subprocess.run([BIN, 'eval', f, '--rung', rung, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True).stdout.strip().splitlines()
    line = out[-1] if out else ''
    m = re.search(r'finishes (\d+)/(\d+) EHT ([\d.]+) h', line)
    if not m:
        return None, 0
    return float(m.group(3)), int(m.group(1))


def run_report(q, tag, rung, R, seed):
    f = write(q, tag)
    out = subprocess.run([BIN, 'report', f, '--rung', rung, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True).stdout.splitlines()
    reach = {}
    for l in out:
        m = re.match(r'^(\S+)\s+([\d.]+)%\s+(\d+:\d\d:\d\d)', l)
        if m:
            reach[m.group(1)] = float(m.group(2)) / 100.0
    return reach


def QD(reach):
    """Pre-door survival (past the starter gate to the door heal) and door-to-finish survival."""
    return reach['indigo-heal'] / reach['rival-lab'], reach['hall-of-fame-2'] / reach['lorelei']
