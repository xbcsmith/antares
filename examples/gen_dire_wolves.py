"""
Create 2 new wolf variants:
- Dire Wolf (id: 13) - bigger, meaner, light grey
- Dire Wolf Leader (id: 14) - biggest, meanest, almost white
"""

import os
OUT = "/home/claude/wolf_variants"
os.makedirs(OUT, exist_ok=True)

# ─────────────────────────────────────────────────────────────────────────
# Core helpers
# ─────────────────────────────────────────────────────────────────────────
BOX_I = [0,1,2,2,3,0, 1,5,6,6,2,1, 7,6,5,5,4,7,
         4,0,3,3,7,4, 4,5,1,1,0,4, 3,2,6,6,7,3]

def c(r,g,b,a=1.0): return [r,g,b,a]
def fv(v): return f"({v[0]}, {v[1]}, {v[2]})"
def fc(c): return f"({c[0]}, {c[1]}, {c[2]}, {c[3]})"

def box_v(x0,y0,z0, x1,y1,z1):
    return [[x0,y0,z0],[x1,y0,z0],[x1,y1,z0],[x0,y1,z0],
            [x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1]]

def quad_v(v0,v1,v2,v3): return [v0,v1,v2,v3]
def quad_i(): return [0,1,2,0,2,3]
def tri_v(v0,v1,v2): return [v0,v1,v2]
def tri_i(): return [0,1,2]
def norm3(nx,ny,nz,n): return [[nx,ny,nz]]*n

def emit_mesh(name, verts, indices, color, normals=None):
    L = ["        ("]
    L.append(f'            name: Some("{name}"),')
    L.append(f'            vertices: [{", ".join(fv(v) for v in verts)}],')
    L.append(f'            indices: [{", ".join(str(i) for i in indices)}],')
    L.append('            normals: None,' if not normals else
             f'            normals: Some([{", ".join(fv(n) for n in normals)}]),')
    L.append('            uvs: None,')
    L.append(f'            color: {fc(color)},')
    L.append('            lod_levels: None,')
    L.append('            lod_distances: None,')
    L.append('            material: None,')
    L.append('            texture_path: None,')
    L.append('        ),')
    return '\n'.join(L)

def emit_transform():
    return ("        (\n            translation: (0.0, 0.0, 0.0),\n"
            "            rotation: (0.0, 0.0, 0.0),\n"
            "            scale: (1.0, 1.0, 1.0),\n        ),")

def write_creature(filename, creature_id, name, meshes, scale):
    mesh_str = "\n".join(meshes)
    transforms = "\n".join([emit_transform() for _ in meshes])
    body = (f"(\n    id: {creature_id},\n    name: \"{name}\",\n"
            f"    meshes: [\n{mesh_str}\n    ],\n"
            f"    mesh_transforms: [\n{transforms}\n    ],\n"
            f"    scale: {scale},\n    color_tint: None,\n)\n")
    path = os.path.join(OUT, filename)
    with open(path, "w") as f:
        f.write(body)
    print(f"  ✓ {filename} (id: {creature_id}, scale: {scale})")

def box_mesh(name, x0,y0,z0, x1,y1,z1, color):
    return emit_mesh(name, box_v(x0,y0,z0,x1,y1,z1), list(BOX_I), color)

def quad_mesh(name, v0,v1,v2,v3, color, nx=0,ny=0,nz=1):
    return emit_mesh(name, quad_v(v0,v1,v2,v3), quad_i(), color, norm3(nx,ny,nz,4))

def tri_mesh(name, v0,v1,v2, color, nx=0,ny=0,nz=1):
    return emit_mesh(name, tri_v(v0,v1,v2), tri_i(), color, norm3(nx,ny,nz,3))

# ─────────────────────────────────────────────────────────────────────────
# Color palettes for each variant
# ─────────────────────────────────────────────────────────────────────────

# Base wolf (dark grey-brown)
WOLF_FUR       = c(0.60, 0.56, 0.50)
WOLF_DARK      = c(0.26, 0.24, 0.20)
EYE_YELLOW     = c(0.88, 0.78, 0.10)

# Dire Wolf (light grey - bigger, meaner)
DIRE_FUR       = c(0.72, 0.70, 0.68)  # lighter grey
DIRE_DARK      = c(0.45, 0.43, 0.40)  # lighter dark accents
DIRE_EYE       = c(0.95, 0.60, 0.10)  # amber-red menacing eyes

