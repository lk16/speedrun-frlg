#!/usr/bin/env python3
"""Run forced.py over the route's legs and gyms and print a table. stdlib only.

Writes human-run/data/forced.json: {leg: {min, always: [...], some: [...]}} with full script labels,
for the simulator's forced-trainer groups.
"""
import json, os, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__))
F = os.path.join(HERE, 'forced.py')

def run(*a):
    return subprocess.run([sys.executable, F, *a], capture_output=True, text=True, timeout=600).stdout

LEGS = {
  'forest': ('MAP_VIRIDIAN_FOREST', '29,62', 'MAP_VIRIDIAN_FOREST', '5,9', []),
  'pewter-cerulean': ('MAP_PEWTER_CITY', '15,16', 'MAP_CERULEAN_CITY', '22,19', ['--ignore', 'Fossil']),
  'cerulean-bill': ('MAP_CERULEAN_CITY', '22,19', 'MAP_ROUTE25', '51,4', []),
  'cerulean-vermilion': ('MAP_CERULEAN_CITY', '22,19', 'MAP_VERMILION_CITY', '15,6', ['--avoid', 'MAP_SAFFRON_CITY']),
  'cerulean-lavender': ('MAP_CERULEAN_CITY', '22,19', 'MAP_LAVENDER_TOWN', '6,5', ['--cut', '--ignore', 'CeruleanCity_EventScript_Grunt', '--avoid', 'MAP_SAFFRON_CITY']),
  'lavender-celadon': ('MAP_LAVENDER_TOWN', '6,5', 'MAP_CELADON_CITY', '48,11', ['--avoid', 'MAP_SAFFRON_CITY']),
  'celadon-fuchsia-cycling': ('MAP_CELADON_CITY', '48,11', 'MAP_FUCHSIA_CITY', '25,31', ['--ignore', 'Snorlax']),
  'lavender-fuchsia': ('MAP_LAVENDER_TOWN', '6,5', 'MAP_FUCHSIA_CITY', '25,31', ['--ignore', 'Snorlax', '--cut']),
  'viridian-route22': ('MAP_VIRIDIAN_CITY', '36,19', 'MAP_ROUTE22', '8,5', []),
  'route22-indigo': ('MAP_ROUTE22', '8,5', 'MAP_INDIGO_PLATEAU_EXTERIOR', '11,6', ['--surf']),
}
if len(sys.argv) > 1:
    LEGS = {k: v for k, v in LEGS.items() if k in sys.argv[1:]}
for name, (ma, a, mb, b, extra) in LEGS.items():
    print('##', name)
    print(run(ma, a, mb, b, *extra))
