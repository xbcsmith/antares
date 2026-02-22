"""
Generate procedural mesh RON files for the 11 missing monsters from monsters.ron.
Already covered: dying_goblin, skeleton_warrior, evil_lich, red_dragon.
New: goblin, kobold, giant_rat, orc, skeleton, wolf, ogre, zombie,
     fire_elemental, dragon, lich
"""

import os
OUT = "/home/claude/monsters_ron"
os.makedirs(OUT, exist_ok=True)

# ── geometry ──────────────────────────────────────────────────────────────
BOX_I = [0,1,2,2,3,0, 1,5,6,6,2,1, 7,6,5,5,4,7,
         4,0,3,3,7,4, 4,5,1,1,0,4, 3,2,6,6,7,3]

def box_v(x0,y0,z0, x1,y1,z1):
    return [
        [x0,y0,z0],[x1,y0,z0],[x1,y1,z0],[x0,y1,z0],
        [x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1],
    ]

def quad_v(v0,v1,v2,v3):
    return [v0,v1,v2,v3]

def quad_i():
    return [0,1,2,0,2,3]

def tri_v(v0,v1,v2):
    return [v0,v1,v2]

def tri_i():
    return [0,1,2]

def norm3(nx,ny,nz,n):
    return [[nx,ny,nz]]*n

# ── serialisation ─────────────────────────────────────────────────────────
def fv(v):  return f"[{v[0]:.3f}, {v[1]:.3f}, {v[2]:.3f}]"
def fc(c):  return f"Some([{c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f}])"

def emit(name, verts, indices, color, normals=None):
    L = [
        f'        MeshDefinition(',
        f'            name: "{name}",',
        f'            vertices: [',
    ]
    for v in verts:
        L.append(f'                {fv(v)},')
    L.append('            ],')
    if normals:
        L.append('            normals: Some([')
        for n in normals:
            L.append(f'                {fv(n)},')
        L.append('            ]),')
    else:
        L.append('            normals: None,')
    L.append('            uvs: None,')
    L.append('            indices: [')
    for i in range(0, len(indices), 12):
        L.append('                ' + ', '.join(str(x) for x in indices[i:i+12]) + ',')
    L.append('            ],')
    L.append(f'            color: {fc(color)},')
    L.append('        ),')
    return '\n'.join(L)

def box_mesh(name, x0,y0,z0, x1,y1,z1, color):
    return emit(name, box_v(x0,y0,z0,x1,y1,z1), list(BOX_I), color)

def quad_mesh(name, v0,v1,v2,v3, color, nx=0,ny=0,nz=1):
    return emit(name, quad_v(v0,v1,v2,v3), quad_i(), color, norm3(nx,ny,nz,4))

def tri_mesh(name, v0,v1,v2, color, nx=0,ny=0,nz=1):
    return emit(name, tri_v(v0,v1,v2), tri_i(), color, norm3(nx,ny,nz,3))

def write_ron(filename, name, scale, health, speed, note, meshes):
    body = (
        f"// {name}\n// {note}\n"
        f"CreatureDefinition(\n"
        f"    name: \"{name}\",\n"
        f"    scale: {scale:.2f},\n"
        f"    health: {health:.1f},\n"
        f"    speed: {speed:.1f},\n"
        f"    meshes: [\n"
        + "\n".join(meshes) +
        "\n    ],\n)\n"
    )
    path = os.path.join(OUT, filename)
    with open(path, "w") as f:
        f.write(body)
    print(f"  {filename}  ({len(meshes)} meshes)")

# ── colour palette ────────────────────────────────────────────────────────
def c(r,g,b,a=1.0): return [r,g,b,a]

