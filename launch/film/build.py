"""Handkit launch film: one set of timing constants -> HyperFrames composition + SFX cue list.

Run: python build.py   (writes index.html, compositions/*.html, work/cues.json)
Every time is film seconds snapped to the 30 fps frame grid; picture and sound read the same T.
"""
import json, os, random

ROOT = os.path.dirname(os.path.abspath(__file__))
# VARIANT=public: no third-party film footage (act 1 = Pexels gestures in B&W, act 2 = an earlier stretch in colour
# with a before/after slider); written to ./public/ (assets shared through a junction), cues to work/cues_public.json
PUBLIC = os.environ.get("VARIANT", "") == "public"
OUT = os.path.join(ROOT, "public") if PUBLIC else ROOT
FPS = 30
W, H = 1920, 1080
ACC = "#FF5B2E"          # the one accent colour
INK = "#F4F2EE"
BG = "#0B0B0D"


def fr(t):
    """snap to the frame grid (floor: picture lands on or just before the sound)"""
    return int(t * FPS + 1e-6) / FPS


# ---- music-derived times (earshot timeline + measured onsets) --------------------------
T = {k: fr(v) for k, v in dict(
    wipe=3.026, split=7.07, nocam=10.953, a2=11.12, flood=11.62, nocam_out=12.13,
    type1=13.13, send1=14.15, split2=15.15,
    cut3=19.125, type2=19.61, send2=20.61, adam_hk=21.11,
    p1=23.12, p2=23.61, p3=24.11, p4=24.61, black=25.12,
    any_a=25.61, any_n=26.12, any_y=26.61,
    drop=27.10, d2=28.09, d3=29.09, d4=30.09, piano=31.10, trip=33.09, skins=35.10,
    sk2=35.61, sk3=36.11, sk4=36.61, sk5=37.09, d9=39.09, d10=40.09, d11=41.09,
    brk=43.07, gap=50.26, ily=50.6, hit=51.852,
    # rock paper scissors: pumps on the beat, throws on 2.0 and 4.0 s
    rps1=52.35, rps2=52.85, rps3=53.35, throw1=53.85, rps4=54.35, rps5=54.85, rps6=55.35, throw2=55.85,
    hold=56.8, four=58.8, lift=60.82, kid=61.82, bump=62.31, ship=62.82, ship_tap=63.82,
    eng=64.8, eng2=65.31, eng3=65.82, eng4=66.32,
    end=68.8, tag=70.82, fade=72.3, out=73.8,
).items()}
DUR = 73.8

FONT = """@font-face{font-family:'Inter Tight';src:url('assets/fonts/inter-tight-latin-wght-normal.woff2') format('woff2');font-weight:100 900;font-style:normal;}"""
BASE_CSS = FONT + """
#root{position:absolute;inset:0;overflow:hidden;font-family:'Inter Tight';color:%s;}
.full{position:absolute;left:0;top:0;width:1920px;height:1080px;}
.vid{position:absolute;left:0;top:0;width:100%%;height:100%%;object-fit:cover;}
.wrap{position:absolute;overflow:hidden;}
.lbl{position:absolute;font-size:30px;font-weight:600;letter-spacing:0.01em;color:rgba(244,242,238,.92);display:flex;align-items:center;gap:12px;}
.lbl i{display:inline-block;width:12px;height:12px;background:%s;border-radius:2px;}
.big{font-weight:820;letter-spacing:-0.045em;line-height:0.9;white-space:nowrap;}
.acc{color:%s;}
.pill{position:absolute;height:104px;border-radius:52px;background:rgba(16,16,18,.86);border:1.5px solid rgba(255,255,255,.16);
 display:flex;align-items:center;padding:0 40px 0 34px;gap:22px;font-size:46px;font-weight:500;letter-spacing:-0.01em;color:%s;box-shadow:0 20px 60px rgba(0,0,0,.45);}
.pill .dot{width:18px;height:18px;border-radius:50%%;background:%s;flex:none;}
.pill .txt{white-space:pre;}
.pill .car{display:inline-block;width:3px;height:50px;background:%s;margin-left:3px;vertical-align:middle;}
.pill .go{width:58px;height:58px;border-radius:50%%;background:rgba(255,255,255,.12);display:flex;align-items:center;justify-content:center;margin-left:18px;flex:none;}
.pill .go b{display:block;width:0;height:0;border-left:11px solid transparent;border-right:11px solid transparent;border-bottom:16px solid %s;}
""" % (INK, ACC, ACC, INK, ACC, ACC, INK)

CUES = []   # (t, role, id, gain_db, extra)


def cue(t, role, sid, gain=0.0, **kw):
    CUES.append(dict(t=round(t, 4), role=role, id=sid, gain=gain, **kw))


def clip(id_, start, dur, inner="", cls="", style="", tag="div", extra=""):
    return f'<{tag} id="{id_}" class="clip {cls}" data-start="{start:.4f}" data-duration="{dur:.4f}" style="{style}" {extra}>{inner}</{tag}>'


def video(id_, src, start, dur, media=0.0, style="", cls="vid", track=1, rate=None):
    r = f'data-playback-rate="{rate}" ' if rate else ""
    return (f'<video id="{id_}" class="clip {cls}" src="assets/v/{src}.mp4" data-start="{start:.4f}" data-duration="{dur:.4f}" '
            f'data-media-start="{media:.4f}" {r}data-track-index="{track}" muted playsinline style="{style}"></video>')


def chars(id_, text):
    out = []
    for i, ch in enumerate(text):
        c = "&nbsp;" if ch == " " else ch
        out.append(f'<span id="{id_}-{i}" class="ch" style="display:none">{c}</span>')
    return "".join(out)


def scene(cid, start, dur, body, js, css=""):
    html = f"""<template>
<style>
{BASE_CSS}
{css}
</style>
<div id="root" data-composition-id="{cid}" data-width="{W}" data-height="{H}" data-start="0" data-duration="{dur:.4f}" data-layout-allow-overlap data-layout-allow-overflow>
{body}
</div>
<script>
(function(){{
const tl = gsap.timeline({{ paused: true }});
{js}
window.__timelines["{cid}"] = tl;
}})();
</script>
</template>
"""
    os.makedirs(os.path.join(OUT, "compositions"), exist_ok=True)
    with open(os.path.join(OUT, "compositions", f"{cid}.html"), "w", encoding="utf-8") as f:
        f.write(html)
    return dict(id=cid, start=start, dur=dur)


def typing_js(prefix, n, t0, dt=2 / FPS):
    """reveal n chars from local t0, one every dt; returns (js, list of char times)"""
    times = [t0 + i * dt for i in range(n)]
    js = "".join(f'tl.set("#{prefix}-{i}",{{display:"inline"}},{t:.4f});' for i, t in enumerate(times))
    return js, times


def caret_js(sel, t0, t1):
    js, t, on = "", t0, True
    while t < t1:
        js += f'tl.set("{sel}",{{opacity:{1 if on else 0}}},{t:.4f});'
        t += 0.4
        on = not on
    return js


