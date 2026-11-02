# ---- poses (R = handkit right = screen-left; L = handkit left = screen-right) ----
EBR = ("edge", -0.07, 0.24); EBL = ("edge", 1.07, 0.24)
WR_F = at_z(0.37, 0.44, 0.05); PR_F = np.array([0, 0.9, 0.45]); PR_F = PR_F / np.linalg.norm(PR_F)
FR_F = np.array([1, -0.05, 0.1]); FL_F = np.array([-1, -0.12, 0.12])

def R_side():  return hand("right", at_z(0.20, 0.55, 0.06), [1, 0, 0.3], [0.15, -1, 0.25], "relaxed")
def R_lap1():  return hand("right", on_lap(0.33, 0.40), INTO_LAP, lapdir(0.2), "spread")
def R_out():   return hand("right", at_z(0.19, 0.52, 0.08), [0.3, 0.1, 1], [-0.25, -1, 0.2], "spread")
def R_under(): return hand("right", on_lap(0.27, 0.42, 0.03), [0.5, -0.3, -0.8], [0.45, -0.9, 0.2], "relaxed")
def R_lap2():  return hand("right", on_lap(0.31, 0.38), INTO_LAP, lapdir(0.05), "flat")
def R_flick(): return hand("right", at_z(0.19, 0.45, 0.10), [0.2, 0.4, 0.9], [-0.15, -1, 0.2], "spread")
def R_ontop(): return hand("right", on_lap(0.31, 0.33, 0.07), INTO_LAP, lapdir(0.2), "relaxed")
def R_lap3():  return hand("right", on_lap(0.29, 0.40), INTO_LAP, lapdir(0.05), "flat")
def R_out2():  return hand("right", at_z(0.13, 0.40, 0.10), [0.3, 0.3, 0.9], [-0.1, -1, 0.3], "relaxed", elbow=("edge", -0.03, 0.05))
def R_bellyU(): return hand("right", WR_F, PR_F, FR_F, "wrap", elbow=EBR)
def R_slide(): return hand("right", at_z(0.22, 0.36, 0.02), INTO_LAP, [0.3, -1, 0.4], "relaxed", elbow=("edge", -0.08, 0.12))
def R_lap4():  return hand("right", on_lap(0.27, 0.22), INTO_LAP, lapdir(0.1), "flat")
def R_lap5():  return hand("right", on_lap(0.38, 0.34), INTO_LAP, lapdir(0.0), "flat")
def R_flick2(): return hand("right", at_z(0.25, 0.36, 0.1), [0.3, 0.6, 0.75], [-0.1, -0.9, 0.4], "loose", elbow=("edge", 0.02, -0.02))
def R_bellyD(): return hand("right", WR_F + np.array([-0.03, 0.01, -0.01]), [0, -0.1, -1], [1, -0.6, 0.2], "curled", elbow=EBR)

def L_side():  return hand("left", at_z(0.80, 0.56, 0.06), [-0.6, 0.1, 0.8], [-0.15, -1, 0.25], "loose")
def L_open():  return hand("left", at_z(0.79, 0.53, 0.07), [-0.3, 0.2, 1], [-0.1, -1, 0.3], "relaxed")
def L_fidg():  return hand("left", at_z(0.82, 0.55, 0.08), [-0.6, 0.3, 0.7], [-0.2, -1, 0.3], "curled")
def L_cross(): return hand("left", on_lap(0.43, 0.38, 0.05), INTO_LAP + np.array([-0.2, 0, 0]), [-0.35, -0.9, 0.3], "relaxed", elbow=("edge", 0.8, -0.08))
def L_over():  return hand("left", on_lap(0.38, 0.40, 0.10), INTO_LAP, [-0.45, -0.9, 0.3], "relaxed", elbow=("edge", 0.8, -0.08))
def L_lap():   return hand("left", on_lap(0.53, 0.36), INTO_LAP, lapdir(-0.2), "flat")
def L_lapL():  return hand("left", on_lap(0.43, 0.40), INTO_LAP, lapdir(-0.25), "flat", elbow=("edge", 0.8, -0.08))
def L_mid():   return hand("left", on_lap(0.57, 0.33), INTO_LAP, lapdir(-0.1), "relaxed")
def L_side2(): return hand("left", at_z(0.80, 0.34, 0.06), [-0.8, 0.1, 0.5], [0, -1, 0.2], "loose", elbow=("edge", 0.92, -0.08))
def L_side2b(): return hand("left", at_z(0.79, 0.37, 0.07), [-0.4, 0.2, 0.9], [-0.05, -1, 0.3], "curled", elbow=("edge", 0.92, -0.08))
def L_top():
    w = WR_F + 0.055 * (FR_F / np.linalg.norm(FR_F) - FL_F / np.linalg.norm(FL_F)) + 0.03 * PR_F
    return hand("left", w, -PR_F, FL_F, "wrap", elbow=EBL)
def L_top2():
    w = WR_F + np.array([0.13, 0.03, 0.05])
    return hand("left", w, [0, -0.3, -1], [-1, -0.25, 0.1], "wrap", elbow=EBL)