SKIN_GOBLIN   = c(0.36,0.50,0.22)
SKIN_KOBOLD   = c(0.55,0.30,0.18)
SKIN_ORC      = c(0.30,0.46,0.20)
SKIN_OGRE     = c(0.50,0.45,0.30)
BONE          = c(0.88,0.86,0.78)
BONE_DIM      = c(0.80,0.78,0.68)
ZOMBIE_GREY   = c(0.46,0.48,0.40)
ZOMBIE_DARK   = c(0.30,0.32,0.26)
RAT_FUR       = c(0.42,0.36,0.28)
RAT_DARK      = c(0.28,0.22,0.16)
WOLF_FUR      = c(0.60,0.56,0.50)
WOLF_DARK     = c(0.26,0.24,0.20)
FUR_BROWN     = c(0.46,0.34,0.20)
IRON_RUST     = c(0.46,0.34,0.26)
IRON          = c(0.48,0.50,0.54)
LEATHER_DARK  = c(0.24,0.18,0.12)
RAGS          = c(0.42,0.38,0.28)
TUSK          = c(0.92,0.88,0.78)
HORN_DARK     = c(0.22,0.18,0.14)
WOOD          = c(0.30,0.20,0.12)
FIRE_BRIGHT   = c(1.00,0.88,0.20)
FIRE_MID      = c(1.00,0.45,0.08)
FIRE_DARK     = c(0.78,0.12,0.04)
FIRE_GLOW     = c(1.00,0.60,0.10,0.70)
LAVA          = c(0.22,0.14,0.10)
SCALE_RED     = c(0.60,0.10,0.08)
SCALE_DARK    = c(0.40,0.06,0.04)
SCALE_BELLY   = c(0.72,0.55,0.28)
WING          = c(0.48,0.06,0.04,0.82)
LICH_ROBE     = c(0.16,0.12,0.26)
LICH_TRIM     = c(0.38,0.28,0.58)
GOLD          = c(0.84,0.70,0.26)
EYE_RED       = c(0.90,0.12,0.08)
EYE_YELLOW    = c(0.88,0.78,0.10)
EYE_ORANGE    = c(0.95,0.48,0.08)
EYE_GREEN     = c(0.22,0.72,0.28)
EYE_WHITE     = c(0.95,0.95,0.90)
EYE_PURPLE    = c(0.62,0.26,0.90)
ORB_PURPLE    = c(0.60,0.25,0.90,0.88)
DISEASE       = c(0.28,0.55,0.18,0.55)
DRAIN         = c(0.50,0.18,0.72,0.60)

# ── reusable part emitters ────────────────────────────────────────────────

def biped_legs(color, sy=1.0):
    return [
        box_mesh("left_leg",  -0.15,-sy,-0.10, -0.05,0.00,0.05, color),
        box_mesh("right_leg",  0.05,-sy,-0.10,  0.15,0.00,0.05, color),
    ]

def biped_torso(color, w=0.27, h=0.80):
    return [box_mesh("torso", -w,0.00,-0.15, w,h,0.12, color)]

def biped_arms(color, reach=0.48):
    return [
        box_mesh("left_arm",  -reach,0.10,-0.10, -0.27,0.65,0.05, color),
        box_mesh("right_arm",  0.27, 0.10,-0.10,  reach,0.65,0.05, color),
    ]

def biped_head(color, w=0.17, h=0.30, yb=0.80):
    return [box_mesh("head", -w,yb,-0.17, w,yb+h,0.07, color)]

def eye_pair(lc, rc=None, y=1.00, z=0.071, ox=0.07, ew=0.04, eh=0.06):
    rc = rc or lc
    return [
        quad_mesh("left_eye",
            [-ox-ew,y,z],[-ox+ew,y,z],[-ox-ew,y+eh,z],[-ox+ew,y+eh,z], lc),
        quad_mesh("right_eye",
            [ ox-ew,y,z],[ ox+ew,y,z],[ ox-ew,y+eh,z],[ ox+ew,y+eh,z], rc),
    ]

def tusk_pair(color):
    return [
        tri_mesh("left_tusk",  [-0.06,0.82,0.07],[-0.02,0.82,0.07],[-0.04,0.70,0.11], color, ny=-1),
        tri_mesh("right_tusk",  [0.02,0.82,0.07],[ 0.06,0.82,0.07],[ 0.04,0.70,0.11], color, ny=-1),
    ]

def ear_pair_large(color):
    return [
        tri_mesh("left_ear",  [-0.17,1.08,-0.08],[-0.17,0.90,-0.08],[-0.38,0.99,-0.12], color, nx=-1),
        tri_mesh("right_ear",  [0.17,1.08,-0.08],[ 0.17,0.90,-0.08],[ 0.38,0.99,-0.12], color, nx=1),
    ]

def ear_pair_pointed(color):
    return [
        tri_mesh("left_ear",  [-0.17,1.02,-0.08],[-0.17,0.94,-0.08],[-0.28,0.98,-0.10], color, nx=-1),
        tri_mesh("right_ear",  [0.17,1.02,-0.08],[ 0.17,0.94,-0.08],[ 0.28,0.98,-0.10], color, nx=1),
    ]

def horn_pair(color, sx=0.13, by=1.10, ty=1.35, tz=-0.10):
    return [
        tri_mesh("left_horn",  [-sx-0.04,by,-0.08],[-sx+0.04,by,-0.08],[-sx,ty,tz], color, ny=1),
        tri_mesh("right_horn",  [sx-0.04,by,-0.08],[ sx+0.04,by,-0.08],[ sx,ty,tz], color, ny=1),
    ]

def brow_ridge(color, w=0.20):
    return [box_mesh("brow", -w,1.06,-0.05, w,1.10,0.08, color)]

def warpaint_stripe(color):
    return [quad_mesh("warpaint",
        [-0.18,0.98,0.071],[0.18,0.98,0.071],
        [-0.18,1.04,0.071],[0.18,1.04,0.071], color)]

