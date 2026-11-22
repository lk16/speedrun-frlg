#!/usr/bin/env python3
"""Battle timeline of a run video from per-frame colour cells (no OCR, stdlib only).

Input: the raw file of `battle-feat.sh` (13 RGB cells per frame: the whole game window, 8 cells of its
bottom band at 76-96 % height, 4 cells of its top band at 5-25 %), the video's frame rate, a palette
name (`wr`: Main97's VOD, `bot`: the played-run renders in `$FRLG_ARTIFACTS/videos/`) and an offset
in seconds added to every time (for windowed extractions).

    python3 battle-timeline.py FEAT.raw FPS PALETTE [OFFSET_S] > battles.jsonl

Per frame the bottom band is classified as
  K  black (whole-window mean below the palette's black level: transitions, fades),
  T  the battle text box (dark teal fill with its edge colours in the outer cells),
  A  the battle action menu (text box on the left half, white FIGHT/BAG box on the right half),
  M  the move menu (white across the band while the top band is the flat battle background),
  W  any other white band (overworld text box), O otherwise.
Segments lie between black runs (>= 4 frames). A battle segment has the text box up in its first
second (the intro slide-in shows it at once) or in 30 % of its frames; a battle is a chain of battle
segments, also across one short (< 6 s) non-battle segment (bag, party screen) that does not end in
an overworld text box. Per battle:
  t0, t1    first frame after the transition's black run / first frame of the black run after the
            last battle segment (fade-out done), video seconds; dur_s = t1 - t0
  menus     runs of A/M frames merged across gaps < 0.25 s: one per player decision (FIGHT -> move
            is one); menu_s the seconds in them (input speed), menu_at their starts
  pre_s     seconds from the first frame of the overworld text episode that ends < 2 s before the
            transition black (the trainer's speech) to that black; None if there is none
  trans_s   seconds from the last full-brightness frame to the black run (transition effect)
  out_s     seconds from t1 to the first frame at >= 80 % of the luminance a second later (field)
"""
import json
import sys

PAL = {
    # edge colours of the battle text box (bottom cells 7, 8), black level of the window mean,
    # minimum channel of a "white" cell
    "wr": {"edge": ((0x2C, 0x44, 0x4D), (0x3B, 0x53, 0x5D)), "black": 20, "white": 0x80},
    "bot": {"edge": ((0x2F, 0x51, 0x61), (0x44, 0x66, 0x76)), "black": 14, "white": 0xA8},
}


def near(c, ref, tol=22):
    return all(abs(a - b) <= tol for a, b in zip(c, ref))


def classify(f, pal):
    c = [tuple(f[j * 3:j * 3 + 3]) for j in range(13)]
    lum = sum(c[0]) / 3
    if lum < pal["black"]:
        return "K", lum
    bot = c[1:9]
    w = pal["white"]
    bright = [min(x) >= w for x in bot]
    if near(bot[6], pal["edge"][0]) and near(bot[7], pal["edge"][1]):
        return "T", lum
    if all(bright[4:]) and max(bot[2]) < 0x70 and max(bot[3]) < 0x70:
        return "A", lum
    if all(bright):
        top = c[9:]
        flat = max(max(x) for x in top) - min(min(x) for x in top) < 0x40
        return ("M" if flat and min(min(x) for x in top) >= w else "W"), lum
    return "O", lum


def runs(states):
    out = []
    s = 0
    for i in range(1, len(states) + 1):
        if i == len(states) or states[i] != states[s]:
            out.append((states[s], s, i))
            s = i
    return out


