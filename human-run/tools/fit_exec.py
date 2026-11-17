#!/usr/bin/env python3
"""Evidence fits for the execution constants `mistake_rate` and `trainer_fit` (journal 2026-11-25). stdlib only.

usage: fit_exec.py [PLAN]   (default: plans/CURRENT)  writes human-run/data/fit-exec-<plan>.json

mistake_rate. The model charges `mistake_rate` events per 100 walked tiles and mashed or menu inputs
(`sim.rs` `mistakes`), each U{60..300} frames plus a reaction U{15..30}: `frlg-human units` gives the exposure
of one run and the mean cost. The runner's own tally of one finished run (3:29:05, stream 2026-11-03,
T03 4:30:15 "it's actually like a minute of mistakes", plus 4:30:22-4:30:38 "a minute in early game not super
fast, a minute from the cave": the other two minutes are not mistakes) is 60 s; its uncertainty is taken as
a factor 2 at 95% (a spoken round number) combined with the Poisson noise of the event count.
A wall bump costs 32 frames on foot (`field_player_avatar.c:884-888`, `event_object_movement.c:6161`) and 16 on
the bike (`field_player_avatar.c:878-882`), so the 8-9 "bonks" of T03 3:40:15 are ~10 s of the 60 s.

trainer_fit. Scales the intro and per-mon overhead of the 8 non-League fights of the WR video. For each value the
model's core seconds of those fights are compared with the video's (`frlg-human splits` fight table);
the fit is the value with mean log-ratio 0, its standard error the fight-to-fight sd / sqrt(8) over the slope.
The WR is the best of ~16k attempts but a per-fight selection effect is ~1 s (fights are <1% of the variance of the
run's total), so the 8 fights are treated as unbiased draws of a typical fight.
"""
import json, math, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sens

ROOT = sens.ROOT
BIN = sens.BIN
NL = ['brock', 'misty', 'rival-ss-anne', 'koga', 'blaine', 'sabrina', 'giovanni', 'rival-route22']
PLAN = sys.argv[1] if len(sys.argv) > 1 else open(os.path.join(ROOT, 'human-run', 'plans', 'CURRENT')).read().strip()
p = sens.load_plan(PLAN)

# ---- mistakes
f = sens.write(p, 'fx_units')
u = subprocess.run([BIN, 'units', f], capture_output=True, text=True).stdout
m = re.search(r'units\(100\) ([\d.]+) \| rate ([\d.]+) -> events/run ([\d.]+), (\d+) f \(([\d.]+) s\) per event', u)
units, rate0, ev0, fr_ev, s_ev = float(m[1]), float(m[2]), float(m[3]), int(m[4]), float(m[5])
M_OBS = 60.0          # s of mistakes in the one finished run the runner itemised
sd_log = math.sqrt((math.log(2) / 1.96) ** 2 + 1.0 / max(1.0, M_OBS / s_ev))
c = M_OBS / s_ev / units
mist = dict(units=units, s_per_event=s_ev, plan_rate=rate0, plan_s_per_run=ev0 * s_ev, obs_s=M_OBS, rate=c,
            lo=c * math.exp(-1.96 * sd_log), hi=c * math.exp(1.96 * sd_log), sd_log=sd_log)

# ---- trainer_fit
rows = []
for tf in (0.60, 0.65, 0.70, 0.75, 0.80, 0.85, 0.90):
    q = sens.patch(p, 'trainer_fit', tf)
    fq = sens.write(q, 'fx_tf')
    out = subprocess.run([BIN, 'splits', fq, '--R', '30000', '--seed', '2'], capture_output=True, text=True).stdout.splitlines()
    fights, door = {}, None
    for l in out:
        mm = re.match(r'^\s+([a-z0-9-]+)\s+model\s+([\d.]+) actions core\s+(\d+) s .*WR\s+([\d.]+) actions\s+(\d+) s', l)
        if mm:
            fights[mm[1]] = (int(mm[3]), int(mm[5]))
        mm = re.match(r'^Start E4R1\s+(\d:\d\d:\d\d)\s+(\d:\d\d:\d\d)', l)
        if mm:
            door = sens.hms(mm[2])
    lr = [math.log(fights[n][0] / fights[n][1]) for n in NL]
    mean = sum(lr) / len(lr)
    sd = math.sqrt(sum((x - mean) ** 2 for x in lr) / (len(lr) - 1))
    rows.append(dict(tf=tf, mean_logratio=mean, sd=sd, door_mean_s=door))
xs = [r['tf'] for r in rows]
ys = [r['mean_logratio'] for r in rows]
n = len(xs)
mx, my = sum(xs) / n, sum(ys) / n
slope = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
tf_fit = mx - my / slope + 0.0  # y = my + slope (x - mx) = 0
sd_fight = sum(r['sd'] for r in rows) / n
se_tf = sd_fight / math.sqrt(len(NL)) / slope
tf = dict(rows=rows, slope=slope, fit=tf_fit, se=se_tf, lo=tf_fit - 1.96 * se_tf, hi=tf_fit + 1.96 * se_tf)

# ---- door-arrival anchor: the runner's five arrivals at the Indigo door with the timer >= 2:37 (journal 2026-11-14-22-00)
arr = [2 * 3600 + 39 * 60 + 37, 2 * 3600 + 42 * 60, 2 * 3600 + 45 * 60, 2 * 3600 + 49 * 60]
door = dict(runner_arrivals_s=arr, runner_mean_s=sum(arr) / len(arr),
            note='4 of the 5 door arrivals (the 5th, the finisher, is at ~2:40:00-2:42:00 by its 3:29:05 end and a 47-49 min post-game); the model (plan, no resets) mean/p10/p90 is in rows[*].door_mean_s')
json.dump(dict(plan=PLAN, mistakes=mist, trainer_fit=tf, door=door), open(os.path.join(ROOT, 'human-run', 'data', f'fit-exec-{PLAN}.json'), 'w'), indent=1)
print(json.dumps(dict(plan=PLAN, mistakes=mist, trainer_fit={k: v for k, v in tf.items() if k != 'rows'}), indent=1))
for r in rows:
    print(r)