def ribs(color):
    out = []
    for k, y in enumerate([0.18,0.34,0.50]):
        out.append(quad_mesh(f"rib_{k}",
            [-0.22,y,0.122],[0.22,y,0.122],
            [-0.22,y+0.06,0.122],[0.22,y+0.06,0.122], color))
    return out

def rot_spots(color):
    out = []
    spots = [(0.10,0.44,0.122),(-0.14,0.22,0.122),(0.04,0.64,0.122)]
    for k,(px,py,pz) in enumerate(spots):
        out.append(quad_mesh(f"rot_{k}",
            [px-0.06,py-0.04,pz],[px+0.06,py-0.04,pz],
            [px-0.06,py+0.04,pz],[px+0.06,py+0.04,pz],
            c(color[0],color[1],color[2],0.80)))
    return out

def aura_sparks(color, count=3):
    pts = [(-0.32,0.50,0.18),(0.28,0.66,0.18),(0.00,1.06,0.16),(0.18,0.28,0.19),(-0.08,0.78,0.20)]
    out = []
    for k in range(count):
        px,py,pz = pts[k]
        out.append(tri_mesh(f"aura_{k}",
            [px-0.08,py,pz],[px+0.08,py,pz],[px,py+0.16,pz],
            c(color[0],color[1],color[2],0.55)))
    return out

def sword_weapon(color):
    return [
        box_mesh("sword_blade",  0.52,-0.20,-0.06, 0.56,0.85,-0.02, color),
        box_mesh("sword_guard",  0.44,0.08,-0.08, 0.64,0.16,0.00, c(0.55,0.50,0.45)),
    ]

def dagger_weapon(color):
    return [box_mesh("dagger", 0.50,0.00,-0.05, 0.53,0.38,-0.01, color)]

def club_weapon(color):
    return [
        box_mesh("club_shaft", -0.54,-0.20,-0.07, -0.50,0.65,-0.03, WOOD),
        box_mesh("club_head",  -0.66,0.55,-0.11, -0.38,0.80,0.03, color),
    ]

def staff_weapon(shaft_c, orb_c):
    return [
        box_mesh("staff",     0.52,0.00,-0.06, 0.56,1.76,-0.02, shaft_c),
        box_mesh("staff_orb", 0.44,1.74,-0.10, 0.64,1.94,0.04, orb_c),
    ]

def bone_weapon():
    return [box_mesh("bone_weapon", -0.54,-0.18,-0.05, -0.50,0.55,-0.01, BONE)]

def fur_mantle(color):
    return [box_mesh("fur_mantle", -0.34,0.55,-0.18, 0.34,0.82,0.16, color)]

def loincloth(color):
    return [
        quad_mesh("loincloth_f",
            [-0.18,0.00,0.13],[0.18,0.00,0.13],
            [-0.14,-0.24,0.10],[0.14,-0.24,0.10], color),
        quad_mesh("loincloth_b",
            [-0.18,0.00,-0.14],[0.18,0.00,-0.14],
            [-0.14,-0.24,-0.11],[0.14,-0.24,-0.11], color, ny=0,nz=-1),
    ]

def crown_spikes(color):
    out = []
    for k, sx in enumerate([-0.14,-0.04,0.04,0.14]):
        out.append(tri_mesh(f"crown_{k}",
            [sx-0.03,1.18,-0.14],[sx+0.03,1.18,-0.14],[sx,1.40,-0.16],
            color, ny=1))
    return out


# ═══════════════════════════════════════════════════════════════════════════
# MONSTERS
# ═══════════════════════════════════════════════════════════════════════════

def build_goblin():
    """id:1 — Speed 8, HP 8, small, cowardly, rusty dagger."""
    m = (
        biped_legs(SKIN_GOBLIN, sy=0.60) +
        biped_torso(SKIN_GOBLIN, w=0.20, h=0.50) +
        ribs(c(0.28,0.42,0.16)) +
        biped_arms(SKIN_GOBLIN, reach=0.36) +
        biped_head(SKIN_GOBLIN, w=0.18, h=0.28, yb=0.52) +
        ear_pair_large(SKIN_GOBLIN) +
        eye_pair(EYE_YELLOW, y=0.72, ox=0.08) +
        tusk_pair(TUSK) +
        loincloth(c(0.36,0.28,0.18)) +
        dagger_weapon(IRON_RUST)
    )
    write_ron("goblin.ron","Goblin",0.72,40.0,6.5,
        "id:1 | Speed 8, HP 8 | Small cowardly fighter, big ears, rusty dagger", m)

