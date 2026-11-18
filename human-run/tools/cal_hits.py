#!/usr/bin/env python3
"""cal_hits.py: engine damage (frlg-human hit) vs the foe-bar drops measured on the WR / 2026-11-14 videos
(video-analysis/data/calib/league-*.json), non-lethal hits only. A bar is 48 px native; a full bar reads 47.
Observed fraction = px / 47; engine fraction = [lo, hi] / foe max HP. stdlib only."""
import json, os, subprocess
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.path.join(ROOT, 'target', 'release', 'frlg-human')
# trainer, foe index, move, (species, level, ivs, evs, nature, stages, boost), observed px of 47, label
BL = dict(species='SPECIES_BLASTOISE', ivs='15,26,9,29,30,20', evs='36,40,20,60,44,14', nature=19, boost='17,10')
MW = dict(species='SPECIES_MEWTWO', ivs='15,15,15,15,15,15', evs='0,0,0,0,0,0', nature=0, boost='')
CASES = [
    ('WR Lorelei Dewgong Bite +4 SpA', 'TRAINER_ELITE_FOUR_LORELEI', 0, 'MOVE_BITE', BL, 50, '0,0,0,4,0', 28),
    ('WR Lorelei Lapras Bite +4 (crit-free?)', 'TRAINER_ELITE_FOUR_LORELEI', 4, 'MOVE_BITE', BL, 52, '0,0,0,4,0', 20),
    ('WR Lance Gyarados Bite +1', 'TRAINER_ELITE_FOUR_LANCE', 0, 'MOVE_BITE', BL, 54, '0,0,0,1,0', 13),
    ('WR Champion Gyarados Bite +3', 'TRAINER_CHAMPION_FIRST_BULBASAUR', 3, 'MOVE_BITE', BL, 56, '0,0,0,3,0', 21),
    ('A1 Lorelei Dewgong Bite +4', 'TRAINER_ELITE_FOUR_LORELEI', 0, 'MOVE_BITE', BL, 52, '0,0,0,4,0', 26),
    ('WR R2 Dewgong Psychic (stage 0)', 'TRAINER_ELITE_FOUR_LORELEI_2', 0, 'MOVE_PSYCHIC', MW, 70, '0,0,0,0,0', 32),
    ('WR R2 Lapras Psychic (stage 0)', 'TRAINER_ELITE_FOUR_LORELEI_2', 4, 'MOVE_PSYCHIC', MW, 70, '0,0,0,0,0', 27),
]


def hit(spec, level, stages, trainer, idx, move):
    a = [BIN, 'hit', '--species', spec['species'], '--level', str(level), '--ivs', spec['ivs'], '--evs', spec['evs'],
         '--nature', str(spec['nature']), '--badges', '85', '--stages', stages, '--foe', '%s:%d' % (trainer, idx), '--move', move]
    if spec['boost']:
        a += ['--boost', spec['boost']]
    return json.loads(subprocess.run(a, capture_output=True, text=True).stdout)


if __name__ == '__main__':
    for lab, tr, idx, mv, spec, lv, st, px in CASES:
        r = hit(spec, lv, st, tr, idx, mv)
        print('%-42s foe %s L%d HP %d | engine %.0f-%.0f%% | video %.0f%% (%d px, +-1px = +-2%%)' % (
            lab, r['foe_species'][8:], r['foe_level'], r['foe_hp'], 100 * r['lo'] / r['foe_hp'], 100 * r['hi'] / r['foe_hp'], 100 * px / 47, px))
