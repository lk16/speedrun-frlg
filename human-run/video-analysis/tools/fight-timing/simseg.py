"""WR decisions vs the 1280 sim's actions per segment (the sim fights the WR's list)."""
import json
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
seq = json.load(open(SP + 'simseq.json'))
rows = {r['name']: r for r in json.load(open(SP + 'pairs.json'))}
SEG = {
    'forest-trainers': ['Bug Catcher Sammy'], 'pewter-gym-trainer': ['Camper Liam'], 'brock': ['Brock'],
    'route3-trainers': ['Bug Catcher Colton', 'Bug Catcher Greg', 'Bug Catcher James'], 'mt-moon-extra': ['Bug Catcher Robby'],
    'mt-moon-miguel': ['Mt. Moon grunt', 'Super Nerd Miguel'], 'misty': ['Misty'], 'rival-cerulean': ['Rival Cerulean'],
    'nugget-bridge-trainers': ['Bug Catcher Cale', 'Lass Ali', 'Youngster Timmy', 'Lass Reli', 'Camper Ethan', 'Route 24 grunt'],
    'route25-trainers': ['Hiker Wayne', 'Picnicker Kelsey', 'Camper Flint', 'Lass Haley'], 'dig-grunt': ['Cerulean grunt 5'],
    'route6-trainers': ['Camper Jeff'], 'rival-ss-anne': ['Rival S.S. Anne'],
    'rock-tunnel-trainers': ['Picnicker Alicia', 'Bug Catcher Conner', 'PokeManiac Ashton', 'PokeManiac Winston', 'Picnicker Martha',
                             'Hiker Allen', 'Hiker Lenny', 'Picnicker Sofia', 'Hiker Dudley', 'Picnicker Dana'],
    'route8-trainers': ['Gamer Rich'], 'hideout-grunts': ['Game Corner grunt 7', 'Hideout grunt 18', 'Hideout grunt 16', 'Hideout grunt 17'],
    'hideout-giovanni': ['Giovanni 1'], 'rival-tower': ['Rival tower'],
    'tower-channelers': ['Channeler Tammy', 'Channeler Jennifer', 'Channeler Emilia'],
    'tower-rockets': ['Tower grunt 19', 'Tower grunt 20', 'Tower grunt 21'], 'surge-gym-trainers': ['Gentleman Tucker'],
    'surge': ['Lt. Surge'], 'fuchsia-gym-trainers': ['Juggler Kirk'], 'koga': ['Koga'], 'blaine': ['Blaine'],
    'three-island-goon-1': ['Biker Goon'], 'three-island-goon-2': ['Biker Goon 2'], 'three-island-goon-3': ['Biker Goon 3'],
    'three-island-paxton': ['Cue Ball Paxton'], 'bond-bridge-violet': ['Aroma Lady Violet'], 'rival-silph': ['Rival Silph'],
    'silph-grunts': ['Silph grunt'], 'sabrina': ['Sabrina'], 'celadon-gym-trainers': ['Beauty Tamia', 'Beauty Lori'],
    'erika': ['Erika'], 'viridian-gym-trainers': ['Black Belt Takashi', 'Cooltrainer Warren'], 'giovanni': ['Giovanni 3'],
    'rival-route22': ['Rival Route 22'],
}
tw = ts = tb = tbs = 0
print(f"{'segment':24s} WR  sim  | bot (same fights)")
for seg, names in SEG.items():
    sim = sum(x[1] for x in seq if x[0] == seg)
    wr = sum(rows[n]['wr_menus'] for n in names)
    b = [rows[n] for n in names if 'bot_turns' in rows[n]]
    bot = sum(r['bot_turns'] for r in b) if len(b) == len(names) else None
    tw += wr
    ts += sim
    if bot is not None:
        tb += bot
        tbs += sim
    print(f"{seg:24s} {wr:3d} {sim:5.1f} | {bot if bot is not None else '-'}")