# Dire Wolf Leader (almost white - biggest, meanest, alpha)
ALPHA_FUR      = c(0.92, 0.90, 0.88)  # almost white
ALPHA_DARK     = c(0.65, 0.63, 0.60)  # silver-grey accents
ALPHA_EYE      = c(1.00, 0.40, 0.05)  # bright red-orange glowing eyes
SCAR_COLOR     = c(0.75, 0.50, 0.45)  # battle scars

# ─────────────────────────────────────────────────────────────────────────
# Wolf mesh builder (parametric - scales and colors can vary)
# ─────────────────────────────────────────────────────────────────────────

def build_wolf_base(
    body_color, dark_color, eye_color,
    body_scale=1.0,        # overall size multiplier
    bulk_scale=1.0,        # width/thickness multiplier for "meanness"
    snout_length=1.0,      # elongated snout for aggression
    ear_height=1.0,        # ear size
    include_scars=False    # battle damage for leader
):
    """
    Parametric wolf builder - all dimensions scale.
    body_scale: overall size (1.0 = normal wolf)
    bulk_scale: thickness/width (1.0 = normal, 1.2 = bulkier)
    """
    bs = body_scale
    bk = bulk_scale
    sl = snout_length
    eh = ear_height

    meshes = [
        # Body (horizontal quadruped) - scaled
        box_mesh("body",
            -0.28*bk*bs, -0.05*bs, -0.55*bs,
             0.28*bk*bs,  0.38*bs,  0.22*bs,
            body_color),
        # Chest bulk
        box_mesh("chest",
            -0.22*bk*bs,  0.12*bs, -0.58*bs,
             0.22*bk*bs,  0.42*bs, -0.28*bs,
            body_color),
        # Head
        box_mesh("head",
            -0.18*bk*bs, -0.02*bs, -0.76*bs,
             0.18*bk*bs,  0.32*bs, -0.52*bs,
            body_color),
        # Snout (longer = meaner)
        box_mesh("snout",
            -0.10*bk*bs, -0.06*bs, -0.90*sl*bs,
             0.10*bk*bs,  0.16*bs, -0.74*sl*bs,
            dark_color),
        # Nostrils
        box_mesh("nostrils",
            -0.05*bk*bs, -0.04*bs, -0.905*sl*bs,
             0.05*bk*bs,  0.04*bs, -0.895*sl*bs,
            c(0.14, 0.10, 0.08)),
        # Ears (triangular, height varies)
        tri_mesh("left_ear",
            [-0.15*bk*bs,  0.30*bs, -0.64*bs],
            [-0.06*bk*bs,  0.30*bs, -0.64*bs],
            [-0.10*bk*bs,  (0.30 + 0.19*eh)*bs, -0.64*bs],
            dark_color, nx=0, ny=0, nz=-1),
        tri_mesh("right_ear",
            [ 0.06*bk*bs,  0.30*bs, -0.64*bs],
            [ 0.15*bk*bs,  0.30*bs, -0.64*bs],
            [ 0.10*bk*bs,  (0.30 + 0.19*eh)*bs, -0.64*bs],
            dark_color, nx=0, ny=0, nz=-1),
        # Eyes
        quad_mesh("left_eye",
            [-0.14*bk*bs,  0.12*bs, -0.752*bs],
            [-0.08*bk*bs,  0.12*bs, -0.752*bs],
            [-0.14*bk*bs,  0.20*bs, -0.752*bs],
            [-0.08*bk*bs,  0.20*bs, -0.752*bs],
            eye_color, nx=0, ny=0, nz=-1),
        quad_mesh("right_eye",
            [ 0.08*bk*bs,  0.12*bs, -0.752*bs],
            [ 0.14*bk*bs,  0.12*bs, -0.752*bs],
            [ 0.08*bk*bs,  0.20*bs, -0.752*bs],
            [ 0.14*bk*bs,  0.20*bs, -0.752*bs],
            eye_color, nx=0, ny=0, nz=-1),
        # Legs (4, thicker for bulk)
        box_mesh("front_left",
            -0.28*bk*bs, -0.50*bs, -0.44*bs,
            -0.18*bk*bs, -0.06*bs, -0.34*bs,
            body_color),
        box_mesh("front_right",
             0.18*bk*bs, -0.50*bs, -0.44*bs,
             0.28*bk*bs, -0.06*bs, -0.34*bs,
            body_color),
        box_mesh("hind_left",
            -0.28*bk*bs, -0.50*bs,  0.06*bs,
            -0.18*bk*bs, -0.06*bs,  0.18*bs,
            body_color),
        box_mesh("hind_right",
             0.18*bk*bs, -0.50*bs,  0.06*bs,
             0.28*bk*bs, -0.06*bs,  0.18*bs,
            body_color),
        # Tail (bushier at tip)
        box_mesh("tail",
            -0.06*bk*bs, -0.02*bs,  0.22*bs,
             0.06*bk*bs,  0.28*bs,  0.54*bs,
            body_color),
        box_mesh("tail_tip",
            -0.04*bk*bs,  0.26*bs,  0.46*bs,
             0.04*bk*bs,  0.42*bs,  0.58*bs,
            dark_color),
    ]

    # Battle scars (leader only)
    if include_scars:
        meshes.extend([
            # Scar across left eye
            quad_mesh("scar_face",
                [-0.18*bk*bs,  0.08*bs, -0.751*bs],
                [-0.02*bk*bs,  0.24*bs, -0.751*bs],
                [-0.16*bk*bs,  0.10*bs, -0.751*bs],
                [-0.00*bk*bs,  0.26*bs, -0.751*bs],
                SCAR_COLOR, nx=0, ny=0, nz=-1),
            # Torn left ear notch (triangle bite mark)
            tri_mesh("ear_notch",
                [-0.12*bk*bs,  0.42*eh*bs, -0.645*bs],
                [-0.08*bk*bs,  0.42*eh*bs, -0.645*bs],
                [-0.10*bk*bs,  0.38*eh*bs, -0.645*bs],
                c(0.20, 0.16, 0.14), nx=0, ny=0, nz=-1),
            # Shoulder scar (jagged)
            quad_mesh("scar_shoulder",
                [-0.24*bk*bs,  0.28*bs,  0.02*bs],
                [-0.14*bk*bs,  0.32*bs,  0.02*bs],
                [-0.22*bk*bs,  0.30*bs,  0.02*bs],
                [-0.12*bk*bs,  0.34*bs,  0.02*bs],
                SCAR_COLOR, nx=0, ny=0, nz=1),
        ])

    # Spine ridge for dire variants (more aggressive silhouette)
    if body_scale > 1.0:
        meshes.extend([
            tri_mesh("spine_ridge_0",
                [-0.04*bk*bs,  0.38*bs, -0.20*bs],
                [ 0.04*bk*bs,  0.38*bs, -0.20*bs],
                [ 0.00*bk*bs,  0.50*bs, -0.22*bs],
                dark_color, ny=1),
            tri_mesh("spine_ridge_1",
                [-0.04*bk*bs,  0.38*bs,  0.00*bs],
                [ 0.04*bk*bs,  0.38*bs,  0.00*bs],
                [ 0.00*bk*bs,  0.48*bs, -0.02*bs],
                dark_color, ny=1),
        ])

    return meshes