scenes = []

# =========================== ACT 1: B&W film 0 -> 12.1 ==================================
S = 0.0
a1_end = T["nocam_out"] + 0.0667
L = lambda t: t - S
rng = random.Random(7)
if PUBLIC:   # 16:9 gate and split panels: the Pexels frame is never cropped
    gx, gy, gw, gh, sw, sh, sy, A1R, A1H, sl = 160, 90, 1600, 900, 900, 506, 287, "a1p_ref", "a1p_hk", "video"
else:
    gx, gy, gw, gh, sw, sh, sy, A1R, A1H, sl = 240, 0, 1440, 1080, 900, 675, 202, "a1_ref", "a1_hk", "film"
weave, flick, scr = "", "", ""
nframes = int(T["nocam"] * FPS)
for i in range(nframes + 1):
    t = i / FPS
    weave += f'tl.set(["#a1-gate","#a1-split"],{{x:{rng.uniform(-2.2, 2.2):.2f},y:{rng.uniform(-1.6, 1.6):.2f}}},{t:.4f});'
    # projector flicker: start-up burst, then a light shimmer
    base = 1.0 if t < 0.12 else (max(0.0, 0.85 - (t - 0.12) * 1.6) if t < 0.65 else 0.0)
    o = min(1.0, base + rng.uniform(0.0, 0.10) + (0.18 if rng.random() < 0.035 else 0))
    flick += f'tl.set("#a1-flick",{{opacity:{o:.3f}}},{t:.4f});'
    if rng.random() < 0.16:
        scr += f'tl.set("#a1-scr",{{x:{rng.uniform(60, gw - 60):.0f},opacity:{rng.uniform(0.12, 0.35):.2f}}},{t:.4f});'
    else:
        scr += f'tl.set("#a1-scr",{{opacity:0}},{t:.4f});'

a1_body = f"""
{clip("a1-bg", 0, T["a2"], cls="full", style="background:#000;")}
<div id="a1-gate" class="wrap" style="left:{gx}px;top:{gy}px;width:{gw}px;height:{gh}px;border-radius:30px;">
  {video("a1-ref", A1R, 0, T["split"], 0)}
  <div id="a1-hkw" class="wrap" style="left:0;top:0;width:{gw}px;height:{gh}px;">
    {video("a1-hk", A1H, T["wipe"], T["split"] - T["wipe"], T["wipe"])}
  </div>
  <div id="a1-line" style="position:absolute;left:0;top:0;width:4px;height:{gh}px;background:{INK};opacity:0;"></div>
  <div id="a1-scr" style="position:absolute;left:0;top:0;width:2px;height:{gh}px;background:#fff;opacity:0;"></div>
  <div class="full" style="width:{gw}px;height:{gh}px;box-shadow:inset 0 0 160px 40px rgba(0,0,0,.85);border-radius:30px;"></div>
  {clip("a1-l1", 0.6, T["split"] - 0.6, f'<span>{sl}</span>', cls="lbl", style="left:44px;bottom:40px;")}
  {clip("a1-l2", T["wipe"], T["split"] - T["wipe"], '<i></i><span>Handkit</span>', cls="lbl", style="right:44px;bottom:40px;")}
</div>
<div id="a1-split" class="full">
  <div id="a1-fl" class="wrap" style="left:40px;top:{sy}px;width:{sw}px;height:{sh}px;border-radius:22px;">
    {video("a1-ref2", A1R, T["split"], T["nocam"] - T["split"], T["split"])}
    <div class="full" style="width:{sw}px;height:{sh}px;box-shadow:inset 0 0 110px 30px rgba(0,0,0,.8);"></div>
    {clip("a1-l3", T["split"], T["nocam"] - T["split"], f'<span>{sl}</span>', cls="lbl", style="left:30px;bottom:26px;")}
  </div>
  <div id="a1-fr" class="wrap" style="left:980px;top:{sy}px;width:{sw}px;height:{sh}px;border-radius:22px;">
    {video("a1-hk2", A1H, T["split"], T["nocam"] - T["split"], T["split"])}
    <div class="full" style="width:{sw}px;height:{sh}px;box-shadow:inset 0 0 110px 30px rgba(0,0,0,.8);"></div>
    {clip("a1-l4", T["split"], T["nocam"] - T["split"], '<i></i><span>Handkit</span>', cls="lbl", style="left:30px;bottom:26px;")}
  </div>
</div>
{clip("a1-flick", 0, T["nocam"], cls="full", style="background:#000;")}
{clip("a1-word", T["nocam"], a1_end - T["nocam"], '<div id="a1-w" class="big" style="font-size:236px;">NO CAMERA.</div>', cls="full", style="display:flex;align-items:center;justify-content:center;")}
"""
a1_js = weave + flick + scr + f"""
tl.fromTo("#a1-hkw",{{clipPath:"inset(0px 0px 0px {gw}px)"}},{{clipPath:"inset(0px 0px 0px {gw // 2}px)",duration:0.55,ease:"expo.inOut"}},{T["wipe"]:.4f});
tl.fromTo("#a1-line",{{x:{gw},opacity:1}},{{x:{gw // 2 - 2},opacity:1,duration:0.55,ease:"expo.inOut"}},{T["wipe"]:.4f});
tl.set("#a1-line",{{opacity:0}},{T["split"]:.4f});
tl.fromTo("#a1-fl",{{x:240,scale:1.06}},{{x:0,scale:1,duration:0.5,ease:"expo.out"}},{T["split"]:.4f});
tl.fromTo("#a1-fr",{{x:-240,scale:1.06}},{{x:0,scale:1,duration:0.5,ease:"expo.out"}},{T["split"]:.4f});
tl.fromTo("#a1-w",{{scale:1.32,opacity:0,filter:"blur(14px)"}},{{scale:1,opacity:1,filter:"blur(0px)",duration:0.13,ease:"power4.out"}},{T["nocam"]:.4f});
tl.to("#a1-w",{{opacity:0,y:-40,filter:"blur(10px)",duration:0.16,ease:"power3.in"}},{T["nocam_out"] - 0.13:.4f});
"""
# split frames hidden until the split
a1_js += f'tl.set("#a1-split",{{opacity:0}},0);tl.set("#a1-split",{{opacity:1}},{T["split"]:.4f});tl.set("#a1-split",{{opacity:0}},{T["nocam"]:.4f});tl.set("#a1-gate",{{opacity:1}},0);tl.set("#a1-gate",{{opacity:0}},{T["split"]:.4f});'
scenes.append(scene("a1", 0, a1_end, a1_body, a1_js))

# SFX act 1
cue(0.0, "projector", "loop", -24, until=T["nocam"])
cue(T["wipe"], "whoosh", "swipe/paper", -16)
cue(T["split"], "whoosh", "swish/soft", -15)
cue(T["nocam"], "impact", "impact/deep", -6)
cue(T["nocam"], "hit", "cut_hit/punch", -9)