def build_kobold():
    """id:2 — Speed 10, HP 5. Reptilian, scaly, fast, small tail."""
    m = (
        biped_legs(SKIN_KOBOLD, sy=0.62) +
        biped_torso(SKIN_KOBOLD, w=0.18, h=0.52) +
        [quad_mesh("scale_row_0",
            [-0.17,0.14,0.122],[0.17,0.14,0.122],[-0.17,0.20,0.122],[0.17,0.20,0.122],
            c(0.48,0.24,0.12,0.70)),
         quad_mesh("scale_row_1",
            [-0.17,0.32,0.122],[0.17,0.32,0.122],[-0.17,0.38,0.122],[0.17,0.38,0.122],
            c(0.48,0.24,0.12,0.70))] +
        biped_arms(SKIN_KOBOLD, reach=0.32) +
        # tail
        [box_mesh("tail_base", -0.04,-0.20,-0.20, 0.04,0.30,-0.14, SKIN_KOBOLD),
         box_mesh("tail_tip",  -0.03,-0.20,-0.36, 0.03,0.10,-0.20, c(0.45,0.24,0.14))] +
        biped_head(SKIN_KOBOLD, w=0.16, h=0.26, yb=0.54) +
        # snout
        [box_mesh("snout", -0.08,0.56,0.08, 0.08,0.70,0.20, c(0.48,0.26,0.14))] +
        horn_pair(HORN_DARK, sx=0.10, by=0.80, ty=0.98, tz=-0.08) +
        eye_pair(EYE_ORANGE, y=0.72, ox=0.07) +
        dagger_weapon(c(0.40,0.40,0.44))
    )
    write_ron("kobold.ron","Kobold",0.70,28.0,7.5,
        "id:2 | Speed 10, HP 5 | Reptilian, scaly, tail, orange eyes", m)

def build_giant_rat():
    """id:3 — Speed 12, Disease carrier. Quadruped, red beady eyes."""
    m = [
        box_mesh("body",        -0.22,-0.10,-0.45,  0.22,0.28, 0.18, RAT_FUR),
        box_mesh("head",        -0.14,-0.06,-0.62,  0.14,0.22,-0.42, RAT_FUR),
        box_mesh("snout",       -0.07,-0.08,-0.72,  0.07,0.10,-0.60, RAT_DARK),
        box_mesh("front_left",  -0.22,-0.36,-0.38, -0.14,-0.10,-0.28, RAT_FUR),
        box_mesh("front_right",  0.14,-0.36,-0.38,  0.22,-0.10,-0.28, RAT_FUR),
        box_mesh("hind_left",   -0.22,-0.36, 0.04, -0.13,-0.10, 0.16, RAT_FUR),
        box_mesh("hind_right",   0.13,-0.36, 0.04,  0.22,-0.10, 0.16, RAT_FUR),
        box_mesh("left_ear",    -0.14, 0.18,-0.60, -0.06,0.32,-0.56, c(0.52,0.36,0.30)),
        box_mesh("right_ear",    0.06, 0.18,-0.60,  0.14,0.32,-0.56, c(0.52,0.36,0.30)),
        # beady disease-red eyes
        quad_mesh("left_eye",
            [-0.11,-0.02,-0.625],[-0.07,-0.02,-0.625],
            [-0.11, 0.04,-0.625],[-0.07, 0.04,-0.625],
            EYE_RED, nx=0,ny=0,nz=-1),
        quad_mesh("right_eye",
            [ 0.07,-0.02,-0.625],[ 0.11,-0.02,-0.625],
            [ 0.07, 0.04,-0.625],[ 0.11, 0.04,-0.625],
            EYE_RED, nx=0,ny=0,nz=-1),
        box_mesh("tail",        -0.03,-0.05, 0.18,  0.03,0.08, 0.55, c(0.36,0.26,0.22)),
    ] + aura_sparks(DISEASE, count=2)
    write_ron("giant_rat.ron","GiantRat",0.65,20.0,8.0,
        "id:3 | Speed 12, Disease | Quadruped, red eyes, disease aura", m)

def build_orc():
    """id:10 — Might 12, HP 25. Heavy warrior, tusks, warpaint, crude club."""
    m = (
        biped_legs(LEATHER_DARK) +
        biped_torso(SKIN_ORC, w=0.30, h=0.82) +
        fur_mantle(FUR_BROWN) +
        [quad_mesh("chest_scar",
            [-0.04,0.42,0.122],[0.14,0.42,0.122],
            [-0.02,0.62,0.122],[0.12,0.62,0.122],
            c(0.22,0.36,0.12,0.70))] +
        biped_arms(SKIN_ORC, reach=0.52) +
        biped_head(SKIN_ORC, w=0.20, h=0.32) +
        brow_ridge(c(0.24,0.38,0.14)) +
        tusk_pair(TUSK) +
        warpaint_stripe(c(0.62,0.18,0.08)) +
        eye_pair(EYE_YELLOW, y=1.00, ox=0.09) +
        club_weapon(IRON_RUST)
    )
    write_ron("orc.ron","Orc",1.10,95.0,4.0,
        "id:10 | Might 12, HP 25 | Tusked warrior, warpaint, crude club", m)