def L_lap2():  return hand("left", on_lap(0.63, 0.32), INTO_LAP, lapdir(-0.8), "flat")
def L_lap3():  return hand("left", on_lap(0.66, 0.27), INTO_LAP, lapdir(-0.4), "flat")
def L_lap4():  return hand("left", on_lap(0.62, 0.31), INTO_LAP, lapdir(-0.2), "flat")
def L_claw():  return hand("left", at_z(0.74, 0.33, 0.10), [-0.3, 0.5, 0.8], [-0.15, -1, 0.15], "claw", elbow=("edge", 0.98, -0.04))
def L_claw2(): return hand("left", at_z(0.73, 0.34, 0.10), [-0.4, 0.4, 0.8], [-0.2, -1, 0.2], "spread", elbow=("edge", 0.9, -0.08))

R_fold, L_fold = solve_pair("fold", R_bellyU(), L_top())
R_fold2, L_fold2 = solve_pair("fold2", R_bellyD(), L_top2())
R_und, L_ovr = solve_pair("under", R_under(), L_over())
R_l2, L_l = solve_pair("lap2", R_lap2(), L_lap())
R_top, L_lL = solve_pair("ontop", R_ontop(), L_lapL(), extra_vary=("right.place.position",))
R_l3, L_m = solve_pair("lap3", R_lap3(), L_mid())

def lifted(h, dv):
    h = json.loads(json.dumps(h)); w = np.array(h["place"]["wrist"]) + np.array(dv)
    h["place"]["wrist"] = R3(w); return h

def reshape(h, shape):
    h = json.loads(json.dumps(h))
    for kk, v in SH[shape].items(): h[kk] = json.loads(json.dumps(v))
    return h

# ---- timeline: (t, hand, pose, extra key fields) ----
EV = [
 (0.0, "right", R_side(), {}), (0.0, "left", L_side(), {}),
 (0.7, "left", L_open(), {"move": 0.5}),
 (1.1, "left", L_fidg(), {"move": 0.3}),
 (1.2, "right", R_lap1(), {"move": 0.28}),
 (1.5, "right", R_out(), {"move": 0.28}),
 (1.75, "left", lifted(L_cross(), [0, 0.01, 0.07]), {"move": 0.4}),
 (2.05, "right", R_und, {"move": 0.4}),
 (2.25, "left", L_ovr, {"move": 0.4}),
 (2.75, "left", lifted(L_ovr, [0.03, 0.0, 0.06]), {"move": 0.3, "pass": True}),
 (3.05, "right", R_l2, {"move": 0.4}),
 (3.0, "left", L_l, {"move": 0.25}),
 (3.35, "right", R_flick(), {"move": 0.2}),
 (3.5, "left", L_lL, {"move": 0.35}),
 (3.85, "right", R_top, {"move": 0.3}),
 (4.25, "left", L_m, {"move": 0.3}),
 (4.25, "right", R_l3, {"move": 0.3}),
 (4.5, "right", R_out2(), {"move": 0.2, "pass": True}),
 (4.8, "right", R_fold, {"move": 0.3}),
 (4.42, "left", lifted(L_m, [0.05, 0.02, 0.03]), {"move": 0.2, "pass": True}),
 (4.9, "left", L_side2(), {"move": 0.45}),
 (5.5, "left", L_side2b(), {"move": 0.4}),
 (5.95, "left", L_side2(), {"move": 0.4}),
 (6.25, "left", lifted(L_fold, [0.03, 0.02, 0.05]), {"move": 0.3, "pass": True}),
 (6.55, "left", L_fold, {"move": 0.3}),
 (7.85, "left", L_fold, {"move": 0.1}),
 (8.02, "left", lifted(L_fold, [0.02, 0.03, 0.08]), {"move": 0.2}),
 (7.95, "right", reshape(R_fold, "flat"), {"move": 0.15}),
 (8.25, "right", R_slide(), {"move": 0.25}),
 (8.5, "right", R_lap4(), {"move": 0.22}),
 (8.5, "left", L_lap2(), {"move": 0.35}),
 (8.8, "right", R_lap5(), {"move": 0.3}),
 (8.85, "left", L_lap3(), {"move": 0.25}),
 (9.05, "right", R_flick2(), {"move": 0.2}),
 (9.05, "left", L_lap4(), {"move": 0.2}),
 (9.45, "right", R_fold2, {"move": 0.3}),
 (9.42, "left", L_side2(), {"move": 0.35}),
 (10.0, "left", L_claw(), {"move": 0.45}),
 (10.4, "left", L_claw2(), {"move": 0.3}),
 (10.75, "left", L_claw(), {"move": 0.3}),
 (11.0, "left", lifted(L_fold2, [0.03, 0.02, 0.05]), {"move": 0.25, "pass": True}),
 (11.3, "left", L_fold2, {"move": 0.3}),
]

def build(ev):
    ev = sorted(ev, key=lambda e: e[0])
    keys = []
    for t, side, pose, ex in ev:
        if keys and abs(keys[-1]["t"] - t) < 1e-6 and keys[-1].get("_ex") == ex:
            keys[-1]["hands"][side] = pose; continue
        k = {"t": t, "_ex": ex, "hands": {side: pose}}; k.update(ex); keys.append(k)
    for k in keys: k.pop("_ex")
    return keys

KEYS = build(EV)