# ─────────────────────────────────────────────────────────────────────────
# Generate variants
# ─────────────────────────────────────────────────────────────────────────

print("🐺 Generating Dire Wolf variants...")
print("=" * 60)

# Dire Wolf - 20% bigger, bulkier, light grey, longer snout
dire_meshes = build_wolf_base(
    body_color=DIRE_FUR,
    dark_color=DIRE_DARK,
    eye_color=DIRE_EYE,
    body_scale=1.2,      # 20% larger
    bulk_scale=1.15,     # 15% thicker/wider (menacing bulk)
    snout_length=1.1,    # 10% longer snout (aggressive)
    ear_height=1.0,      # normal ears
    include_scars=False
)
write_creature("direwolf.ron", 13, "DireWolf", dire_meshes, 1.1)

# Dire Wolf Leader - 40% bigger, very bulky, almost white, battle-scarred alpha
leader_meshes = build_wolf_base(
    body_color=ALPHA_FUR,
    dark_color=ALPHA_DARK,
    eye_color=ALPHA_EYE,
    body_scale=1.4,      # 40% larger (dominant)
    bulk_scale=1.3,      # 30% thicker (massive)
    snout_length=1.15,   # 15% longer (intimidating)
    ear_height=1.1,      # 10% taller ears (alert alpha)
    include_scars=True   # battle scars = veteran
)
write_creature("direwolf_leader.ron", 14, "DireWolfLeader", leader_meshes, 1.15)

print("\n✅ Complete!")
print("\nStats comparison:")
print("  Wolf         : scale 0.9  | grey-brown | yellow eyes")
print("  Dire Wolf    : scale 1.1  | light grey | amber-red eyes | +spine ridges")
print("  Dire Leader  : scale 1.15 | near-white | red eyes | +scars +ridges")