def build_skeleton():
    """id:11 — Undead, HP 20, immune cold/paralysis/fear/sleep. Bone body, rusty sword."""
    m = [
        # thin bone legs
        box_mesh("left_leg",   -0.10,-1.00,-0.06, -0.04,0.00,0.02, BONE),
        box_mesh("right_leg",   0.04,-1.00,-0.06,  0.10,0.00,0.02, BONE),
        # narrow ribbed torso
        box_mesh("torso",      -0.22, 0.00,-0.10,  0.22,0.76,0.06, BONE),
    ] + ribs(BONE_DIM) + [
        box_mesh("left_arm",   -0.42, 0.10,-0.06, -0.22,0.62,0.02, BONE),
        box_mesh("right_arm",   0.22, 0.10,-0.06,  0.42,0.62,0.02, BONE),
        # skull
        box_mesh("skull",      -0.16, 0.78,-0.16,  0.16,1.06,0.05, BONE),
        # hollow eye sockets
        quad_mesh("left_socket",
            [-0.13,0.92,0.051],[-0.07,0.92,0.051],
            [-0.13,1.00,0.051],[-0.07,1.00,0.051],
            c(0.10,0.08,0.06)),
        quad_mesh("right_socket",
            [ 0.07,0.92,0.051],[ 0.13,0.92,0.051],
            [ 0.07,1.00,0.051],[ 0.13,1.00,0.051],
            c(0.10,0.08,0.06)),
        # glowing green eye lights
        quad_mesh("left_glow",
            [-0.12,0.93,0.055],[-0.08,0.93,0.055],
            [-0.12,0.99,0.055],[-0.08,0.99,0.055],
            EYE_GREEN),
        quad_mesh("right_glow",
            [ 0.08,0.93,0.055],[ 0.12,0.93,0.055],
            [ 0.08,0.99,0.055],[ 0.12,0.99,0.055],
            EYE_GREEN),
        # jaw gap
        quad_mesh("jaw_gap",
            [-0.10,0.80,0.051],[0.10,0.80,0.051],
            [-0.10,0.86,0.051],[0.10,0.86,0.051],
            c(0.10,0.08,0.06)),
    ] + sword_weapon(IRON_RUST)
    write_ron("skeleton.ron","Skeleton",1.00,60.0,3.5,
        "id:11 | Undead, HP 20, cold/fear immune | Bone body, glowing eyes, rusty sword", m)

def build_wolf():
    """id:12 — Speed 14, HP 18. Fast quadruped hunter, grey fur, amber eyes."""
    m = [
        box_mesh("body",         -0.28,-0.05,-0.55,  0.28,0.38,0.22, WOLF_FUR),
        box_mesh("chest",        -0.22, 0.12,-0.58,  0.22,0.42,-0.28, WOLF_FUR),
        box_mesh("head",         -0.18,-0.02,-0.76,  0.18,0.32,-0.52, WOLF_FUR),
        box_mesh("snout",        -0.10,-0.06,-0.90,  0.10,0.16,-0.74, WOLF_DARK),
        box_mesh("nostrils",     -0.05,-0.04,-0.905, 0.05,0.04,-0.895, c(0.14,0.10,0.08)),
        tri_mesh("left_ear",
            [-0.15,0.30,-0.64],[-0.06,0.30,-0.64],[-0.10,0.49,-0.64],
            WOLF_DARK, nx=0,ny=0,nz=-1),
        tri_mesh("right_ear",
            [ 0.06,0.30,-0.64],[ 0.15,0.30,-0.64],[ 0.10,0.49,-0.64],
            WOLF_DARK, nx=0,ny=0,nz=-1),
        quad_mesh("left_eye",
            [-0.14, 0.12,-0.752],[-0.08,0.12,-0.752],
            [-0.14, 0.20,-0.752],[-0.08,0.20,-0.752],
            EYE_YELLOW, nx=0,ny=0,nz=-1),
        quad_mesh("right_eye",
            [ 0.08, 0.12,-0.752],[ 0.14,0.12,-0.752],
            [ 0.08, 0.20,-0.752],[ 0.14,0.20,-0.752],
            EYE_YELLOW, nx=0,ny=0,nz=-1),
        box_mesh("front_left",   -0.28,-0.50,-0.44, -0.18,-0.06,-0.34, WOLF_FUR),
        box_mesh("front_right",   0.18,-0.50,-0.44,  0.28,-0.06,-0.34, WOLF_FUR),
        box_mesh("hind_left",    -0.28,-0.50, 0.06, -0.18,-0.06,0.18, WOLF_FUR),
        box_mesh("hind_right",    0.18,-0.50, 0.06,  0.28,-0.06,0.18, WOLF_FUR),
        box_mesh("tail",         -0.06,-0.02, 0.22,  0.06,0.28,0.54, WOLF_FUR),
        box_mesh("tail_tip",     -0.04, 0.26, 0.46,  0.04,0.42,0.58, WOLF_DARK),
    ]
    write_ron("wolf.ron","Wolf",0.90,55.0,8.5,
        "id:12 | Speed 14, HP 18 | Fast quadruped, grey fur, amber eyes, pack hunter", m)