def main():
    path, fps, pal = sys.argv[1], float(sys.argv[2]), PAL[sys.argv[3]]
    off = float(sys.argv[4]) if len(sys.argv) > 4 else 0.0
    data = open(path, "rb").read()
    n = len(data) // 39
    st = []
    lums = []
    for i in range(n):
        s, lum = classify(data[i * 39:(i + 1) * 39], pal)
        st.append(s)
        lums.append(lum)
    blacks = [(a, b) for s, a, b in runs(st) if s == "K" and b - a >= 4]
    sec = int(fps)

    def is_battle(a, b):
        seg = st[a:b]
        head = seg[:sec]
        return len(seg) > 0 and (head.count("T") >= 0.5 * len(head) or seg.count("T") >= 0.3 * len(seg))

    def ends_in_speech(a, b):
        # a trainer's speech box closes and the battle transition (0.4-1.2 s of darkening) follows;
        # a bag / party screen's white box is followed by a quick fade (< 0.2 s)
        i = b - 1
        while i > a and st[i] != "W" and b - i < 2 * sec:
            i -= 1
        return st[i] == "W" and b - 1 - i >= 0.4 * fps

    segs = [(blacks[k][1], blacks[k + 1][0], k) for k in range(len(blacks) - 1)]
    k = 0
    while k < len(segs):
        a, b, kb = segs[k]
        if not is_battle(a, b):
            k += 1
            continue
        last = k
        j = k + 1
        while j < len(segs):
            a2, b2, _ = segs[j]
            if is_battle(a2, b2):
                last = j
                j += 1
            else:
                # up to three short non-battle segments (bag -> party screen -> ...) back into battle:
                # together < 10 s, none ending in a trainer's speech, and the black run before the
                # battle screen comes back shorter than a battle transition's (>= 0.8 s; a menu's
                # fade back is 0.5-0.75 s)
                m = j
                while (m < len(segs) and m - j < 3 and not is_battle(segs[m][0], segs[m][1])
                       and segs[m][1] - segs[j][0] < 10 * sec and not ends_in_speech(segs[m][0], segs[m][1])):
                    m += 1
                bk = blacks[segs[m][2]] if m < len(segs) else (0, 0)
                if (m > j and m < len(segs) and m - j <= 3 and is_battle(segs[m][0], segs[m][1])
                        and bk[1] - bk[0] < 0.78 * fps):
                    last = m
                    j = m + 1
                else:
                    break
        t0, t1 = a, segs[last][1]
        a0, b0 = blacks[kb]
        nseg = last - k + 1
        k = last + 1
        if t1 - t0 < 3 * sec:
            continue
        body = st[t0:t1]
        menus = []
        for s, x, y in runs(body):
            if s in "AMW":  # inside a battle a white band is a menu (cave backgrounds fail the M test)
                if menus and x - menus[-1][1] < fps / 4:
                    menus[-1][1] = y
                    menus[-1][2] += y - x
                else:
                    menus.append([x, y, y - x])
        pre = None
        i = a0 - 1
        while i > a0 - 2 * sec and i > 0 and st[i] != "W":
            i -= 1
        if st[i] == "W":
            gap = 0
            while i > max(0, a0 - 15 * sec):
                if st[i] == "W":
                    pre, gap = i, 0
                elif st[i] == "K":
                    break
                else:
                    gap += 1
                    if gap > fps * 0.3:
                        break
                i -= 1
        tr = a0
        ref = lums[max(0, a0 - 2 * sec)]
        while tr > max(1, a0 - 3 * sec) and lums[tr - 1] < 0.9 * ref:
            tr -= 1
        out = t1
        while out < n - 1 and (st[out] == "K" or lums[out] < 0.8 * lums[min(n - 1, out + sec)]):
            out += 1
        print(json.dumps({
            "t0": round(off + t0 / fps, 3),
            "t1": round(off + t1 / fps, 3),
            "dur_s": round((t1 - t0) / fps, 2),
            "segs": nseg,
            "black_in_s": round((b0 - a0) / fps, 2),
            "menus": len(menus),
            "menu_s": round(sum(m[2] for m in menus) / fps, 2),
            "menu_at": [round(off + (t0 + m[0]) / fps, 2) for m in menus],
            "pre_s": round((a0 - pre) / fps, 2) if pre is not None else None,
            "trans_s": round((a0 - tr) / fps, 2),
            "out_s": round((out - t1) / fps, 2),
            "frac_T": round(body.count("T") / len(body), 2),
        }))


if __name__ == "__main__":
    main()