# =========================== ACT 2: colour floods in 11.1 -> 19.1 =======================
S = T["a2"]
L = lambda t: t - S
a2_dur = T["cut3"] - S
p1 = "recreate this."
A2 = "a2p" if PUBLIC else "a2"
tjs, ttimes = typing_js("a2t", len(p1), L(T["type1"]))
a2_body = f"""
<div class="full" style="background:{BG};"></div>
<div id="a2-refw" class="full">
  {video("a2-refbw", A2 + "_ref_bw", 0, a2_dur, 0)}
  <div id="a2-flood" class="full">{video("a2-ref", A2 + "_ref", 0, a2_dur, 0)}</div>
</div>
<div id="a2-hkw" class="full">{video("a2-hk", A2 + "_hk", L(T["split2"]), T["cut3"] - T["split2"], L(T["split2"]))}</div>
<div id="a2-div" style="position:absolute;left:{0 if PUBLIC else 958}px;top:0;width:4px;height:1080px;background:{ACC};"></div>
{clip("a2-l1", L(T["split2"]) + 0.25, T["cut3"] - T["split2"] - 0.25, '<span>video</span>', cls="lbl", style="left:48px;top:44px;")}
{clip("a2-l2", L(T["split2"]) + 0.25, T["cut3"] - T["split2"] - 0.25, '<i></i><span>Handkit</span>', cls="lbl", style=("right:48px;" if PUBLIC else "left:1008px;") + "top:44px;")}
{clip("a2-pr", L(T["type1"]) - 0.25, T["split2"] - T["type1"] + 0.25,
      f'<div id="a2-pill" class="pill" style="left:0;top:0;position:relative;"><span class="dot"></span><span class="txt">{chars("a2t", p1)}</span><span id="a2-car" class="car"></span><span id="a2-go" class="go"><b></b></span></div>',
      cls="full", style="display:flex;align-items:flex-end;justify-content:center;padding-bottom:96px;box-sizing:border-box;")}
"""
a2_js = f"""
tl.fromTo("#a2-refw",{{opacity:0.32}},{{opacity:0.32,duration:0.01}},0);
tl.to("#a2-refw",{{opacity:1,duration:0.35,ease:"power2.out"}},{L(T["nocam_out"]) - 0.1:.4f});
tl.fromTo("#a2-flood",{{clipPath:"circle(0% at 50% 50%)"}},{{clipPath:"circle(80% at 50% 50%)",duration:1.5,ease:"power2.inOut"}},{L(T["flood"]):.4f});
tl.fromTo("#a2-pill",{{y:40,opacity:0,scale:0.96}},{{y:0,opacity:1,scale:1,duration:0.3,ease:"expo.out"}},{L(T["type1"]) - 0.25:.4f});
{tjs}
{caret_js("#a2-car", L(T["type1"]), L(T["split2"]))}
tl.fromTo("#a2-go",{{backgroundColor:"rgba(255,255,255,0.12)",scale:1}},{{backgroundColor:"{ACC}",scale:1.12,duration:0.08,ease:"power2.out"}},{L(T["send1"]):.4f});
tl.to("#a2-go",{{scale:1,duration:0.25,ease:"power3.out"}},{L(T["send1"]) + 0.08:.4f});
tl.to("#a2-pill",{{y:30,opacity:0,duration:0.2,ease:"power3.in"}},{L(T["split2"]) - 0.2:.4f});
""" + (f"""
tl.fromTo("#a2-hkw",{{clipPath:"inset(0px 0px 0px 1920px)"}},{{clipPath:"inset(0px 0px 0px 960px)",duration:0.55,ease:"expo.inOut"}},{L(T["split2"]) - 0.12:.4f});
tl.fromTo("#a2-div",{{x:1920}},{{x:958,duration:0.55,ease:"expo.inOut"}},{L(T["split2"]) - 0.12:.4f});
tl.to("#a2-hkw",{{clipPath:"inset(0px 0px 0px 700px)",duration:1.6,ease:"power2.inOut"}},{L(T["split2"]) + 0.6:.4f});
tl.to("#a2-div",{{x:698,duration:1.6,ease:"power2.inOut"}},{L(T["split2"]) + 0.6:.4f});
tl.to("#a2-hkw",{{clipPath:"inset(0px 0px 0px 1180px)",duration:1.6,ease:"power2.inOut"}},{L(T["split2"]) + 2.3:.4f});
tl.to("#a2-div",{{x:1178,duration:1.6,ease:"power2.inOut"}},{L(T["split2"]) + 2.3:.4f});
""" if PUBLIC else f"""tl.fromTo("#a2-refw",{{x:0,clipPath:"inset(0px 0px 0px 0px)"}},{{x:-480,clipPath:"inset(0px 480px 0px 480px)",duration:0.55,ease:"expo.inOut"}},{L(T["split2"]) - 0.12:.4f});
tl.fromTo("#a2-hkw",{{x:1440,clipPath:"inset(0px 480px 0px 480px)"}},{{x:480,clipPath:"inset(0px 480px 0px 480px)",duration:0.55,ease:"expo.inOut"}},{L(T["split2"]) - 0.12:.4f});
tl.fromTo("#a2-div",{{scaleY:0,opacity:1}},{{scaleY:1,duration:0.45,ease:"expo.out"}},{L(T["split2"]) + 0.2:.4f});
""") + f"""
tl.fromTo(["#a2-l1 span","#a2-l2 span","#a2-l2 i"],{{opacity:0,filter:"blur(6px)"}},{{opacity:1,filter:"blur(0px)",duration:0.15,stagger:0.07}},{L(T["split2"]) + 0.25:.4f});
"""
scenes.append(scene("a2", S, a2_dur, a2_body, a2_js))
cue(T["flood"], "swell", "reverse_swell/breath", -18)
for i, t in enumerate(ttimes):
    cue(S + t, "key", ["typing_key/light", "typing_key/laptop", "typing_key/crisp"][i % 3], -17, seed=i)
cue(T["send1"], "click", "pro_click/soft", -12)
cue(T["split2"] - 0.12, "whoosh", "whoosh_fast/sleeve", -14)

# =========================== ACT 3: Creation of Adam 19.1 -> 25.1 ======================
S = T["cut3"]
L = lambda t: t - S
a3_dur = T["black"] - S
p2a, p2b = "recreate this.", " spider-verse style."
tjs2, ttimes2 = typing_js("a3t", len(p2b), L(T["type2"]))
pw, ph, py = 562, 1000, 40
cx = (W - pw) // 2
lx, rx = (W - 2 * pw - 56) // 2, (W - 2 * pw - 56) // 2 + pw + 56
punch = [("p1", "a3_ref", 1.0, "50% 45%"), ("p2", "a3_hk", 1.0, "50% 45%"), ("p3", "a3_ref", 1.45, "52% 45%"), ("p4", "a3_hk", 1.45, "52% 45%")]
pun_html, pun_js = "", ""
for k, (key, src, sc, op) in enumerate(punch):
    t0 = T[key]
    t1 = T["black"] if k == 3 else T[punch[k + 1][0]]
    pun_html += f'<div id="a3-{key}w" class="full" style="overflow:hidden;">{video("a3-"+key, src, L(t0), t1 - t0, L(t0), style="object-position:"+op+";")}</div>'
    pun_js += f'tl.fromTo("#a3-{key}w",{{scale:{sc * 1.06:.3f}}},{{scale:{sc:.3f},duration:{t1 - t0:.3f},ease:"power2.out"}},{L(t0):.4f});'