def build_ogre():
    """id:20 — Might 18, HP 60, can_regenerate. Massive brute with huge club."""
    m = (
        biped_legs(c(0.42,0.38,0.26), sy=1.10) +
        biped_torso(SKIN_OGRE, w=0.40, h=0.90) +
        # pot belly
        [box_mesh("belly", -0.36,0.05,-0.12, 0.36,0.46,0.26, c(0.54,0.49,0.34))] +
        fur_mantle(c(0.38,0.28,0.16)) +
        # massive arms
        [box_mesh("left_arm",   -0.64,0.08,-0.14, -0.38,0.76,0.10, SKIN_OGRE),
         box_mesh("right_arm",   0.38,0.08,-0.14,  0.64,0.76,0.10, SKIN_OGRE),
         box_mesh("left_fist",  -0.68,-0.08,-0.12, -0.40,0.12,0.08, c(0.44,0.40,0.28)),
         box_mesh("right_fist",  0.40,-0.08,-0.12,  0.68,0.12,0.08, c(0.44,0.40,0.28))] +
        biped_head(SKIN_OGRE, w=0.26, h=0.36, yb=0.92) +
        [box_mesh("brow", -0.27,1.22,-0.08, 0.27,1.28,0.10, c(0.44,0.40,0.26))] +
        tusk_pair(TUSK) +
        eye_pair(EYE_RED, y=1.12, ox=0.12) +
        # massive club
        [box_mesh("club_shaft", -0.74,-0.30,-0.09, -0.64,0.74,-0.03, WOOD),
         box_mesh("club_knob",  -0.88,0.64,-0.15, -0.50,0.92,0.07, c(0.26,0.16,0.08))] +
        # regen glow
        aura_sparks(c(0.28,0.68,0.28), count=2)
    )
    write_ron("ogre.ron","Ogre",1.36,145.0,3.0,
        "id:20 | Might 18, HP 60, regenerates | Massive brute, pot belly, tusks, huge club", m)

def build_zombie():
    """id:21 — Undead, HP 35, Disease, Speed 4 (very slow)."""
    m = (
        biped_legs(ZOMBIE_GREY) +
        biped_torso(ZOMBIE_GREY, w=0.28, h=0.78) +
        rot_spots(c(0.18,0.20,0.14)) +
        # tattered rags hanging off torso
        [quad_mesh("rags_front",
            [-0.28,0.08,0.122],[0.28,0.08,0.122],
            [-0.32,-0.22,0.118],[0.32,-0.22,0.118],
            c(0.38,0.33,0.22,0.82))] +
        biped_arms(ZOMBIE_DARK, reach=0.50) +
        # one arm outstretched lower (shambling pose)
        [box_mesh("left_arm_low", -0.52,-0.10,-0.10, -0.26,0.22,0.06, ZOMBIE_DARK)] +
        biped_head(ZOMBIE_GREY, w=0.18, h=0.30) +
        eye_pair(c(0.82,0.58,0.08), y=0.98, ox=0.08) +
        # gaping jaw
        [box_mesh("jaw",        -0.12,0.80,-0.02, 0.12,0.86,0.08, c(0.28,0.26,0.20)),
         quad_mesh("open_mouth",
            [-0.10,0.80,0.082],[0.10,0.80,0.082],
            [-0.10,0.84,0.082],[0.10,0.84,0.082],
            c(0.12,0.08,0.08))] +
        aura_sparks(DISEASE, count=3)
    )
    write_ron("zombie.ron","Zombie",1.00,100.0,2.0,
        "id:21 | Undead, HP 35, Disease, Speed 4 | Shambling, rot spots, disease aura", m)

