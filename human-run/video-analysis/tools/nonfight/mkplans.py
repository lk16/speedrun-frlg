import json, sys
R = '/home/luuk/projects/speedrun-frlg/.claude/worktrees/agent-a37a90bda8b357d6d/'
S = '/tmp/claude-1000/-home-luuk-projects-speedrun-frlg/bf1774e1-c01a-4d6b-9ba8-a55653344b5b/scratchpad/'
src = R + 'human-run/plans/1423-boss-turns-whole.md'
t = open(src).read()
j = json.loads(t[t.index('```json') + 7:t.rindex('```')])
# WR VOD (video = run + 35 s), frames at 59.7275/s; journal 2026-11-22 nonfight-windows.
VOD = {
    "misty-walk": 1075,          # Cerulean center door 1603.0 -> Misty's speech ~1621
    "misty-badge": 850,          # Misty field (fade) 1686.8 -> gym exit 1701.0
    "captain-hm01": 3740,        # rival field 2586.0 -> Vermilion control after the departure 2648.6
    "teleport-cerulean-2": 3225,  # walk (no Abra) fan club 2721.0 -> bike shop door 2775.0
    "koga-badge": 1325,          # Koga field 5507.2 -> gym exit 5529.4
    "blaine-quiz-text": 4240,    # gym door 6010.1 -> Blaine's speech ~6086, minus blaine-walk's 5 s
    "blaine-badge": 1690,        # Blaine field 6178.9 -> Pokemon Center (Bill) 6207.2
    "sevii-bill-boat": 1925,     # Bill 6208.5 -> One Island PC 6240.7
    "celio-tri-pass": 2475,      # Celio 6241.3 -> out 6282.7
    "lostelle-hypno": 450,       # Lostelle's text before the static Hypno (7.5 s of 6884.8-6896.7)
    "blaine-badge-and-sevii-scene": 1980,  # Celio/Bill 7069.9 -> Cinnabar arrival text done 7103.0
    "erika-badge": 910,          # Erika field 8095.4 -> exit 8110.6
    "viridian-gym-walk": 1285,   # door->Takashi 4.5 s, ->Warren 7 s, ->Giovanni 10 s
    "giovanni-badge": 1190,      # Giovanni field 8638.1 -> gym exit 8658.0
    "mt-moon-1f": 1585,          # Mt. Moon field time 1170.9-1182.4 + 1211.5-1226.5 (walls, items; catches out)
    "mt-moon-b1f": 1000,         # 1293.6-1318.4 minus the move-learn text (17 s)
    "mt-moon-b2f": 2460,         # 1350.3-1391.6 (to the B2F grunt)
    "route4": 1430,              # fossil choice text after Miguel 1488.8-1501.9 (no model segment) + Route 4 floor 669 f
}
EVO = {"evo_stone_frames": 1600, "evo_candy_frames": 1620}
# Corrected (plans 1444+): a leader's / rival's post-battle field speech is inside the fitted fight price (the
# played episode runs from losing control to regaining it: Misty overhead 15.8 s, Koga 15.5, S.S. Anne rival 14.6),
# so the badge / captain lumps keep only what follows the speech (walking out), and the Mt. Moon floors lose the
# two trainers' approach + speech (~3 s each).
VOD2 = dict(VOD)
VOD2.update({
    "misty-badge": 345,          # 14.2 s, of it 8.4 s text box (W) -> 5.8 s walk-out
    "koga-badge": 880,           # 22.2 s, 7.5 s text -> 14.7 s
    "blaine-badge": 1285,        # 28.3 s, 6.8 s text -> 21.5 s to the Pokemon Center
    "erika-badge": 405,          # 15.2 s, 8.4 s text -> 6.8 s
    "giovanni-badge": 585,       # 19.9 s, 10.1 s text -> 9.8 s
    "captain-hm01": 3350,        # from the end of the rival's speech 2592.55 -> 2648.6
    "mt-moon-1f": 1405,
    "mt-moon-b2f": 2280,
})
variants = {
    'nfw-1440': {"vod_frames": VOD, **EVO},
    'nfw-1441': {"vod_frames": VOD},
    'nfw-1442': dict(EVO),
    'nfw-1443': {"vod_frames": VOD, **EVO, "repel_on": j["repel_on"] + ["mt-moon-b1f", "mt-moon-b2f"]},
    'nfw-1444': {"vod_frames": VOD2, **EVO},
    'nfw-1445': {"vod_frames": VOD2},
    'nfw-1446': {"vod_frames": VOD2, **EVO, "repel_on": j["repel_on"] + ["mt-moon-b1f", "mt-moon-b2f"]},
}
for name, d in variants.items():
    k = dict(j)
    k.update(d)
    k['name'] = name
    open(S + name + '.md', 'w').write('# ' + name + '\n\n```json\n' + json.dumps(k, indent=1) + '\n```\n')
json.dump(variants, open(S + 'nfw-variants.json', 'w'), indent=1)
print('ok')