a3_body = f"""
<div class="full" style="background:{BG};"></div>
<div id="a3-pr" class="wrap" style="left:{cx}px;top:{py}px;width:{pw}px;height:{ph}px;border-radius:26px;">
  {video("a3-ref", "a3_ref", 0, L(T["p1"]), 0)}
  {clip("a3-l1", 0.3, L(T["p1"]) - 0.3, '<span>video</span>', cls="lbl", style="left:30px;top:28px;")}
</div>
<div id="a3-ph" class="wrap" style="left:{rx}px;top:{py}px;width:{pw}px;height:{ph}px;border-radius:26px;">
  {video("a3-hk", "a3_hk", L(T["adam_hk"]), T["p1"] - T["adam_hk"], L(T["adam_hk"]))}
  {clip("a3-l2", L(T["adam_hk"]), T["p1"] - T["adam_hk"], '<i></i><span>Handkit</span>', cls="lbl", style="left:30px;top:28px;")}
</div>
{clip("a3-prm", 0.0, L(T["adam_hk"]) + 0.6,
      f'<div id="a3-pill" class="pill" style="position:relative;"><span class="dot"></span><span class="txt">{p2a}{chars("a3t", p2b)}</span><span id="a3-car" class="car"></span><span id="a3-go" class="go"><b></b></span></div>',
      cls="full", style="display:flex;align-items:flex-end;justify-content:center;padding-bottom:110px;box-sizing:border-box;")}
{pun_html}
"""
a3_js = f"""
tl.fromTo("#a3-pr",{{x:0,scale:1.04}},{{x:0,scale:1,duration:0.6,ease:"expo.out"}},0);
tl.fromTo("#a3-pill",{{y:30,opacity:0}},{{y:0,opacity:1,duration:0.3,ease:"expo.out"}},0.12);
{tjs2}
{caret_js("#a3-car", 0.12, L(T["adam_hk"]) + 0.6)}
tl.fromTo("#a3-go",{{backgroundColor:"rgba(255,255,255,0.12)",scale:1}},{{backgroundColor:"{ACC}",scale:1.12,duration:0.08}},{L(T["send2"]):.4f});
tl.to("#a3-go",{{scale:1,duration:0.25,ease:"power3.out"}},{L(T["send2"]) + 0.08:.4f});
tl.to("#a3-pr",{{x:{lx - cx},duration:0.6,ease:"expo.inOut"}},{L(T["adam_hk"]) - 0.15:.4f});
tl.fromTo("#a3-ph",{{x:{W - rx + 40}}},{{x:0,duration:0.6,ease:"expo.inOut"}},{L(T["adam_hk"]) - 0.15:.4f});
tl.to("#a3-pill",{{y:40,opacity:0,duration:0.2,ease:"power3.in"}},{L(T["adam_hk"]) + 0.35:.4f});
{pun_js}
"""
scenes.append(scene("a3", S, a3_dur, a3_body, a3_js))
cue(T["cut3"], "hit", "cut_hit/kick", -9)
cue(T["cut3"], "whoosh", "whoosh_fast/tight", -15)
for i, t in enumerate(ttimes2):
    cue(S + t, "key", ["typing_key/light", "typing_key/laptop", "typing_key/crisp"][i % 3], -17, seed=20 + i)
cue(T["send2"], "click", "pro_click/soft", -12)
cue(T["adam_hk"] - 0.15, "whoosh", "whoosh_fast/whip", -14)
for k, key in enumerate(["p1", "p2", "p3", "p4"]):
    cue(T[key], "whoosh", ["swish/blade", "swish/flick", "swish/zing", "swish/stick"][k], -15 + k)

# =========================== ACT 4: ANY ... drop 25.1 -> 43.067 =========================
S = T["black"]
L = lambda t: t - S
a4_dur = T["brk"] - S
looks = [("spider_medium", T["drop"], T["d2"]), ("arcane_brown", T["d2"], T["d3"]), ("toon_light", T["d3"], T["d4"]), ("clay_tan", T["d4"], T["piano"])]
lk_html = "".join(video(f"a4-lk{i}", s, L(a), b - a, a - T["drop"]) for i, (s, a, b) in enumerate(looks))
trip = ""
for i in range(3):
    x = 50 + i * 615
    trip += f'<div id="a4-tq{i}" class="wrap" style="left:{x}px;top:245px;width:590px;height:590px;border-radius:24px;">{video(f"a4-tv{i}", f"scroll_{i}", L(T["trip"]), T["skins"] - T["trip"], 0.0)}</div>'
bands = [(0, 570), (570, 780), (780, 990), (990, 1200), (1200, 1920)]
skins = ["light", "medium", "tan", "brown", "dark"]
band_html = video("a4-sk0", "skin_light", L(T["skins"]), T["d9"] - T["skins"], 0.0)
band_js = ""
for i in range(1, 5):
    a, b = bands[i]
    band_html += f'<div id="a4-b{i}" class="full">{video(f"a4-sk{i}", "skin_" + skins[i], L(T["skins"]), T["d9"] - T["skins"], 0.0)}</div>'
    band_html += f'<div id="a4-bl{i}" style="position:absolute;left:{a - 1}px;top:0;width:2px;height:1080px;background:rgba(255,255,255,.18);"></div>'
    tb = T[["skins", "sk2", "sk3", "sk4", "sk5"][i]]
    band_js += f'tl.fromTo("#a4-b{i}",{{clipPath:"inset(0px {W - b}px 1080px {a}px)"}},{{clipPath:"inset(0px {W - b}px 0px {a}px)",duration:0.32,ease:"expo.out"}},{L(tb):.4f});'
    band_js += f'tl.fromTo("#a4-bl{i}",{{scaleY:0,transformOrigin:"50% 0%"}},{{scaleY:1,duration:0.32,ease:"expo.out"}},{L(tb):.4f});'