def build_fire_elemental():
    """id:22 — Fire+Physical immune, HP 70, magic_resistance 50."""
    m = [
        # lava rock base/feet
        box_mesh("base",       -0.22,-0.05,-0.20, 0.22,0.22,0.20, LAVA),
        # flame body — 4 stacking layers getting narrower toward top
        box_mesh("flame_0",    -0.28, 0.00,-0.24, 0.28,0.38,0.24, FIRE_MID),
        box_mesh("flame_1",    -0.24, 0.30,-0.20, 0.24,0.64,0.20, c(1.00,0.60,0.10)),
        box_mesh("flame_2",    -0.20, 0.56,-0.16, 0.20,0.88,0.16, c(1.00,0.78,0.14)),
        box_mesh("flame_3",    -0.16, 0.80,-0.12, 0.16,1.06,0.12, FIRE_BRIGHT),
        # bright inner core
        box_mesh("core",       -0.12, 0.10,-0.10, 0.12,0.96,0.10, FIRE_BRIGHT),
        # flame head column
        box_mesh("flame_head", -0.18, 0.94,-0.14, 0.18,1.22,0.14, c(1.00,0.82,0.18,0.92)),
        # arm tendrils of fire
        box_mesh("left_arm",   -0.52, 0.20,-0.12, -0.28,0.74,0.08, FIRE_MID),
        box_mesh("right_arm",   0.28, 0.20,-0.12,  0.52,0.74,0.08, FIRE_MID),
        # eyes: white-hot cores
        quad_mesh("left_eye",
            [-0.10,1.03,0.141],[-0.06,1.03,0.141],
            [-0.10,1.09,0.141],[-0.06,1.09,0.141],
            c(1.00,1.00,0.85)),
        quad_mesh("right_eye",
            [ 0.06,1.03,0.141],[ 0.10,1.03,0.141],
            [ 0.06,1.09,0.141],[ 0.10,1.09,0.141],
            c(1.00,1.00,0.85)),
        # flame tips above head
        tri_mesh("tip_0", [-0.20,1.22,0.0],[-0.08,1.22,0.0],[-0.14,1.46,0.0], FIRE_GLOW, ny=1),
        tri_mesh("tip_1", [-0.06,1.18,0.0],[ 0.06,1.18,0.0],[ 0.00,1.50,0.0], FIRE_GLOW, ny=1),
        tri_mesh("tip_2", [ 0.08,1.22,0.0],[ 0.20,1.22,0.0],[ 0.14,1.46,0.0], FIRE_GLOW, ny=1),
    ] + aura_sparks(FIRE_DARK, count=3)
    write_ron("fire_elemental.ron","FireElemental",1.10,160.0,6.0,
        "id:22 | Fire+Physical immune, HP 70, magic_res 50 | Pure flame creature", m)

def build_dragon():
    """id:30 — Might 25, HP 200, fire immune, 2 attacks, regenerates. Base combat dragon."""
    m = [
        # scaled body (horizontal, front-facing)
        box_mesh("body",          -0.38,-0.12,-0.82,  0.38,0.72,0.40, SCALE_RED),
        # lighter belly underside
        quad_mesh("belly",
            [-0.36,-0.11,-0.80],[0.36,-0.11,-0.80],
            [-0.36,-0.11, 0.38],[0.36,-0.11, 0.38],
            SCALE_BELLY, nx=0,ny=-1,nz=0),
        # neck
        box_mesh("neck",          -0.18, 0.52,-0.90,  0.18,0.96,-0.38, SCALE_RED),
        # head
        box_mesh("head",          -0.22, 0.76,-1.10,  0.22,1.12,-0.82, SCALE_RED),
        # snout
        box_mesh("snout",         -0.14, 0.76,-1.32,  0.14,0.96,-1.08, SCALE_DARK),
        # lower jaw
        box_mesh("jaw",           -0.18, 0.72,-1.12,  0.18,0.80,-0.84, SCALE_DARK),
        # fire glow inside mouth
        box_mesh("mouth_glow",    -0.14, 0.76,-1.10,  0.14,0.80,-0.86, c(1.00,0.70,0.10,0.80)),
        # eyes
        quad_mesh("left_eye",
            [-0.21,0.96,-0.852],[-0.13,0.96,-0.852],
            [-0.21,1.04,-0.852],[-0.13,1.04,-0.852],
            EYE_ORANGE, nx=0,ny=0,nz=-1),
        quad_mesh("right_eye",
            [ 0.13,0.96,-0.852],[ 0.21,0.96,-0.852],
            [ 0.13,1.04,-0.852],[ 0.21,1.04,-0.852],
            EYE_ORANGE, nx=0,ny=0,nz=-1),
        # head horns
        tri_mesh("left_horn",
            [-0.19,1.10,-0.91],[-0.11,1.10,-0.91],[-0.15,1.40,-0.99],
            HORN_DARK, ny=1),
        tri_mesh("right_horn",
            [ 0.11,1.10,-0.91],[ 0.19,1.10,-0.91],[ 0.15,1.40,-0.99],
            HORN_DARK, ny=1),
        # 4 legs
        box_mesh("front_left",    -0.52,-0.54,-0.64, -0.36,-0.10,-0.48, SCALE_RED),
        box_mesh("front_right",    0.36,-0.54,-0.64,  0.52,-0.10,-0.48, SCALE_RED),
        box_mesh("hind_left",     -0.52,-0.54, 0.14, -0.36,-0.10, 0.30, SCALE_RED),
        box_mesh("hind_right",     0.36,-0.54, 0.14,  0.52,-0.10, 0.30, SCALE_RED),
        # front claws
        tri_mesh("claw_fl",
            [-0.50,-0.72,-0.62],[-0.44,-0.72,-0.62],[-0.47,-0.88,-0.56],
            BONE, ny=-1,nz=0),
        tri_mesh("claw_fr",
            [ 0.44,-0.72,-0.62],[ 0.50,-0.72,-0.62],[ 0.47,-0.88,-0.56],
            BONE, ny=-1,nz=0),
        # tail
        box_mesh("tail_base",     -0.20,-0.10, 0.40,  0.20,0.44,0.72, SCALE_RED),
        box_mesh("tail_mid",      -0.14,-0.08, 0.72,  0.14,0.34,1.02, SCALE_DARK),
        box_mesh("tail_tip",      -0.06,-0.04, 1.02,  0.06,0.18,1.26, SCALE_DARK),
        # tail spikes
        tri_mesh("tail_spike_0",
            [-0.04,0.42,0.58],[0.04,0.42,0.58],[0.00,0.60,0.54],
            HORN_DARK, ny=1),
        tri_mesh("tail_spike_1",
            [-0.04,0.32,0.88],[0.04,0.32,0.88],[0.00,0.48,0.84],
            HORN_DARK, ny=1),
        # wing membranes
        quad_mesh("left_wing",
            [-0.38,0.56,-0.22],[-0.38,0.56, 0.34],
            [-1.30, 0.00,-0.26],[-1.30, 0.00, 0.40],
            WING, nx=-1,ny=0,nz=0),
        quad_mesh("right_wing",
            [ 0.38,0.56,-0.22],[ 0.38,0.56, 0.34],
            [ 1.30, 0.00,-0.26],[ 1.30, 0.00, 0.40],
            WING, nx=1,ny=0,nz=0),
    ] + aura_sparks(FIRE_MID, count=3)
    write_ron("dragon.ron","Dragon",2.20,310.0,5.5,
        "id:30 | Might 25, HP 200, fire immune, 2 attacks, regenerates | Red dragon", m)

