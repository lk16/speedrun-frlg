#!/usr/bin/env python3
"""Summarise human-run/data/starter-sweep-*.tsv (see crates/frlg-route/examples/starter-sweep.rs)."""
import sys, collections, json
NAT = ["Hardy","Lonely","Brave","Adamant","Naughty","Bold","Docile","Relaxed","Impish","Lax","Timid","Hasty","Serious","Jolly","Naive","Modest","Mild","Quiet","Bashful","Rash","Calm","Gentle","Sassy","Careful","Quirky"]
rows = [l.rstrip("\n").split("\t") for l in open(sys.argv[1])]
ok = [r for r in rows if len(r) >= 14]
bad = [r for r in rows if len(r) < 14]
def tiers(pid, hp, atk, de, spe, spa, spd):
    n = pid % 25; m3 = n in (15, 16, 19); r2 = n in (16, 19)
    t1 = m3 and spa >= 30
    return dict(T0=m3 and spa >= 25, T1=t1, T1R=r2 and spa >= 30,
                T2=t1 and min(hp, de, spe) >= 20, T3=t1 and min(hp, de, spe) >= 25)
out = {"rows": len(rows), "fail_rows": len(bad)}
byN = collections.defaultdict(list)
for r in ok:
    N, val, rng0, kb, j = int(r[0]), int(r[1]), int(r[2], 16), int(r[3]), int(r[4])
    pid = int(r[5], 16); iv = list(map(int, r[6:12])); frames, m = int(r[12]), int(r[13])
    byN[N].append(dict(j=j, val=val, rng0=rng0, kb=kb, pid=pid, iv=iv, frames=frames, m=m))
Ns = sorted(byN)
j0 = {N: byN[N][0] for N in Ns}
out["N_count"] = len(Ns)
out["ball_frames_distinct"] = sorted(set(j0[N]["kb"] for N in Ns))[:10]
out["m_at_j0_distinct"] = dict(collections.Counter(j0[N]["m"] for N in Ns))
out["frames_press_to_creation_distinct"] = dict(collections.Counter(j0[N]["frames"] for N in Ns))
# naming-seed step per frame
d = collections.Counter()
for a, b in zip(Ns, Ns[1:]):
    if b == a + 1:
        diff = (j0[b]["val"] - j0[a]["val"]) % 65536
        d[min((diff - 18752) % 65536, (18752 - diff) % 65536) // 64 * 64] += 1
out["naming_val_step_minus_18752_abs_binned64"] = dict(sorted(d.items())[:12])
out["naming_val_distinct"] = len(set(j0[N]["val"] for N in Ns))
cnt = collections.Counter(); nat = collections.Counter()
for N in Ns:
    g = j0[N]; t = tiers(g["pid"], *g["iv"])
    nat[NAT[g["pid"] % 25]] += 1
    for k, v in t.items(): cnt[k] += v
out["j0_tier_hits"] = dict(cnt)
out["j0_good_N"] = [N for N in Ns if tiers(j0[N]["pid"], *j0[N]["iv"])["T1"]]
# j-dial
steps = collections.Counter(); tot = 0; jN = []
jtier = collections.Counter(); jn = 0; runs = []
for N in Ns:
    rs = byN[N]
    if len(rs) < 10: continue
    jN.append(N)
    for a, b in zip(rs, rs[1:]):
        steps[b["m"] - a["m"]] += 1
    for g in rs:
        jn += 1
        for k, v in tiers(g["pid"], *g["iv"]).items(): jtier[k] += v
    out.setdefault("m_minus_j_range_per_N", {})[N] = [min(g["m"] - g["j"] for g in rs), max(g["m"] - g["j"] for g in rs), len(rs)]
out["m_step_histogram_per_j"] = dict(steps)
out["j_samples"] = jn; out["j_tier_hits"] = dict(jtier)
print(json.dumps(out, indent=1))