fin = [("real_dark", T["d9"], T["d10"], 1.0), ("arcane_brown", T["d10"], T["d11"], 2.95), ("piano", T["d11"], T["brk"], 3.6)]
fin_html = "".join(video(f"a4-f{i}", s, L(a), b - a, mo) for i, (s, a, b, mo) in enumerate(fin))
a4_body = f"""
<div class="full" style="background:#000;"></div>
<div id="a4-lw" class="full">{lk_html}</div>
<div id="a4-pnw" class="full">{video("a4-pn", "piano", L(T["piano"]), T["trip"] - T["piano"], 1.0)}</div>
{clip("a4-tb", L(T["trip"]), T["skins"] - T["trip"], "", cls="full", style=f"background:{BG};")}
{trip}
<div id="a4-bw" class="full">{band_html}</div>
{fin_html}
{clip("a4-pan", 0, L(T["d3"]) + 0.5, "", cls="full", style="background:#000;")}
{clip("a4-any", L(T["any_a"]), T["d3"] + 0.5 - T["any_a"], '<div id="a4-anyw" class="big" style="font-size:210px;position:absolute;left:0;top:0;"><span id="a4-A">A</span><span id="a4-N">N</span><span id="a4-Y">Y</span></div>', cls="full")}
{clip("a4-look", L(T["drop"]) - 1 / FPS, T["d3"] + 0.5 - T["drop"] + 1 / FPS, '<div style="position:absolute;left:92px;top:556px;overflow:hidden;padding:0 20px 18px 0;"><div id="a4-lookw" class="big" style="font-size:210px;">LOOK<span class="acc">.</span></div></div>', cls="full")}
{clip("a4-pan2", L(T["sk5"]), T["d9"] - T["sk5"], '<div style="position:absolute;left:70px;top:300px;" class="big"><div id="a4-s1" style="font-size:200px;">ANY</div><div style="overflow:hidden;padding:0 20px 18px 0;margin-top:10px;"><div id="a4-s2" style="font-size:200px;">SKIN<span class="acc">.</span></div></div></div>', cls="full", style="left:1160px;width:760px;background:#000;")}
"""
# ANY: centred on black, then rides to the panel position on the drop
any_w = 0.62 * 210 * 3
a4_js = f"""
tl.set("#a4-anyw",{{x:{(W - 400) / 2:.0f},y:{(H - 190) / 2:.0f}}},0);
tl.fromTo("#a4-A",{{opacity:0,scale:1.5,filter:"blur(10px)"}},{{opacity:1,scale:1,filter:"blur(0px)",duration:0.12,ease:"power4.out"}},{L(T["any_a"]):.4f});
tl.fromTo("#a4-N",{{opacity:0,scale:1.5,filter:"blur(10px)"}},{{opacity:1,scale:1,filter:"blur(0px)",duration:0.12,ease:"power4.out"}},{L(T["any_n"]):.4f});
tl.fromTo("#a4-Y",{{opacity:0,scale:1.5,filter:"blur(10px)"}},{{opacity:1,scale:1,filter:"blur(0px)",duration:0.12,ease:"power4.out"}},{L(T["any_y"]):.4f});
tl.to("#a4-anyw",{{x:92,y:340,duration:0.34,ease:"expo.out"}},{L(T["drop"]) - 1 / FPS:.4f});
tl.fromTo("#a4-pan",{{x:0}},{{x:-1160,duration:0.34,ease:"expo.out"}},{L(T["drop"]) - 1 / FPS:.4f});
tl.fromTo("#a4-lw",{{x:380}},{{x:380,duration:0.01}},0);
tl.fromTo("#a4-lookw",{{yPercent:105}},{{yPercent:0,duration:0.3,ease:"expo.out"}},{L(T["drop"]) - 1 / FPS:.4f});
tl.to("#a4-pan",{{x:-1920,duration:0.32,ease:"power3.in"}},{L(T["d3"]) - 0.2:.4f});
tl.to("#a4-anyw",{{x:-760,duration:0.32,ease:"power3.in"}},{L(T["d3"]) - 0.2:.4f});
tl.to("#a4-lookw",{{x:-760,duration:0.32,ease:"power3.in"}},{L(T["d3"]) - 0.2:.4f});
tl.to("#a4-lw",{{x:0,duration:0.32,ease:"power3.inOut"}},{L(T["d3"]) - 0.2:.4f});
tl.fromTo("#a4-pnw",{{scale:1.12}},{{scale:1.0,duration:{T["trip"] - T["piano"]:.3f},ease:"power2.out"}},{L(T["piano"]):.4f});
tl.fromTo(["#a4-tq0","#a4-tq1","#a4-tq2"],{{y:90,opacity:0}},{{y:0,opacity:1,duration:0.42,ease:"expo.out",stagger:0.07}},{L(T["trip"]):.4f});
{band_js}
tl.fromTo("#a4-bw",{{x:0}},{{x:-360,duration:0.45,ease:"expo.inOut"}},{L(T["sk5"]) + 0.25:.4f});
tl.fromTo("#a4-pan2",{{x:800}},{{x:0,duration:0.45,ease:"expo.inOut"}},{L(T["sk5"]) + 0.25:.4f});
tl.fromTo("#a4-s1",{{opacity:0,filter:"blur(10px)"}},{{opacity:1,filter:"blur(0px)",duration:0.14}},{L(T["sk5"]) + 0.55:.4f});
tl.fromTo("#a4-s2",{{yPercent:105}},{{yPercent:0,duration:0.3,ease:"expo.out"}},{L(T["sk5"]) + 0.62:.4f});
"""
scenes.append(scene("a4", S, a4_dur, a4_body, a4_js))
cue(T["black"], "riser", "riser/tension", -13, dur=T["drop"] - T["black"])
for k in ["any_a", "any_n", "any_y"]:
    cue(T[k], "tick", "fit/land.light/finger_wood", -12)
cue(T["drop"], "impact", "impact/punch", -7)
cue(T["drop"], "whoosh", "whoosh_big/thump", -12)
for k, sid in zip(["d2", "d4"], ["swish/flick", "swish/bright"]):
    cue(T[k], "whoosh", sid, -18)
cue(T["d3"] - 0.2, "whoosh", "whoosh_fast/tight", -15)
cue(T["piano"], "whoosh", "whoosh_fast/crisp", -15)
cue(T["trip"], "whoosh", "swipe/glide", -15)
for i, k in enumerate(["sk2", "sk3", "sk4", "sk5"]):
    cue(T[k], "tick", ["pro_tap/soft", "pro_tap/finger", "pro_tap/knock", "pro_tap/double"][i], -12)
cue(T["sk5"] + 0.25, "whoosh", "whoosh_fast/sleeve", -15)
cue(T["sk5"] + 0.62, "impact", "pro_impact/short", -9)
for k, sid in zip(["d9", "d10", "d11"], ["swish/blade", "swish/cane", "swipe/velvet"]):
    cue(T[k], "whoosh", sid, -16)