def build_lich():
    """id:31 — Intellect 22, HP 150, undead, fire/cold/elec immune, Drain, regenerates."""
    m = (
        biped_legs(LICH_ROBE) +
        biped_torso(LICH_ROBE, w=0.32, h=0.88) +
        [quad_mesh("robe_trim_bot",
            [-0.32,0.02,0.122],[0.32,0.02,0.122],
            [-0.32,0.08,0.122],[0.32,0.08,0.122], LICH_TRIM),
         quad_mesh("robe_trim_top",
            [-0.32,0.80,0.122],[0.32,0.80,0.122],
            [-0.32,0.86,0.122],[0.32,0.86,0.122], LICH_TRIM)] +
        # wide flowing sleeves
        [box_mesh("left_sleeve",  -0.60,0.12,-0.14, -0.28,0.80,0.06, LICH_ROBE),
         box_mesh("right_sleeve",  0.28,0.12,-0.14,  0.60,0.80,0.06, LICH_ROBE),
         # skeletal hand ends
         box_mesh("left_hand",   -0.62,0.06,-0.08, -0.52,0.18,0.00, BONE),
         box_mesh("right_hand",   0.52,0.06,-0.08,  0.62,0.18,0.00, BONE)] +
        # skull
        [box_mesh("skull",       -0.17,0.90,-0.16, 0.17,1.18,0.05, BONE)] +
        # hollow eye sockets
        [quad_mesh("left_socket",
            [-0.14,0.98,0.052],[-0.08,0.98,0.052],
            [-0.14,1.06,0.052],[-0.08,1.06,0.052],
            c(0.08,0.06,0.12)),
         quad_mesh("right_socket",
            [ 0.08,0.98,0.052],[ 0.14,0.98,0.052],
            [ 0.08,1.06,0.052],[ 0.14,1.06,0.052],
            c(0.08,0.06,0.12))] +
        # glowing purple eyes (Drain attack)
        eye_pair(EYE_PURPLE, y=0.99, ox=0.11, ew=0.03, eh=0.06) +
        # crown band + spikes
        [box_mesh("crown_band", -0.19,1.17,-0.16, 0.19,1.22,0.06, GOLD)] +
        crown_spikes(GOLD) +
        # necromantic staff with drain orb
        staff_weapon(c(0.18,0.12,0.22), ORB_PURPLE) +
        # drain aura (purple), regen aura (faint green)
        aura_sparks(DRAIN, count=4) +
        aura_sparks(c(0.28,0.72,0.36,0.55), count=2)
    )
    write_ron("lich.ron","Lich",1.05,185.0,4.0,
        "id:31 | Intellect 22, HP 150, undead, fire/cold/elec immune, Drain | Necromancer lich", m)


# ── Run all ───────────────────────────────────────────────────────────────
print("Generating monster RON files...")
build_goblin()
build_kobold()
build_giant_rat()
build_orc()
build_skeleton()
build_wolf()
build_ogre()
build_zombie()
build_fire_elemental()
build_dragon()
build_lich()

files = sorted(os.listdir(OUT))
print(f"\nDone — {len(files)} files:")
for f in files:
    size = os.path.getsize(os.path.join(OUT, f))
    print(f"  {f}  ({size} bytes)")
