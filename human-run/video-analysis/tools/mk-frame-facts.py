import json

V = 'frlgr2-wr-3.25.25-main97.mp4'


def hms(t):
    return '%d:%02d:%02d' % (t // 3600, (t % 3600) // 60, t % 60)


rows = []


def r(t, kind, what, value, note=''):
    rows.append({'video': V, 't': t, 'hms': hms(t), 'kind': kind, 'what': what, 'value': value, 'note': note})


# ---- carry identity, level / HP / stats --------------------------------------------------------------
r(9333, 'item', 'carry held item', 'BLACKGLASSES', 'PC party pane "A / BLASTOISE Lv50 BLACKGLASSES"; 2:35:33, just before E4R1')
for t, lv, hp, hpmax, note in [
    (5435, 40, 33, 120, 'Koga start, Torrent window (<= 120/3)'),
    (5496, 41, 4, 123, 'Koga end'),
    (6130, 41, 109, 123, 'Blaine start (full-ish, outside Torrent)'),
    (7848, 45, 43, 135, 'Sabrina start (Kadabra out), Torrent window (<= 45)'),
    (8559, 48, 35, 143, 'Giovanni 3 start, Torrent window (<= 47.67)'),
    (8700, 49, 147, 147, 'Route 22 rival start, full HP'),
    (9395, 50, 150, 150, 'Lorelei start'),
    (9455, 51, 28, 153, 'after Dewgong'),
    (9540, 52, 20, 156, 'after Lapras'),
    (9659, 53, 51, 159, 'after Bruno'),
    (9731, 54, 54, 162, 'after Agatha'),
    (9911, 55, 85, 165, 'after Lance Dragonite'),
    (11658, 59, 109, 178, 'R2 party screen: Blastoise A Lv59 109/178 (benched)'),
]:
    r(t, 'carry-hp', 'carry Blastoise Lv%d HP' % lv, {'level': lv, 'hp': hp, 'max_hp': hpmax}, note)
r(7857, 'stats', 'carry level-up stat box Lv46 (the only stat box the runner leaves on screen long enough)',
  {'level': 46, 'hp': 138, 'atk': 96, 'def': 106, 'spa': 110, 'spd': 100, 'spe': 97},
  'sheet 3 s step at 7830+27; engine EV0 with IV 15/26/9/29/30/20 Rash: hp 135 atk 93 def 101 spa 106 spd 99 spe 90 -> EV/4 about 3 HP, 3 Atk, 5 Def, 3 SpA, 0 SpD, 7 Spe')

# ---- X items / bag counts ----------------------------------------------------------------------------
for t, what, val, note in [
    (8703, 'bag X Special', 20, 'Route 22 rival, before the first item (list shows x20 under the cursor on X Accuracy)'),
    (8709, 'bag X Special', 19, 'Route 22 rival, second item turn (X Speed 4, X Attack 3, X Accuracy 2, Guard Spec 2)'),
    (9402, 'bag X Special', 17, 'Lorelei turn 2 (X Speed 4, X Accuracy 2, Guard Spec 2, X Attack 3); T1 was already an X Special (18 -> 17)'),
    (9425, 'bag X Special', 15, 'Lorelei turn 4'),
    (9581, 'bag X Special', 14, 'Bruno turn 2 (after the Full Restore of turn 1); Lorelei therefore used 4 (18 -> 14)'),
    (9590, 'bag X Special', 13, 'Bruno turn 3'),
    (9596, 'bag X Special', 12, 'Bruno turn 4, cursor on X Speed (4)'),
    (9695, 'bag X Special', 12, 'Agatha turn 1, cursor on X Speed (3)'),
    (9830, 'bag X Special', 11, 'Lance turn 1 (X Speed 2, X Accuracy 2 -> 1 here)'),
    (9983, 'bag X Special', 10, 'Champion, cursor on X Speed (2); Guard Spec 2 -> 1 and Mist on turn 1'),
    (9998, 'bag X Special', 10, 'Champion, X Special selected'),
    (10010, 'bag X Special', 9, 'Champion'),
    (10016, 'bag X Special', 8, 'Champion'),
    (10037, 'bag X Special', 7, 'Champion, Venusaur out (cursor on X Accuracy; the next action is Blizzard, no 4th X Special is visible)'),
    (11662, 'bag X Special', 6, 'R2 between Lorelei 2 and Bruno 2 (journal v4-e4r2)'),
    (11882, 'bag X Special', 5, 'R2 before Lance 2 (journal v4-e4r2)'),
    (11947, 'bag X Special', 5, 'Lance 2 turn 2 (Safeguard was turn 1)'),
    (11954, 'bag X Special', 4, 'Lance 2 turn 3'),
    (12084, 'bag X Special', 3, 'Champion 2 (no X item used: 3 -> 3)'),
]:
    r(t, 'bag', what, val, note)
r(8709, 'bag', 'bag X Speed', 4, 'X Speed 5 at 2:11 (Celadon stop), 4 here')
r(9402, 'bag', 'bag X Speed', 4, 'Lorelei')
r(9596, 'bag', 'bag X Speed', 4, 'Bruno turn 4: used here (3 at the Agatha bag)')
r(9695, 'bag', 'bag X Speed', 3, 'Agatha turn 1: used here (2 at Lance)')
r(9830, 'bag', 'bag X Speed', 2, 'Lance; 1 left after Champion turn 2 ("A\'s SPEED rose!" at 9998)')
r(9830, 'bag', 'bag X Accuracy', 2, 'Lance turn 1 uses one (-> 1 at 9836)')
r(9998, 'bag', 'bag Guard Spec', 2, 'Champion turn 1: Guard Spec, Mist (-> 1)')
r(8709, 'bag', 'bag X Attack', 3, 'never used: still 3 at Lorelei, Lance, Champion, R2 Lance 2 and Champion 2')

# ---- per-fight item turns (derived from the counts) ---------------------------------------------------
for fight, x, t0 in [
    ('rival-route22', {'special': 2, 'accuracy': 1}, 8685),
    ('lorelei', {'special': 4}, 9384),
    ('bruno', {'special': 2, 'speed': 1}, 9551),
    ('agatha', {'special': 1, 'speed': 1}, 9659),
    ('lance', {'accuracy': 1, 'special': 1}, 9803),
    ('champion', {'guard': 1, 'speed': 1, 'special': 3}, 9983),
    ('lorelei-2', {}, 11536),
    ('bruno-2', {}, 11652),
    ('lance-2', {'special': 2}, 11904),
    ('champion-2', {}, 12048),
]:
    r(t0, 'x-queue', fight, x, 'items used in the fight, from the bag counts above')

# ---- hits with a readable foe bar (non-lethal) --------------------------------------------------------
for t, what, val, note in [
    (5450, 'Koga Muk L39: Surf #1 (Torrent, 33/120, stage 0)', {'bar_px': 32, 'of': 47, 'pct': 68}, 'engine EV0 74-88 of 130 = 57-68%: top of the roll range'),
    (5478, 'Koga Weezing L43: Surf #1 (Torrent)', {'bar_px': 45, 'of': 47, 'pct': 96}, 'engine EV0 95-112 of 108 = 88-104%'),
    (5491, 'Koga Weezing L43: Bite (1/120 HP, Black Glasses)', {'bar_px': 13, 'of': 47, 'pct': 28}, 'engine EV0 + glasses 29-35 of 108 = 27-32%'),
    (9440, 'Lorelei Dewgong L52: Bite #1 at SpA +4 (4 X Specials), Black Glasses (Bite is Dark = special in gen 3)', {'bar_px': 28, 'of': 47, 'pct': 60}, 'engine EV0 79-93 of 171 = 46-54%; the +3% SpA EV and the pixel rounding cover most of the gap'),
    (11623, 'R2 Lorelei Lapras L66: Mewtwo Psychic #1 (stage 0)', {'bar_px': 27, 'of': 47, 'pct': 57}, 'engine, SpA IV 31: 137-162 of 268 = 51-60%; IV 14: 130-154 = 49-57%'),
    (11691, 'R2 Bruno Steelix L65: Fire Blast', {'bar_px': 47, 'of': 47, 'pct': 100}, 'one-shot (engine 300-354 vs HP 192)'),
    (11976, 'R2 Lance Kingdra L66 / Dragonite L66 / Aerodactyl: one Psychic each at SpA +2', {'one_shot': True}, 'engine stage 0 gives 62-65% of Dragonite; +2 (two X Specials) is x2.0'),
]:
    r(t, 'hit', what, val, note)

# ---- Mewtwo -------------------------------------------------------------------------------------------
for t, lv, hp in [(11580, 70, 231), (11724, 71, 235), (11970, 72, 238), (12120, 73, 241)]:
    r(t, 'mewtwo-hp', 'Mewtwo Lv%d max HP' % lv, {'level': lv, 'max_hp': hp}, 'engine: HP IV 5, no HP EVs (journal 2026-11-21)')
r(11542, 'hit', 'R2 Lorelei Dewgong L64: two Psychics, no item (Mewtwo stage 0)', {'psychic_hits': 2}, 'engine SpA IV 31: 127-150 of 209 = 61-72%, two hits')
r(11580, 'mewtwo-item', 'Mewtwo held item', None, 'not seen: no summary / party-detail screen of Mewtwo appears in R2 (3 s sheets of 11580..12400); the HP-bar width never shows an item')

doc = {
    'video': V,
    'video_note': 'WR run, Main97 3:25:25.3, 2026-08-18; run time = video time - 0:35; 1920x1080, game window crop 365,30,1190x795',
    'method': [
        'ffmpeg 3 s keyframe sheets of the game window (12 cells of 420 px per image, timestamp burned in), read by eye: tools in the session scratchpad (sheet.sh, batch.sh), the commands are in journal 2026-11-22-10-00-frame-facts.md',
        'bag counts and HP numbers are read directly; foe HP bars are measured with a 1 s raw-RGB decode (row y=68 of the 480x320 game crop, saturated pixels in x 103..198 = 48 native px, so +-1 px = +-2%)',
        'a t is video seconds of the first frame that shows the value; the neighbouring 3 s cells bound it',
    ],
    'derived': {
        'carry': 'Blastoise A: Lv50 at E4R1 with Black Glasses (not Mystic Water), HP 150/153/156/159/162/165 at Lv50..55 (EV0 engine 146 at Lv50: about 8 HP EV/4 -> 32 EVs); Lv46 stats 138/96/106/110/100/97 (EV/4 about 3 HP, 3 Atk, 5 Def, 3 SpA, 0 SpD, 7 Spe, i.e. ~12-30 SpA EVs)',
        'torrent': 'the WR enters Koga (33/120), Sabrina (43/135) and Giovanni 3 (35/143) inside the 1/3-HP Torrent window; Blaine 109/123 and Route 22 147/147 outside it',
        'x_items': 'League X Special use: Lorelei 4, Bruno 2 (+1 X Speed), Agatha 1 (+1 X Speed), Lance 1 (+1 X Accuracy), Champion 3 (+1 X Speed, +1 Guard Spec); R22 2; R2: Lance 2 two, Champion 2 none, Bruno 2 + Agatha 2 one together; X Attack never',
        'damage': 'the engine reproduces every non-lethal bar reading with EV0 within the roll range (Koga Muk/Weezing, R2 Lapras) or within ~7% (Dewgong at +4): no hidden damage multiplier in the carry or in Mewtwo',
    },
    'readings': rows,
}
open('human-run/video-analysis/data/wr-frame-facts.json', 'w').write(json.dumps(doc, indent=1) + '\n')
print(len(rows))