# =========================== ACT 5: sign language 43.07 -> 51.85 ========================
# sign.mp4 (showcase/sign-handkit/film.json): H A N D K I T land on the clip's 0.5 .. 6.5 s, a letter a
# second, each held still; I-LOVE-YOU lands at the caption's time and holds through the silence.
# After the first letter the picture slides right and shows how it is made: on the left the rig's
# own source (crates/handkit-core/src/rig.rs) and all 29 joint values of the frame, on the hand its
# skeleton. Values and joints are the clip's own (showcase/sign-handkit/joints.py), frame by frame.
S = T["brk"]
L = lambda t: t - S
a5_dur = T["hit"] - S
LETTERS = "HANDKIT"
land = [S + 0.5 + i for i in range(len(LETTERS))]
cap = "".join(f'<span id="a5-c{i}" style="display:inline-block;opacity:0;">{c}</span>' for i, c in enumerate(LETTERS))
with open(os.path.join(ROOT, "..", "..", "showcase", "sign-handkit", "joints.json")) as f:
    JOINTS = json.load(f)
PANEL, SLIDE = 860, 430     # the code panel's width, and how far the hand and its caption slide
REVEAL = 1.0                # local time the panel comes in, after the first letter has landed
JOINT_KEYS = dict(thumb=[["cmc_flex", "cmc_abd"], ["cmc_rot", "mcp_flex", "ip_flex"]])
JOINT_KEYS.update({d: [["mcp_abd", "mcp_roll"], ["mcp_flex", "pip_flex", "dip_flex"]] for d in ("index", "middle", "ring", "pinky")})
JOINT_KEYS["wrist"] = [["twist", "wrist_flex"], ["wrist_dev", "palm_cup"]]


def value_rows():
    """the frame's pose as JSON, a span for every joint value (draw() fills them in)"""
    out = ['<div>"right": {</div>']
    items = list(JOINT_KEYS.items())
    for n, (digit, rows) in enumerate(items):
        out.append(f'<div>&nbsp;&nbsp;"{digit}": {{</div>')
        for r, row in enumerate(rows):
            pairs = ", ".join(f'<i>"{k}":</i> <b id="a5-v-{digit}-{k}"></b>' for k in row)
            end = ("," if r < len(rows) - 1 else " }" + ("," if n < len(items) - 1 else ""))
            out.append(f'<div>&nbsp;&nbsp;&nbsp;&nbsp;{pairs}{end}</div>')
    return "".join(out) + "<div>}</div>"


RIG = ["m = m * rz(-f.fan + abd_sign * abd) * ry(roll);", "for i in 0..3 {", "    m = m * rx(flex[i]);",
       "    let next = m * tr(0.0, f.len[i] * k, 0.0);", "}"]
rig_html = "".join("<div>" + line.replace(" ", "&nbsp;") + "</div>" for line in RIG)
a5_css = """
#a5-panel{position:absolute;left:0;top:0;width:%dpx;height:1080px;background:#0B0B0D;box-sizing:border-box;padding:60px 0 0 44px;overflow:hidden;
 font-family:Consolas,'Cascadia Mono',monospace;font-size:22px;line-height:30px;color:rgba(244,242,238,.9);white-space:nowrap;}
#a5-panel .file{color:%s;margin-bottom:10px;}
#a5-panel .rig{margin-bottom:34px;}
#a5-panel i{font-style:normal;color:rgba(244,242,238,.5);}
#a5-panel b{font-weight:400;display:inline-block;width:5ch;text-align:right;}
#a5-panel .on{color:%s;}
""" % (PANEL, ACC, ACC)
a5_body = f"""
<div class="full" style="background:#000;"></div>
<div id="a5-slide" class="full">
  <div id="a5-sw" class="full">{video("a5-sign", "sign", 0, a5_dur, 0.0)}
    <svg id="a5-bones" class="full" viewBox="0 0 1920 1080" style="opacity:0;"></svg>
  </div>
</div>
<div class="full" style="background:linear-gradient(to bottom,rgba(0,0,0,0) 62%,rgba(0,0,0,.72) 100%);"></div>
<div class="full" style="display:flex;align-items:flex-end;justify-content:center;padding-bottom:86px;box-sizing:border-box;">
  <div id="a5-cap" class="big" style="font-size:132px;letter-spacing:0.14em;margin-right:-0.14em;">{cap}</div>
</div>
<div id="a5-panel">
  <div class="file">rig.rs</div>
  <div class="rig">{rig_html}</div>
  <div class="file">sign.json&nbsp;&nbsp;<i id="a5-frame"></i></div>
  {value_rows()}
</div>
{clip("a5-ily", L(T["ily"]) - 0.1, T["hit"] - T["ily"] + 0.1, '<div id="a5-t" class="big" style="font-size:112px;letter-spacing:-0.035em;">I love you<span class="acc">.</span></div>', cls="full", style="display:flex;align-items:flex-end;justify-content:center;padding-bottom:96px;box-sizing:border-box;")}
"""
# The frame's joints and values, set from the timeline's own time: a tweened setter runs on every
# seek, so the numbers and the skeleton always belong to the frame shown.
a5_js = f"""
const J = {json.dumps(JOINTS, separators=(",", ":"))};
const NS = "http://www.w3.org/2000/svg", svg = document.getElementById("a5-bones");
const lines = J.bones.map(() => {{ const l = document.createElementNS(NS, "line"); l.setAttribute("stroke", "{ACC}"); l.setAttribute("stroke-width", "3.5"); l.setAttribute("stroke-linecap", "round"); svg.appendChild(l); return l; }});
const dots = J.names.map(() => {{ const c = document.createElementNS(NS, "circle"); c.setAttribute("r", "7"); c.setAttribute("fill", "#0B0B0D"); c.setAttribute("stroke", "#F4F2EE"); c.setAttribute("stroke-width", "2.5"); svg.appendChild(c); return c; }});
const cells = [];
for (const d in J.values[0]) for (const k in J.values[0][d]) {{ const e = document.getElementById("a5-v-" + d + "-" + k); if (e) cells.push([e, d, k]); }}
const frameLabel = document.getElementById("a5-frame");
function draw(x) {{
  const f = Math.min(J.xy.length - 1, Math.round(x)), p = J.xy[f], now = J.values[f], last = J.values[Math.max(0, f - 1)];
  J.bones.forEach((b, i) => {{ const l = lines[i]; l.setAttribute("x1", p[b[0] * 2]); l.setAttribute("y1", p[b[0] * 2 + 1]); l.setAttribute("x2", p[b[1] * 2]); l.setAttribute("y2", p[b[1] * 2 + 1]); }});
  dots.forEach((c, i) => {{ c.setAttribute("cx", p[i * 2]); c.setAttribute("cy", p[i * 2 + 1]); }});
  cells.forEach(([e, d, k]) => {{ e.textContent = now[d][k].toFixed(1); e.classList.toggle("on", Math.abs(now[d][k] - last[d][k]) > 0.25); }});
  frameLabel.textContent = "frame " + String(f).padStart(3, "0") + " · 29 joint angles";
}}
let frameNow = 0;
const clock = {{ get f() {{ return frameNow; }}, set f(v) {{ frameNow = v; draw(v); }} }};
draw(0);
tl.fromTo("#a5-sw",{{scale:1.08,opacity:0}},{{scale:1.0,opacity:1,duration:0.7,ease:"power2.out"}},0);
tl.fromTo(clock,{{f:0}},{{f:{len(JOINTS["xy"]) - 1},duration:{(len(JOINTS["xy"]) - 1) / FPS:.4f},ease:"none"}},0);
tl.fromTo("#a5-panel",{{x:-{PANEL}}},{{x:0,duration:0.6,ease:"expo.inOut"}},{REVEAL});
tl.fromTo("#a5-slide",{{x:0}},{{x:{SLIDE},duration:0.6,ease:"expo.inOut"}},{REVEAL});
tl.fromTo("#a5-cap",{{x:0}},{{x:{SLIDE},duration:0.6,ease:"expo.inOut"}},{REVEAL});
tl.fromTo("#a5-bones",{{opacity:0}},{{opacity:1,duration:0.35,ease:"power1.out"}},{REVEAL + 0.5});
"""
for i, t in enumerate(land):
    a5_js += f'tl.fromTo("#a5-c{i}",{{opacity:0,y:26,color:"{ACC}"}},{{opacity:1,y:0,duration:0.18,ease:"expo.out"}},{L(t) - 2 / FPS:.4f});'
    if i:
        a5_js += f'tl.to("#a5-c{i - 1}",{{color:"{INK}",duration:0.15}},{L(t) - 2 / FPS:.4f});'