print(f"total WR {tw} vs sim {ts:.1f} (ratio {tw / ts:.3f}); where the bot fought the same list: bot {tb} vs sim {tbs:.1f}")

# model fight time (trainer_cost at the sim's mean actions) vs the WR's episodes, per segment
import random, statistics as S
F = 59.7275
OV = S.median(r['bot_ep'] - r['bot_dur'] for r in rows.values() if r.get('bot_ep'))
segrows = []
for seg, names in SEG.items():
    sim = sum(x[1] for x in seq if x[0] == seg)
    mons = sum(rows[n]['mons'] for n in names)
    nf = len(names)
    model = (838 * nf + 579 * sim + 223 * mons) / F
    model_clock = (sum(x[2] for x in seq if x[0] == seg) + 240 * nf) / F
    wr_ep = sum(rows[n]['wr_dur'] + ((rows[n]['bot_ep'] - rows[n]['bot_dur']) if rows[n].get('bot_ep') else OV) for n in names)
    wr_at = sum((838 + 579 * rows[n]['wr_menus'] + 223 * rows[n]['mons']) / F for n in names)
    segrows.append((seg, model, wr_ep, wr_at, model_clock))
m = sum(r[1] for r in segrows); w = sum(r[2] for r in segrows); a = sum(r[3] for r in segrows)
print(f"model (trainer_cost at sim actions) {m:.0f} s; trainer_cost at WR decisions {a:.0f} s; WR episodes {w:.0f} s")
print(f"WR / model {w / m:.3f}; speed part WR / tc(WR decisions) {w / a:.3f}; turn part tc(WR) / tc(sim) {a / m:.3f}")
rnd = random.Random(1)
v = []
for _ in range(4000):
    s = [rnd.choice(segrows) for _ in segrows]
    v.append(sum(r[2] for r in s) / sum(r[1] for r in s))
v.sort()
print(f"bootstrap over segments, WR/model 95% [{v[100]:.3f}, {v[3900]:.3f}]")
for r in segrows:
    print(f"  {r[0]:24s} model {r[1]:6.1f} WR {r[2]:6.1f} diff {r[2] - r[1]:+6.1f}")
mc = sum(r[4] for r in segrows)
print(f"model clock per fight (sim fight frames incl. reactions/jitter + outro, no level-ups) {mc:.0f} s; WR / model clock {w / mc:.3f}")
v = []
for _ in range(4000):
    s = [rnd.choice(segrows) for _ in segrows]
    v.append(sum(r[2] for r in s) / sum(r[4] for r in s))
v.sort()
print(f"  95% [{v[100]:.3f}, {v[3900]:.3f}]")

# variants: model clock under a different trainer_cost, + the sim's level-up text (~40 trainer level-ups x 240 f)
LU = 40 * 240 / F
print(f"\nlevel-up text the sim adds on top (~40 trainer level-ups x 240 f): {LU:.0f} s")
for lab, tcv in (('1280 [838,579,223]', (838, 579, 223)), ('x0.9 (1283)', (754.2, 521.1, 200.7)),
                 ('x0.944', (791, 547, 211)), ('per-action 0.82 [838,475,223]', (838, 475, 223))):
    mcv = mc
    wat = 0
    for seg, names in SEG.items():
        sim = sum(x[1] for x in seq if x[0] == seg)
        mons = sum(rows[n]['mons'] for n in names)
        nf = len(names)
        mcv -= ((838 - tcv[0]) * nf + (579 - tcv[1]) * sim + (223 - tcv[2]) * mons) / F
        wat += sum((tcv[0] + tcv[1] * rows[n]['wr_menus'] + tcv[2] * rows[n]['mons']) / F for n in names)
    print(f"{lab:30s} model fights {mcv:5.0f} s (+LU {mcv + LU:5.0f}); WR {w:.0f}: WR/model {w / mcv:.3f} (incl. LU {w / (mcv + LU):.3f}); at WR decisions WR/tc {w / wat:.3f}")
