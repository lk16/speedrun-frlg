"""WR trainer fights (identified by the intro text on contact sheets) x the played run's same fights."""
import json
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
FPS_B = 59.7275
# video t0 (rounded) -> (name, trainer id, mons); ids from decompiled/include/constants/opponents.h,
# mons from src/data/trainer_parties.h. Silph grunt and Gamer Rich have custom-move parties.
WR = [
    (641, 'Bug Catcher Sammy', 104, 1), (754, 'Camper Liam', 142, 2), (820, 'Brock', 414, 2),
    (973, 'Bug Catcher Colton', 105, 3), (1024, 'Bug Catcher Greg', 106, 4), (1084, 'Bug Catcher James', 107, 2),
    (1226, 'Bug Catcher Robby', 109, 3), (1392, 'Mt. Moon grunt', 351, 2), (1428, 'Super Nerd Miguel', 170, 3),
    (1627, 'Misty', 415, 2), (1738, 'Rival Cerulean', 333, 4), (1849, 'Bug Catcher Cale', 110, 4),
    (1902, 'Lass Ali', 123, 3), (1949, 'Youngster Timmy', 92, 2), (1986, 'Lass Reli', 122, 2),
    (2021, 'Camper Ethan', 144, 1), (2054, 'Route 24 grunt', 356, 2), (2110, 'Hiker Wayne', 184, 1),
    (2140, 'Picnicker Kelsey', 153, 2), (2177, 'Camper Flint', 471, 2), (2221, 'Lass Haley', 125, 3),
    (2341, 'Cerulean grunt 5', 355, 2), (2426, 'Camper Jeff', 146, 2), (2508, 'Rival S.S. Anne', 427, 4),
    (2811, 'Picnicker Alicia', 154, 4), (2871, 'Bug Catcher Conner', 115, 3), (2961, 'PokeManiac Ashton', 168, 2),
    (3010, 'PokeManiac Winston', 166, 1), (3043, 'Picnicker Martha', 159, 2), (3094, 'Hiker Allen', 190, 1),
    (3122, 'Hiker Lenny', 192, 4), (3185, 'Picnicker Sofia', 158, 3), (3241, 'Hiker Dudley', 189, 3),
    (3296, 'Picnicker Dana', 474, 3), (3364, 'Gamer Rich', 264, 2), (3524, 'Game Corner grunt 7', 357, 2),
    (3601, 'Hideout grunt 18', 368, 2), (3682, 'Hideout grunt 16', 366, 3), (3737, 'Hideout grunt 17', 367, 3),
    (3848, 'Giovanni 1', 348, 3), (4101, 'Rival tower', 430, 5), (4312, 'Channeler Tammy', 447, 1),
    (4340, 'Channeler Jennifer', 453, 1), (4366, 'Channeler Emilia', 452, 1), (4433, 'Tower grunt 19', 369, 3),
    (4485, 'Tower grunt 20', 370, 2), (4525, 'Tower grunt 21', 371, 4), (4664, 'Gentleman Tucker', 423, 1),
    (4697, 'Lt. Surge', 416, 3), (5353, 'Juggler Kirk', 288, 4), (5428, 'Koga', 418, 4),
    (6093, 'Blaine', 419, 4), (6427, 'Biker Goon', 527, 2), (6463, 'Biker Goon 2', 528, 1),
    (6490, 'Biker Goon 3', 529, 1), (6518, 'Cue Ball Paxton', 742, 2), (6657, 'Aroma Lady Violet', 558, 3),
    (7414, 'Rival Silph', 433, 5), (7580, 'Silph grunt', None, 3), (7840, 'Sabrina', 420, 4),
    (7973, 'Beauty Tamia', 266, 2), (8015, 'Beauty Lori', 267, 1), (8045, 'Erika', 417, 3),
    (8314, 'Black Belt Takashi', 324, 3), (8415, 'Cooltrainer Warren', 401, 5), (8551, 'Giovanni 3', 350, 5),
    (8689, 'Rival Route 22', 436, 6),
]
wr = [json.loads(l) for l in open(SP + 'wr-full.jsonl')]
LAST = json.load(open(SP + 'lastseg.json'))
EVO = {'Bug Catcher Robby', 'Rival Silph', 'Silph grunt', 'Black Belt Takashi', 'Cooltrainer Warren'}
bot = json.load(open(SP + 'botfights.json'))
botv = [json.loads(l) for l in open(SP + 'bot-full.jsonl')]
rows = []
for t, name, tid, mons in WR:
    w = min(wr, key=lambda r: abs(r['t0'] - t))
    assert abs(w['t0'] - t) < 4, (t, w['t0'])
    b = [x for x in bot if x['tid'] == tid]
    row = dict(name=name, tid=tid, mons=mons, wr_t0=w['t0'], wr_dur=w['dur_s'], wr_menus=w['menus'],
               wr_menu_s=w['menu_s'], wr_pre=w['pre_s'], wr_segs=w['segs'])
    if b:
        b = b[0]
        # the bot's visible battle = RAM inBattle + 34 f .. - 16 f (checked on the bot render)
        row.update(bot_dur=round(b['battle_f'] / FPS_B, 2), bot_turns=b['turns'], bot_lvups=b['lvups'],
                   bot_lead=b['lead'], bot_ep=round(b['ep_f'] / FPS_B, 2) if b['ep_f'] else None,
                   bot_pre=round((b['trans'] - b['speech']) / FPS_B, 2) if b['trans'] and b['speech'] else None,
                   bot_menu_s=round(b['menu_f'] / FPS_B, 2))
        v = [x for x in botv if abs(x['t0'] * FPS_B - b['bs'] - 34) < 60]
        if v:
            row.update(botv_dur=v[0]['dur_s'], botv_menus=v[0]['menus'], botv_menu_s=v[0]['menu_s'], botv_pre=v[0]['pre_s'])
    # post-battle evolution scenes (checked on sheets: "X is evolving!") are not fight time: the chain's
    # last segment and the black before it are cut
    if name in EVO:
        start, black, length = LAST[name]
        row['wr_dur'] = round(row['wr_dur'] - length - black, 2)
        row['wr_evo_cut'] = round(length + black, 1)
    rows.append(row)
json.dump(rows, open(SP + 'pairs.json', 'w'), indent=0)
print(f"{'fight':22s} mons | WR dur menus menu_s pre | bot dur turns menus menu_s pre lv")
for r in rows:
    s = f"{r['name']:22s} {r['mons']:4d} | {r['wr_dur']:6.1f} {r['wr_menus']:3d} {r['wr_menu_s']:5.1f} {r['wr_pre'] or 0:4.1f}"
    if 'bot_dur' in r:
        s += f" | {r['bot_dur']:6.1f} {r['bot_turns']:3d} {r.get('botv_menus', '-')!s:>3} {r.get('botv_menu_s', 0):5.1f} {r.get('botv_pre') or 0:4.1f} {r['bot_lvups']}"
    print(s)