a5_js += f'tl.to("#a5-cap",{{opacity:0,y:-20,duration:0.22,ease:"power2.in"}},{L(T["ily"]) - 0.32:.4f});'
a5_js += f'tl.fromTo("#a5-t",{{opacity:0,x:{SLIDE},y:24,filter:"blur(6px)"}},{{opacity:1,x:{SLIDE},y:0,filter:"blur(0px)",duration:0.4,ease:"expo.out"}},{L(T["ily"]) - 0.1:.4f});'
scenes.append(scene("a5", S, a5_dur, a5_body, a5_js, a5_css))
cue(T["brk"], "drop", "downlifter/air_fall", -16)
cue(T["brk"] + REVEAL, "whoosh", "swish/blade", -18)
for i, t in enumerate(land):
    cue(t, "tick", ["pro_tap/glass", "pro_tap/soft"][i % 2], -15)
cue(T["ily"], "tick", "fit/land.light/finger_phone", -18)

# =========================== ACT 6: people 51.85 -> 68.8 ===============================
S = T["hit"]
L = lambda t: t - S
a6_dur = T["end"] - S
# rock paper scissors (showcase/rock-paper-scissors): the clip's pumps land on its 0.5 s beats, throws on 2.0 and 4.0 s
ride = [("rps", T["hit"], T["hold"], 0.0),
        ("holding_hands", T["hold"], T["four"], 0.5),
        ("hs_kid", T["kid"], T["ship"], 4.07 - (T["bump"] - T["kid"]))]
ride_html = "".join(video(f"a6-v{i}", s, L(a), b - a, mo) for i, (s, a, b, mo) in enumerate(ride))
# huddle showcase: the five-hand stack at 1.6x into the throw-up, whose arrival (source 6.45 s) lands on the
# 60.82 downbeat; then the open palms at normal speed
HUD_LIFT, HUD_RATE = 6.45, 1.6
ride_html += video("a6-hud1", "huddle", L(T["four"]), T["lift"] - T["four"], HUD_LIFT - HUD_RATE * (T["lift"] - T["four"]), rate=HUD_RATE)
ride_html += video("a6-hud2", "huddle", L(T["lift"]), T["kid"] - T["lift"], HUD_LIFT)
ship_mo = 2.5 - (T["ship_tap"] - T["ship"])
ew = [("fframes", T["eng"]), ("three.js", T["eng2"]), ("Remotion", T["eng3"]), ("HyperFrames", T["eng4"])]
eng_html = f"""<div style="position:absolute;left:0;top:150px;width:1920px;display:flex;flex-direction:column;align-items:center;gap:6px;" class="big">
<div style="font-size:124px;font-weight:760;letter-spacing:-0.035em;"><span id="a6-e0">fframes</span><span id="a6-d0" class="acc"> · </span><span id="a6-e1">three.js</span></div>
<div style="font-size:124px;font-weight:760;letter-spacing:-0.035em;"><span id="a6-e2">Remotion</span><span id="a6-d1" class="acc"> · </span><span id="a6-e3">HyperFrames</span></div></div>
<div id="a6-webw" class="wrap" style="left:{(W - 1500) // 2}px;top:600px;width:1500px;height:375px;border-radius:18px;">{video("a6-web", "web3", L(T["eng"]), T["end"] - T["eng"], 1.0)}</div>"""
word = lambda id_, txt, top=800: f'<div style="position:absolute;left:96px;top:{top}px;overflow:hidden;padding:0 30px 22px 0;"><div id="{id_}" class="big" style="font-size:150px;text-shadow:0 6px 40px rgba(0,0,0,.45);">{txt}</div></div>'
a6_body = f"""
<div class="full" style="background:#1b1d22;"></div>
{ride_html}
<div id="a6-sw" class="full">{video("a6-ship", "shipit", L(T["ship"]), T["eng"] - T["ship"], ship_mo)}</div>
{clip("a6-w1", 0, T["throw1"] - T["hit"] - 0.1, word("a6-w1t", 'two people<span class="acc">.</span>', top=34), cls="full")}
{clip("a6-w2", L(T["lift"]) - 1 / FPS, T["kid"] - T["lift"] - 0.05, word("a6-w2t", 'the whole team<span class="acc">.</span>'), cls="full")}
{clip("a6-eng", L(T["eng"]), T["end"] - T["eng"], eng_html, cls="full", style=f"background:{BG};")}
"""
a6_js = f"""
tl.fromTo("#a6-w1t",{{yPercent:110}},{{yPercent:0,duration:0.38,ease:"expo.out"}},0.0001);
tl.to("#a6-w1t",{{yPercent:110,duration:0.2,ease:"power3.in"}},{T["throw1"] - T["hit"] - 0.32:.4f});
tl.fromTo("#a6-w2t",{{yPercent:110}},{{yPercent:0,duration:0.3,ease:"expo.out"}},{L(T["lift"]) - 1 / FPS:.4f});
tl.to("#a6-w2t",{{yPercent:110,duration:0.18,ease:"power3.in"}},{L(T["kid"]) - 0.25:.4f});
tl.fromTo("#a6-sw",{{scale:1.55,transformOrigin:"50% 56%"}},{{scale:1.42,duration:{T["eng"] - T["ship"]:.3f},ease:"power2.out"}},{L(T["ship"]):.4f});
tl.fromTo("#a6-webw",{{y:60,opacity:0}},{{y:0,opacity:1,duration:0.5,ease:"expo.out"}},{L(T["eng"]) + 0.1:.4f});
"""
for i, (_, t) in enumerate(ew):
    a6_js += f'tl.fromTo("#a6-e{i}",{{opacity:0,filter:"blur(8px)"}},{{opacity:1,filter:"blur(0px)",duration:0.15,ease:"power2.out"}},{L(t):.4f});'
a6_js += f'tl.fromTo(["#a6-d0","#a6-d1"],{{opacity:0}},{{opacity:1,duration:0.1}},{L(T["eng3"]):.4f});'
scenes.append(scene("a6", S, a6_dur, a6_body, a6_js))
cue(T["hit"], "sub", "sub_drop/808", -6)
cue(T["hit"], "impact", "impact/classic", -9)
for k in ["rps1", "rps2", "rps3", "rps4", "rps5", "rps6"]:
    cue(T[k], "tick", "fit/land.light/finger_wood", -14)
cue(T["throw1"], "clap", "fit/hit/bamboo_tok", -11)
cue(T["throw2"], "hit", "cut_hit/punch", -11)
cue(T["throw2"] + 0.25, "pop", "pro_pop_in/round", -15)
cue(T["hold"], "whoosh", "swipe/cloth", -16)
cue(T["four"], "whoosh", "whoosh_fast/heavy", -15)
cue(T["lift"], "whoosh", "whoosh_big/thump", -13)
cue(T["lift"], "impact", "pro_impact/air", -9)
cue(T["kid"], "whoosh", "swipe/flick", -16)
cue(T["bump"], "hit", "cut_hit/punch", -14)
cue(T["ship"], "whoosh", "swish/soft", -16)
cue(T["ship_tap"], "click", "pro_click/mouse", -9)
cue(T["eng"], "whoosh", "whoosh_big/thump_airy", -15)
for i, (_, t) in enumerate(ew):
    cue(t, "pop", ["pro_pop_in/round", "pro_pop_in/airy", "pro_pop_in/deep", "pro_pop_in/knock"][i], -12)

# =========================== ACT 7: wordmark 68.8 -> 73.8 =============================
S = T["end"]
L = lambda t: t - S
a7_dur = DUR - S
REPO = "github.com/Ayushmaniar/3d-accurate-hands"
GH_MARK = ('<svg width="40" height="40" viewBox="0 0 16 16" fill="rgba(244,242,238,.7)"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 '
           '0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07'
           '-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92'
           '.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>')
mark = "".join(f'<span class="m" id="a7-m{i}" style="display:inline-block;">{c}</span>' for i, c in enumerate("HANDKIT"))
a7_body = f"""
<div class="full" style="background:{BG};"></div>
<div class="full" style="display:flex;flex-direction:column;align-items:center;justify-content:center;">
  <div style="overflow:hidden;padding:10px 24px 8px;"><div id="a7-mk" class="big" style="font-size:262px;font-weight:840;letter-spacing:-0.05em;">{mark}</div></div>
  <div id="a7-bar" style="width:1010px;height:8px;background:{ACC};border-radius:4px;margin-top:26px;"></div>
  <div id="a7-tag" style="margin-top:44px;font-size:76px;font-weight:600;letter-spacing:-0.02em;color:{INK};white-space:nowrap;">Hands, by <span class="acc">prompt.</span></div>
  <div id="a7-gh" style="margin-top:64px;font-size:40px;font-weight:500;letter-spacing:0.005em;color:rgba(244,242,238,.7);white-space:nowrap;display:flex;align-items:center;gap:16px;">{GH_MARK}<span>{REPO}</span></div>
</div>
{clip("a7-fade", L(T["fade"]), DUR - T["fade"], "", cls="full", style="background:#000;")}
"""
a7_js = f"""
tl.fromTo(".m",{{yPercent:112}},{{yPercent:0,duration:0.7,ease:"expo.out",stagger:0.045}},0.0001);
tl.fromTo("#a7-bar",{{scaleX:0,transformOrigin:"0% 50%"}},{{scaleX:1,duration:0.4,ease:"expo.out"}},{L(T["tag"]) - 1 / FPS:.4f});
tl.fromTo("#a7-tag",{{clipPath:"inset(0px 100% 0px 0px)",filter:"blur(6px)"}},{{clipPath:"inset(0px 0% 0px 0px)",filter:"blur(0px)",duration:0.3,ease:"expo.out"}},{L(T["tag"]) - 1 / FPS:.4f});
tl.fromTo("#a7-gh",{{opacity:0,y:18}},{{opacity:1,y:0,duration:0.5,ease:"expo.out"}},{L(T["tag"]) + 0.4:.4f});
tl.fromTo("#a7-fade",{{opacity:0}},{{opacity:1,duration:{DUR - T["fade"] - 0.15:.3f},ease:"power1.in"}},{L(T["fade"]):.4f});
"""
scenes.append(scene("a7", S, a7_dur, a7_body, a7_js))
cue(T["end"], "whoosh", "whoosh_big/swell", -14)
cue(T["tag"], "impact", "impact/deep", -8)

# =========================== root ======================================================
z = {"a1": 20, "a2": 10, "a3": 10, "a4": 10, "av": 10, "a5": 10, "a6": 10, "a7": 10}
hosts = "\n".join(
    f'<div id="{s["id"]}" data-composition-id="{s["id"]}" data-composition-src="compositions/{s["id"]}.html" data-start="{s["start"]:.4f}" '
    f'data-duration="{s["dur"]:.4f}" data-track-index="{i + 1}" data-track-kind="graphics" data-width="{W}" data-height="{H}" style="z-index:{z[s["id"]]};"></div>'
    for i, s in enumerate(scenes))
index = f"""<!doctype html>
<html>
<head><meta charset="utf-8"><title>Handkit launch</title><script src="vendor/gsap.min.js"></script>
<style>{FONT} html,body{{margin:0;background:#000;}} #root{{background:#000;}}</style>
</head>
<body>
<div id="root" data-composition-id="main" data-start="0" data-duration="{DUR}" data-width="{W}" data-height="{H}">
<div class="full" style="position:absolute;inset:0;background:#000;"></div>
{hosts}
<audio id="mix" class="clip" src="assets/audio/mix.wav" data-start="0" data-duration="{DUR}" data-track-index="20" data-volume="1"></audio>
</div>
<script>window.__timelines = window.__timelines || {{}}; window.__timelines["main"] = gsap.timeline({{ paused: true }});</script>
</body>
</html>
"""
with open(os.path.join(OUT, "index.html"), "w", encoding="utf-8") as f:
    f.write(index)
os.makedirs(os.path.join(ROOT, "work"), exist_ok=True)
with open(os.path.join(ROOT, "work", "cues_public.json" if PUBLIC else "cues.json"), "w") as f:
    json.dump(dict(T=T, duration=DUR, cues=sorted(CUES, key=lambda c: c["t"])), f, indent=1)
print("scenes", [(s["id"], round(s["start"], 3), round(s["dur"], 3)) for s in scenes], "cues", len(CUES))
