"""
Unified mesh generator
"""
import os, math
OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "campaigns", "tutorial", "assets", "creatures")
os.makedirs(OUT, exist_ok=True)

def get_id(filename):
    mapping = {'ancientskeleton.ron': 133, 'ancientwolf.ron': 122, 'apprenticezara.ron': 59, 'bandit.ron': 6, 'direwolf.ron': 4, 'direwolf_leader.ron': 14, 'direwolfleader.ron': 5, 'dragon.ron': 30, 'dwarfcleric.ron': 103, 'dyinggoblin.ron': 151, 'elfmage.ron': 102, 'evillich.ron': 153, 'fireelemental.ron': 22, 'giantrat.ron': 3, 'goblin.ron': 1, 'highpriest.ron': 54, 'highpriestess.ron': 55, 'humanfighter.ron': 101, 'innkeeper.ron': 52, 'kira.ron': 60, 'kobold.ron': 2, 'lich.ron': 31, 'merchant.ron': 53, 'mira.ron': 61, 'ogre.ron': 20, 'oldgareth.ron': 58, 'orc.ron': 10, 'pyramiddragon.ron': 33, 'ranger.ron': 57, 'reddragon.ron': 32, 'sirius.ron': 62, 'skeleton.ron': 11, 'skeletonwarrior.ron': 152, 'villageelder.ron': 51, 'whisper.ron': 63, 'wizardarcturus.ron': 56, 'wolf.ron': 12, 'zombie.ron': 21}

    # 1. Look up old mapping, apply scaling logic if it exists
    if filename in mapping:
        old_id = mapping[filename]
        # Shift according to new scale
        if 51 <= old_id <= 100:
            return old_id - 51 + 1000  # NPCs
        elif 101 <= old_id <= 150:
            return old_id - 101 + 2000 # Templates
        elif 151 <= old_id <= 200:
            return old_id - 151 + 3000 # Variants
        elif old_id >= 201:
            return old_id - 201 + 4000 # Custom
        return old_id # Monsters (1-50 -> 1-999)

    # 2. Dynamic generation for non-mapped meshes instead of 999
    global next_monster_id, next_npc_id, next_custom_id

    # Heuristics for category based on filename
    # Characters are NPCs or Custom. "male_" or "female_"
    if filename.startswith("male_") or filename.startswith("female_"):
        new_id = next_custom_id
        next_custom_id += 1
        return new_id
    elif any(name in filename for name in ["mira", "sirius", "whisper", "kira"]):
        new_id = next_npc_id
        next_npc_id += 1
        return new_id
    else:  # Assume default is monster
        new_id = next_monster_id
        next_monster_id += 1
        return new_id

next_monster_id = 51
next_npc_id = 1050
next_custom_id = 4000
"""
ENHANCED MESH GENERATOR
=======================
Creates highly detailed, recognizable procedural meshes with:
- Segmented curved surfaces (not just boxes)
- Anatomical accuracy (proper proportions)
- Defining features (claws, teeth, facial details)
- Multiple mesh segments for natural shapes
"""


# ─────────────────────────────────────────────────────────────────────────
# CORE GEOMETRY
# ─────────────────────────────────────────────────────────────────────────

def c(r,g,b,a=1.0): return [r,g,b,a]
def fv(v): return f"({v[0]:.4f}, {v[1]:.4f}, {v[2]:.4f})"
def fc(c): return f"({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f})"

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
    with open(os.path.join(OUT, filename), "w") as f:
        f.write(body)
    print(f"  ✓ {filename} ({len(meshes)} meshes)")

# ─────────────────────────────────────────────────────────────────────────
# ADVANCED GEOMETRY BUILDERS
# ─────────────────────────────────────────────────────────────────────────

def cylinder(name, cx, cy, cz, radius, height, color, segments=8):
    """Create a segmented cylinder for smooth curves."""
    verts = []
    indices = []

    # Bottom cap center
    verts.append([cx, cy, cz])

    # Bottom ring
    for i in range(segments):
        angle = (i / segments) * 2 * math.pi
        x = cx + radius * math.cos(angle)
        z = cz + radius * math.sin(angle)
        verts.append([x, cy, z])

    # Top ring
    for i in range(segments):
        angle = (i / segments) * 2 * math.pi
        x = cx + radius * math.cos(angle)
        z = cz + radius * math.sin(angle)
        verts.append([x, cy + height, z])

    # Top cap center
    verts.append([cx, cy + height, cz])

    # Bottom cap triangles
    for i in range(segments):
        indices.extend([0, i + 1, ((i + 1) % segments) + 1])

    # Side quads (as triangles)
    for i in range(segments):
        bottom1 = i + 1
        bottom2 = ((i + 1) % segments) + 1
        top1 = i + 1 + segments
        top2 = ((i + 1) % segments) + 1 + segments
        indices.extend([bottom1, bottom2, top1, bottom2, top2, top1])

    # Top cap triangles
    top_center = len(verts) - 1
    for i in range(segments):
        indices.extend([top_center, ((i + 1) % segments) + 1 + segments, i + 1 + segments])

    return emit_mesh(name, verts, indices, color)

def tapered_cylinder(name, cx, cy, cz, radius_bottom, radius_top, height, color, segments=8):
    """Tapered cylinder for legs, snouts, tails."""
    verts = []
    indices = []

    # Bottom ring
    for i in range(segments):
        angle = (i / segments) * 2 * math.pi
        x = cx + radius_bottom * math.cos(angle)
        z = cz + radius_bottom * math.sin(angle)
        verts.append([x, cy, z])

    # Top ring
    for i in range(segments):
        angle = (i / segments) * 2 * math.pi
        x = cx + radius_top * math.cos(angle)
        z = cz + radius_top * math.sin(angle)
        verts.append([x, cy + height, z])

    # Side quads
    for i in range(segments):
        bottom1 = i
        bottom2 = (i + 1) % segments
        top1 = i + segments
        top2 = ((i + 1) % segments) + segments
        indices.extend([bottom1, bottom2, top1, bottom2, top2, top1])

    # Caps (simple fans)
    center_bottom = len(verts)
    verts.append([cx, cy, cz])
    for i in range(segments):
        indices.extend([center_bottom, (i + 1) % segments, i])

    center_top = len(verts)
    verts.append([cx, cy + height, cz])
    for i in range(segments):
        indices.extend([center_top, i + segments, ((i + 1) % segments) + segments])

    return emit_mesh(name, verts, indices, color)

def sphere_section(name, cx, cy, cz, radius, color, segments_h=6, segments_v=6):
    """Spherical section for heads, joints."""
    verts = []
    indices = []

    for v in range(segments_v + 1):
        theta = (v / segments_v) * math.pi
        for h in range(segments_h):
            phi = (h / segments_h) * 2 * math.pi
            x = cx + radius * math.sin(theta) * math.cos(phi)
            y = cy + radius * math.cos(theta)
            z = cz + radius * math.sin(theta) * math.sin(phi)
            verts.append([x, y, z])

    # Build quad faces
    for v in range(segments_v):
        for h in range(segments_h):
            i1 = v * segments_h + h
            i2 = v * segments_h + (h + 1) % segments_h
            i3 = (v + 1) * segments_h + h
            i4 = (v + 1) * segments_h + (h + 1) % segments_h
            indices.extend([i1, i2, i3, i2, i4, i3])

    return emit_mesh(name, verts, indices, color)

def ellipsoid(name, cx, cy, cz, rx, ry, rz, color, segments_h=8, segments_v=6):
    """Ellipsoid for body masses."""
    verts = []
    indices = []

    for v in range(segments_v + 1):
        theta = (v / segments_v) * math.pi
        for h in range(segments_h):
            phi = (h / segments_h) * 2 * math.pi
            x = cx + rx * math.sin(theta) * math.cos(phi)
            y = cy + ry * math.cos(theta)
            z = cz + rz * math.sin(theta) * math.sin(phi)
            verts.append([x, y, z])

    for v in range(segments_v):
        for h in range(segments_h):
            i1 = v * segments_h + h
            i2 = v * segments_h + (h + 1) % segments_h
            i3 = (v + 1) * segments_h + h
            i4 = (v + 1) * segments_h + (h + 1) % segments_h
            indices.extend([i1, i2, i3, i2, i4, i3])

    return emit_mesh(name, verts, indices, color)

def cone_mesh(name, cx, cy, cz, radius, height, color, segments=8):
    """Cone for teeth, claws, horns."""
    verts = []
    indices = []

    # Base center
    verts.append([cx, cy, cz])

    # Base ring
    for i in range(segments):
        angle = (i / segments) * 2 * math.pi
        x = cx + radius * math.cos(angle)
        z = cz + radius * math.sin(angle)
        verts.append([x, cy, z])

    # Apex
    apex = len(verts)
    verts.append([cx, cy + height, cz])

    # Base cap
    for i in range(segments):
        indices.extend([0, ((i + 1) % segments) + 1, i + 1])

    # Sides
    for i in range(segments):
        indices.extend([i + 1, ((i + 1) % segments) + 1, apex])

    return emit_mesh(name, verts, indices, color)

# ─────────────────────────────────────────────────────────────────────────
# COLORS
# ─────────────────────────────────────────────────────────────────────────

WOLF_FUR = c(0.60, 0.56, 0.50)
WOLF_DARK = c(0.26, 0.24, 0.20)
EYE_YELLOW = c(0.88, 0.78, 0.10)
NOSE_BLACK = c(0.12, 0.10, 0.10)
CLAW_BONE = c(0.88, 0.86, 0.78)
TEETH_WHITE = c(0.95, 0.93, 0.90)
TONGUE_PINK = c(0.85, 0.45, 0.42)

BONE = c(0.88, 0.86, 0.78)
BONE_DIM = c(0.80, 0.78, 0.68)
EYE_GREEN = c(0.22, 0.72, 0.28)
IRON_RUST = c(0.46, 0.34, 0.26)
IRON_SHINE = c(0.58, 0.56, 0.60)

# ─────────────────────────────────────────────────────────────────────────
# DETAILED WOLF
# ─────────────────────────────────────────────────────────────────────────

def build_detailed_wolf():
    meshes = []

    # BODY - ellipsoid torso
    meshes.append(ellipsoid("body", 0, 0.15, -0.2, 0.25, 0.22, 0.35, WOLF_FUR, 12, 8))

    # CHEST - wider front section
    meshes.append(ellipsoid("chest", 0, 0.25, -0.55, 0.20, 0.18, 0.15, WOLF_FUR, 10, 6))

    # NECK - tapered cylinder connecting chest to head
    meshes.append(tapered_cylinder("neck", 0, 0.30, -0.68, 0.12, 0.15, 0.15, WOLF_FUR, 10))

    # HEAD - spherical with slight elongation
    meshes.append(ellipsoid("head", 0, 0.38, -0.80, 0.16, 0.14, 0.12, WOLF_FUR, 10, 8))

    # SNOUT - tapered cone for muzzle
    meshes.append(tapered_cylinder("snout_upper", 0, 0.35, -0.90, 0.09, 0.06, 0.12, WOLF_DARK, 8))
    meshes.append(tapered_cylinder("snout_lower", 0, 0.30, -0.90, 0.08, 0.05, 0.12, WOLF_DARK, 8))

    # NOSE - small sphere at tip
    meshes.append(sphere_section("nose", 0, 0.35, -1.02, 0.04, NOSE_BLACK, 6, 4))

    # NOSTRILS - tiny dark spots
    for side, sx in [("left", -0.02), ("right", 0.02)]:
        meshes.append(sphere_section(f"nostril_{side}", sx, 0.36, -1.03, 0.015, c(0.05, 0.05, 0.05), 4, 3))

    # EYES - spheres with pupils
    for side, sx in [("left", -0.10), ("right", 0.10)]:
        # Eye socket (darker)
        meshes.append(sphere_section(f"eye_socket_{side}", sx, 0.42, -0.85, 0.05, WOLF_DARK, 6, 4))
        # Eye (bright yellow)
        meshes.append(sphere_section(f"eye_{side}", sx, 0.42, -0.84, 0.038, EYE_YELLOW, 6, 4))
        # Pupil (dark center)
        meshes.append(sphere_section(f"pupil_{side}", sx, 0.42, -0.835, 0.018, c(0.08, 0.08, 0.08), 4, 3))

    # EARS - triangular with inner detail
    for side, sx in [("left", -0.12), ("right", 0.12)]:
        # Outer ear (tapered)
        meshes.append(cone_mesh(f"ear_outer_{side}", sx, 0.48, -0.78, 0.06, 0.16, WOLF_DARK, 8))
        # Inner ear (lighter, smaller)
        meshes.append(cone_mesh(f"ear_inner_{side}", sx, 0.50, -0.78, 0.04, 0.14, c(0.75, 0.70, 0.65), 6))

    # TEETH - visible fangs
    for side, sx in [("left", -0.03), ("right", 0.03)]:
        meshes.append(cone_mesh(f"fang_upper_{side}", sx, 0.38, -0.95, 0.012, 0.04, TEETH_WHITE, 6))
        meshes.append(cone_mesh(f"fang_lower_{side}", sx, 0.30, -0.95, 0.010, 0.03, TEETH_WHITE, 6))

    # TONGUE (slightly visible)
    verts = [[-0.02, 0.32, -0.94], [0.02, 0.32, -0.94], [0.015, 0.32, -0.98], [-0.015, 0.32, -0.98]]
    indices = [0, 1, 2, 0, 2, 3]
    normals = [[0, 1, 0]] * 4
    meshes.append(emit_mesh("tongue", verts, indices, TONGUE_PINK, normals))

    # LEGS - tapered cylinders with joints
    legs = [
        ("front_left",  -0.18, -0.50, -0.50),
        ("front_right",  0.18, -0.50, -0.50),
        ("hind_left",   -0.18, -0.50,  0.10),
        ("hind_right",   0.18, -0.50,  0.10),
    ]

    for name, lx, ly, lz in legs:
        # Upper leg (thicker)
        meshes.append(tapered_cylinder(f"{name}_upper", lx, ly, lz, 0.055, 0.045, 0.22, WOLF_FUR, 8))
        # Lower leg (thinner)
        meshes.append(tapered_cylinder(f"{name}_lower", lx, ly + 0.22, lz, 0.045, 0.035, 0.22, WOLF_FUR, 8))
        # Paw (small sphere)
        meshes.append(sphere_section(f"{name}_paw", lx, ly + 0.44, lz, 0.04, WOLF_DARK, 6, 4))
        # Claws (4 small cones)
        for i, offset in enumerate([[-0.02, 0.02], [0.02, 0.02], [-0.02, -0.02], [0.02, -0.02]]):
            meshes.append(cone_mesh(
                f"{name}_claw_{i}",
                lx + offset[0], ly + 0.46, lz + offset[1],
                0.006, 0.015, CLAW_BONE, 4
            ))

    # TAIL - segmented for natural curve
    tail_segments = [
        (0.22, 0.05, 0.05),   # Base (thick)
        (0.32, 0.045, 0.04),  # Mid
        (0.42, 0.04, 0.035),  # Tip approach
        (0.52, 0.035, 0.03),  # Tip
    ]
    for i, (tz, r1, r2) in enumerate(tail_segments):
        y_base = 0.10 if i == 0 else 0.06
        meshes.append(tapered_cylinder(
            f"tail_seg_{i}",
            0, y_base, tz,
            r1, r2, 0.10,
            WOLF_FUR if i < 3 else WOLF_DARK,
            8
        ))

    return meshes

# ─────────────────────────────────────────────────────────────────────────
# DETAILED SKELETON
# ─────────────────────────────────────────────────────────────────────────

def build_detailed_skeleton():
    meshes = []

    # LEGS - thin bone cylinders with joints
    for side, sx in [("left", -0.08), ("right", 0.08)]:
        # Femur
        meshes.append(tapered_cylinder(f"{side}_femur", sx, -1.0, 0, 0.025, 0.022, 0.50, BONE, 6))
        # Knee joint
        meshes.append(sphere_section(f"{side}_knee", sx, -0.50, 0, 0.032, BONE, 6, 4))
        # Tibia
        meshes.append(tapered_cylinder(f"{side}_tibia", sx, -0.50, 0, 0.022, 0.020, 0.50, BONE, 6))
        # Ankle
        meshes.append(sphere_section(f"{side}_ankle", sx, 0.00, 0, 0.028, BONE, 6, 4))
        # Foot bones (3 small cylinders)
        for i, offset in enumerate([-0.03, 0, 0.03]):
            meshes.append(tapered_cylinder(
                f"{side}_toe_{i}",
                sx + offset, 0.00, 0.02, 0.012, 0.008, 0.06, BONE, 4
            ))

    # PELVIS - connecting piece
    meshes.append(ellipsoid("pelvis", 0, -0.05, 0, 0.12, 0.08, 0.08, BONE, 8, 4))

    # SPINE - segmented vertebrae
    for i in range(8):
        y = 0.05 + i * 0.10
        meshes.append(cylinder(f"vertebra_{i}", 0, y, 0, 0.030, 0.08, BONE_DIM, 6))

    # RIBCAGE - individual ribs
    for i in range(6):
        y = 0.15 + i * 0.10
        # Left rib
        verts_l = [[0, y, 0], [-0.08, y, 0.02], [-0.14, y + 0.04, 0.04],
                   [-0.16, y + 0.06, 0.02], [-0.15, y + 0.08, 0]]
        indices_l = [0, 1, 2, 2, 3, 4]
        meshes.append(emit_mesh(f"rib_left_{i}", verts_l, indices_l, BONE_DIM))

        # Right rib (mirror)
        verts_r = [[0, y, 0], [0.08, y, 0.02], [0.14, y + 0.04, 0.04],
                   [0.16, y + 0.06, 0.02], [0.15, y + 0.08, 0]]
        indices_r = [0, 1, 2, 2, 3, 4]
        meshes.append(emit_mesh(f"rib_right_{i}", verts_r, indices_r, BONE_DIM))

    # SHOULDERS - ball joints
    for side, sx in [("left", -0.20), ("right", 0.20)]:
        meshes.append(sphere_section(f"{side}_shoulder", sx, 0.65, 0, 0.04, BONE, 6, 4))
        # Humerus
        meshes.append(tapered_cylinder(f"{side}_humerus", sx, 0.65, 0, 0.022, 0.020, -0.25, BONE, 6))
        # Elbow
        meshes.append(sphere_section(f"{side}_elbow", sx, 0.40, 0, 0.028, BONE, 6, 4))
        # Forearm
        meshes.append(tapered_cylinder(f"{side}_forearm", sx, 0.40, 0, 0.020, 0.018, -0.30, BONE, 6))
        # Hand bones
        for i, offset in enumerate([-0.02, 0, 0.02]):
            meshes.append(tapered_cylinder(
                f"{side}_finger_{i}",
                sx + offset, 0.10, 0.02, 0.010, 0.006, -0.08, BONE, 4
            ))

    # SKULL - detailed cranium
    meshes.append(ellipsoid("cranium", 0, 0.95, -0.08, 0.14, 0.16, 0.12, BONE, 10, 8))

    # JAW - hinged mandible
    jaw_verts = [
        [-0.10, 0.82, -0.05], [0.10, 0.82, -0.05],  # Back
        [-0.08, 0.82, 0.05], [0.08, 0.82, 0.05],    # Front inner
        [-0.06, 0.80, 0.08], [0.06, 0.80, 0.08],    # Chin
    ]
    jaw_indices = [0, 1, 3, 0, 3, 2, 2, 3, 5, 2, 5, 4]
    meshes.append(emit_mesh("jaw", jaw_verts, jaw_indices, BONE))

    # TEETH - in jaw
    for i in range(8):
        x = -0.05 + i * 0.0125
        meshes.append(cone_mesh(f"tooth_{i}", x, 0.80, 0.08, 0.006, 0.02, TEETH_WHITE, 4))

    # EYE SOCKETS - dark hollow spheres
    for side, sx in [("left", -0.08), ("right", 0.08)]:
        meshes.append(sphere_section(f"{side}_socket", sx, 0.96, 0.04, 0.04, c(0.08, 0.08, 0.08), 6, 4))
        # Glowing eyes inside
        meshes.append(sphere_section(f"{side}_eye", sx, 0.96, 0.045, 0.025, EYE_GREEN, 6, 4))

    # WEAPON - rusted sword with detail
    # Blade (tapered)
    meshes.append(tapered_cylinder("sword_blade", 0.54, -0.20, 0, 0.02, 0.015, 1.05, IRON_RUST, 6))
    # Blood groove
    verts_groove = [[0.545, -0.15, 0], [0.545, 0.80, 0]]
    indices_groove = [0, 1]
    # Guard (crosspiece)
    meshes.append(cylinder("sword_guard", 0.54, 0.08, 0, 0.06, 0.04, IRON_SHINE, 6))
    # Grip (wrapped)
    for i in range(4):
        y = 0.12 + i * 0.04
        meshes.append(cylinder(f"grip_wrap_{i}", 0.54, y, 0, 0.018, 0.03, c(0.35, 0.25, 0.18), 6))
    # Pommel
    meshes.append(sphere_section("pommel", 0.54, 0.28, 0, 0.025, IRON_SHINE, 6, 4))

    return meshes

# ─────────────────────────────────────────────────────────────────────────
# GENERATE
# ─────────────────────────────────────────────────────────────────────────

"""
Create 2 new wolf variants:
- Dire Wolf (id: 13) - bigger, meaner, light grey
- Dire Wolf Leader (id: 14) - biggest, meanest, almost white
"""


# ─────────────────────────────────────────────────────────────────────────
# Core helpers
# ─────────────────────────────────────────────────────────────────────────
BOX_I = [0,1,2,2,3,0, 1,5,6,6,2,1, 7,6,5,5,4,7,
         4,0,3,3,7,4, 4,5,1,1,0,4, 3,2,6,6,7,3]


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

"""
Generate procedural mesh RON files for the 11 missing monsters from monsters.ron.
Already covered: dying_goblin, skeleton_warrior, evil_lich, red_dragon.
New: goblin, kobold, giant_rat, orc, skeleton, wolf, ogre, zombie,
     fire_elemental, dragon, lich
"""


# ── geometry ──────────────────────────────────────────────────────────────
BOX_I = [0,1,2,2,3,0, 1,5,6,6,2,1, 7,6,5,5,4,7,
         4,0,3,3,7,4, 4,5,1,1,0,4, 3,2,6,6,7,3]

def m_box_v(x0,y0,z0, x1,y1,z1):
    return [
        [x0,y0,z0],[x1,y0,z0],[x1,y1,z0],[x0,y1,z0],
        [x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1],
    ]

def m_quad_v(v0,v1,v2,v3):
    return [v0,v1,v2,v3]

def m_quad_i():
    return [0,1,2,0,2,3]

def m_tri_v(v0,v1,v2):
    return [v0,v1,v2]

def m_tri_i():
    return [0,1,2]

def m_norm3(nx,ny,nz,n):
    return [[nx,ny,nz]]*n

# ── serialisation ─────────────────────────────────────────────────────────
def fv_monsters(v):  return f"({v[0]:.3f}, {v[1]:.3f}, {v[2]:.3f})"
def fc_monsters(c):  return f"({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f})"

def emit(name, verts, indices, color, normals=None):
    L = [
        f'        (',
        f'            name: Some("{name}"),',
        f'            vertices: [',
    ]
    for v in verts:
        L.append(f'                {fv_monsters(v)},')
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
    L.append(f'            color: {fc_monsters(color)},')
    L.append('            lod_levels: None,')
    L.append('            lod_distances: None,')
    L.append('            material: None,')
    L.append('            texture_path: None,')
    L.append('        ),')
    return '\n'.join(L)

def m_box_mesh(name, x0,y0,z0, x1,y1,z1, color):
    return emit(name, m_box_v(x0,y0,z0,x1,y1,z1), list(BOX_I), color)

def m_quad_mesh(name, v0,v1,v2,v3, color, nx=0,ny=0,nz=1):
    return emit(name, m_quad_v(v0,v1,v2,v3), m_quad_i(), color, m_norm3(nx,ny,nz,4))

def m_tri_mesh(name, v0,v1,v2, color, nx=0,ny=0,nz=1):
    return emit(name, m_tri_v(v0,v1,v2), m_tri_i(), color, m_norm3(nx,ny,nz,3))



# ── colour palette ────────────────────────────────────────────────────────

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
        m_box_mesh("left_leg",  -0.15,-sy,-0.10, -0.05,0.00,0.05, color),
        m_box_mesh("right_leg",  0.05,-sy,-0.10,  0.15,0.00,0.05, color),
    ]

def biped_torso(color, w=0.27, h=0.80):
    return [m_box_mesh("torso", -w,0.00,-0.15, w,h,0.12, color)]

def biped_arms(color, reach=0.48):
    return [
        m_box_mesh("left_arm",  -reach,0.10,-0.10, -0.27,0.65,0.05, color),
        m_box_mesh("right_arm",  0.27, 0.10,-0.10,  reach,0.65,0.05, color),
    ]

def biped_head(color, w=0.17, h=0.30, yb=0.80):
    return [m_box_mesh("head", -w,yb,-0.17, w,yb+h,0.07, color)]

def eye_pair(lc, rc=None, y=1.00, z=0.071, ox=0.07, ew=0.04, eh=0.06):
    rc = rc or lc
    return [
        m_quad_mesh("left_eye",
            [-ox-ew,y,z],[-ox+ew,y,z],[-ox-ew,y+eh,z],[-ox+ew,y+eh,z], lc),
        m_quad_mesh("right_eye",
            [ ox-ew,y,z],[ ox+ew,y,z],[ ox-ew,y+eh,z],[ ox+ew,y+eh,z], rc),
    ]

def tusk_pair(color):
    return [
        m_tri_mesh("left_tusk",  [-0.06,0.82,0.07],[-0.02,0.82,0.07],[-0.04,0.70,0.11], color, ny=-1),
        m_tri_mesh("right_tusk",  [0.02,0.82,0.07],[ 0.06,0.82,0.07],[ 0.04,0.70,0.11], color, ny=-1),
    ]

def ear_pair_large(color):
    return [
        m_tri_mesh("left_ear",  [-0.17,1.08,-0.08],[-0.17,0.90,-0.08],[-0.38,0.99,-0.12], color, nx=-1),
        m_tri_mesh("right_ear",  [0.17,1.08,-0.08],[ 0.17,0.90,-0.08],[ 0.38,0.99,-0.12], color, nx=1),
    ]

def ear_pair_pointed(color):
    return [
        m_tri_mesh("left_ear",  [-0.17,1.02,-0.08],[-0.17,0.94,-0.08],[-0.28,0.98,-0.10], color, nx=-1),
        m_tri_mesh("right_ear",  [0.17,1.02,-0.08],[ 0.17,0.94,-0.08],[ 0.28,0.98,-0.10], color, nx=1),
    ]

def horn_pair(color, sx=0.13, by=1.10, ty=1.35, tz=-0.10):
    return [
        m_tri_mesh("left_horn",  [-sx-0.04,by,-0.08],[-sx+0.04,by,-0.08],[-sx,ty,tz], color, ny=1),
        m_tri_mesh("right_horn",  [sx-0.04,by,-0.08],[ sx+0.04,by,-0.08],[ sx,ty,tz], color, ny=1),
    ]

def brow_ridge(color, w=0.20):
    return [m_box_mesh("brow", -w,1.06,-0.05, w,1.10,0.08, color)]

def warpaint_stripe(color):
    return [m_quad_mesh("warpaint",
        [-0.18,0.98,0.071],[0.18,0.98,0.071],
        [-0.18,1.04,0.071],[0.18,1.04,0.071], color)]

def ribs(color):
    out = []
    for k, y in enumerate([0.18,0.34,0.50]):
        out.append(m_quad_mesh(f"rib_{k}",
            [-0.22,y,0.122],[0.22,y,0.122],
            [-0.22,y+0.06,0.122],[0.22,y+0.06,0.122], color))
    return out

def rot_spots(color):
    out = []
    spots = [(0.10,0.44,0.122),(-0.14,0.22,0.122),(0.04,0.64,0.122)]
    for k,(px,py,pz) in enumerate(spots):
        out.append(m_quad_mesh(f"rot_{k}",
            [px-0.06,py-0.04,pz],[px+0.06,py-0.04,pz],
            [px-0.06,py+0.04,pz],[px+0.06,py+0.04,pz],
            c(color[0],color[1],color[2],0.80)))
    return out

def aura_sparks(color, count=3):
    pts = [(-0.32,0.50,0.18),(0.28,0.66,0.18),(0.00,1.06,0.16),(0.18,0.28,0.19),(-0.08,0.78,0.20)]
    out = []
    for k in range(count):
        px,py,pz = pts[k]
        out.append(m_tri_mesh(f"aura_{k}",
            [px-0.08,py,pz],[px+0.08,py,pz],[px,py+0.16,pz],
            c(color[0],color[1],color[2],0.55)))
    return out

def sword_weapon(color):
    return [
        m_box_mesh("sword_blade",  0.52,-0.20,-0.06, 0.56,0.85,-0.02, color),
        m_box_mesh("sword_guard",  0.44,0.08,-0.08, 0.64,0.16,0.00, c(0.55,0.50,0.45)),
    ]

def dagger_weapon(color):
    return [m_box_mesh("dagger", 0.50,0.00,-0.05, 0.53,0.38,-0.01, color)]

def club_weapon(color):
    return [
        m_box_mesh("club_shaft", -0.54,-0.20,-0.07, -0.50,0.65,-0.03, WOOD),
        m_box_mesh("club_head",  -0.66,0.55,-0.11, -0.38,0.80,0.03, color),
    ]

def staff_weapon(shaft_c, orb_c):
    return [
        m_box_mesh("staff",     0.52,0.00,-0.06, 0.56,1.76,-0.02, shaft_c),
        m_box_mesh("staff_orb", 0.44,1.74,-0.10, 0.64,1.94,0.04, orb_c),
    ]

def bone_weapon():
    return [m_box_mesh("bone_weapon", -0.54,-0.18,-0.05, -0.50,0.55,-0.01, BONE)]

def m_fur_mantle(color):
    return [m_box_mesh("m_fur_mantle", -0.34,0.55,-0.18, 0.34,0.82,0.16, color)]

def loincloth(color):
    return [
        m_quad_mesh("loincloth_f",
            [-0.18,0.00,0.13],[0.18,0.00,0.13],
            [-0.14,-0.24,0.10],[0.14,-0.24,0.10], color),
        m_quad_mesh("loincloth_b",
            [-0.18,0.00,-0.14],[0.18,0.00,-0.14],
            [-0.14,-0.24,-0.11],[0.14,-0.24,-0.11], color, ny=0,nz=-1),
    ]

def crown_spikes(color):
    out = []
    for k, sx in enumerate([-0.14,-0.04,0.04,0.14]):
        out.append(m_tri_mesh(f"crown_{k}",
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
        [m_quad_mesh("scale_row_0",
            [-0.17,0.14,0.122],[0.17,0.14,0.122],[-0.17,0.20,0.122],[0.17,0.20,0.122],
            c(0.48,0.24,0.12,0.70)),
         m_quad_mesh("scale_row_1",
            [-0.17,0.32,0.122],[0.17,0.32,0.122],[-0.17,0.38,0.122],[0.17,0.38,0.122],
            c(0.48,0.24,0.12,0.70))] +
        biped_arms(SKIN_KOBOLD, reach=0.32) +
        # tail
        [m_box_mesh("tail_base", -0.04,-0.20,-0.20, 0.04,0.30,-0.14, SKIN_KOBOLD),
         m_box_mesh("tail_tip",  -0.03,-0.20,-0.36, 0.03,0.10,-0.20, c(0.45,0.24,0.14))] +
        biped_head(SKIN_KOBOLD, w=0.16, h=0.26, yb=0.54) +
        # snout
        [m_box_mesh("snout", -0.08,0.56,0.08, 0.08,0.70,0.20, c(0.48,0.26,0.14))] +
        horn_pair(HORN_DARK, sx=0.10, by=0.80, ty=0.98, tz=-0.08) +
        eye_pair(EYE_ORANGE, y=0.72, ox=0.07) +
        dagger_weapon(c(0.40,0.40,0.44))
    )
    write_ron("kobold.ron","Kobold",0.70,28.0,7.5,
        "id:2 | Speed 10, HP 5 | Reptilian, scaly, tail, orange eyes", m)

def build_giant_rat():
    """id:3 — Speed 12, Disease carrier. Quadruped, red beady eyes."""
    m = [
        m_box_mesh("body",        -0.22,-0.10,-0.45,  0.22,0.28, 0.18, RAT_FUR),
        m_box_mesh("head",        -0.14,-0.06,-0.62,  0.14,0.22,-0.42, RAT_FUR),
        m_box_mesh("snout",       -0.07,-0.08,-0.72,  0.07,0.10,-0.60, RAT_DARK),
        m_box_mesh("front_left",  -0.22,-0.36,-0.38, -0.14,-0.10,-0.28, RAT_FUR),
        m_box_mesh("front_right",  0.14,-0.36,-0.38,  0.22,-0.10,-0.28, RAT_FUR),
        m_box_mesh("hind_left",   -0.22,-0.36, 0.04, -0.13,-0.10, 0.16, RAT_FUR),
        m_box_mesh("hind_right",   0.13,-0.36, 0.04,  0.22,-0.10, 0.16, RAT_FUR),
        m_box_mesh("left_ear",    -0.14, 0.18,-0.60, -0.06,0.32,-0.56, c(0.52,0.36,0.30)),
        m_box_mesh("right_ear",    0.06, 0.18,-0.60,  0.14,0.32,-0.56, c(0.52,0.36,0.30)),
        # beady disease-red eyes
        m_quad_mesh("left_eye",
            [-0.11,-0.02,-0.625],[-0.07,-0.02,-0.625],
            [-0.11, 0.04,-0.625],[-0.07, 0.04,-0.625],
            EYE_RED, nx=0,ny=0,nz=-1),
        m_quad_mesh("right_eye",
            [ 0.07,-0.02,-0.625],[ 0.11,-0.02,-0.625],
            [ 0.07, 0.04,-0.625],[ 0.11, 0.04,-0.625],
            EYE_RED, nx=0,ny=0,nz=-1),
        m_box_mesh("tail",        -0.03,-0.05, 0.18,  0.03,0.08, 0.55, c(0.36,0.26,0.22)),
    ] + aura_sparks(DISEASE, count=2)
    write_ron("giant_rat.ron","GiantRat",0.65,20.0,8.0,
        "id:3 | Speed 12, Disease | Quadruped, red eyes, disease aura", m)

def build_orc():
    """id:10 — Might 12, HP 25. Heavy warrior, tusks, warpaint, crude club."""
    m = (
        biped_legs(LEATHER_DARK) +
        biped_torso(SKIN_ORC, w=0.30, h=0.82) +
        m_fur_mantle(FUR_BROWN) +
        [m_quad_mesh("chest_scar",
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
        m_box_mesh("left_leg",   -0.10,-1.00,-0.06, -0.04,0.00,0.02, BONE),
        m_box_mesh("right_leg",   0.04,-1.00,-0.06,  0.10,0.00,0.02, BONE),
        # narrow ribbed torso
        m_box_mesh("torso",      -0.22, 0.00,-0.10,  0.22,0.76,0.06, BONE),
    ] + ribs(BONE_DIM) + [
        m_box_mesh("left_arm",   -0.42, 0.10,-0.06, -0.22,0.62,0.02, BONE),
        m_box_mesh("right_arm",   0.22, 0.10,-0.06,  0.42,0.62,0.02, BONE),
        # skull
        m_box_mesh("skull",      -0.16, 0.78,-0.16,  0.16,1.06,0.05, BONE),
        # hollow eye sockets
        m_quad_mesh("left_socket",
            [-0.13,0.92,0.051],[-0.07,0.92,0.051],
            [-0.13,1.00,0.051],[-0.07,1.00,0.051],
            c(0.10,0.08,0.06)),
        m_quad_mesh("right_socket",
            [ 0.07,0.92,0.051],[ 0.13,0.92,0.051],
            [ 0.07,1.00,0.051],[ 0.13,1.00,0.051],
            c(0.10,0.08,0.06)),
        # glowing green eye lights
        m_quad_mesh("left_glow",
            [-0.12,0.93,0.055],[-0.08,0.93,0.055],
            [-0.12,0.99,0.055],[-0.08,0.99,0.055],
            EYE_GREEN),
        m_quad_mesh("right_glow",
            [ 0.08,0.93,0.055],[ 0.12,0.93,0.055],
            [ 0.08,0.99,0.055],[ 0.12,0.99,0.055],
            EYE_GREEN),
        # jaw gap
        m_quad_mesh("jaw_gap",
            [-0.10,0.80,0.051],[0.10,0.80,0.051],
            [-0.10,0.86,0.051],[0.10,0.86,0.051],
            c(0.10,0.08,0.06)),
    ] + sword_weapon(IRON_RUST)
    write_ron("skeleton.ron","Skeleton",1.00,60.0,3.5,
        "id:11 | Undead, HP 20, cold/fear immune | Bone body, glowing eyes, rusty sword", m)

def build_wolf():
    """id:12 — Speed 14, HP 18. Fast quadruped hunter, grey fur, amber eyes."""
    m = [
        m_box_mesh("body",         -0.28,-0.05,-0.55,  0.28,0.38,0.22, WOLF_FUR),
        m_box_mesh("chest",        -0.22, 0.12,-0.58,  0.22,0.42,-0.28, WOLF_FUR),
        m_box_mesh("head",         -0.18,-0.02,-0.76,  0.18,0.32,-0.52, WOLF_FUR),
        m_box_mesh("snout",        -0.10,-0.06,-0.90,  0.10,0.16,-0.74, WOLF_DARK),
        m_box_mesh("nostrils",     -0.05,-0.04,-0.905, 0.05,0.04,-0.895, c(0.14,0.10,0.08)),
        m_tri_mesh("left_ear",
            [-0.15,0.30,-0.64],[-0.06,0.30,-0.64],[-0.10,0.49,-0.64],
            WOLF_DARK, nx=0,ny=0,nz=-1),
        m_tri_mesh("right_ear",
            [ 0.06,0.30,-0.64],[ 0.15,0.30,-0.64],[ 0.10,0.49,-0.64],
            WOLF_DARK, nx=0,ny=0,nz=-1),
        m_quad_mesh("left_eye",
            [-0.14, 0.12,-0.752],[-0.08,0.12,-0.752],
            [-0.14, 0.20,-0.752],[-0.08,0.20,-0.752],
            EYE_YELLOW, nx=0,ny=0,nz=-1),
        m_quad_mesh("right_eye",
            [ 0.08, 0.12,-0.752],[ 0.14,0.12,-0.752],
            [ 0.08, 0.20,-0.752],[ 0.14,0.20,-0.752],
            EYE_YELLOW, nx=0,ny=0,nz=-1),
        m_box_mesh("front_left",   -0.28,-0.50,-0.44, -0.18,-0.06,-0.34, WOLF_FUR),
        m_box_mesh("front_right",   0.18,-0.50,-0.44,  0.28,-0.06,-0.34, WOLF_FUR),
        m_box_mesh("hind_left",    -0.28,-0.50, 0.06, -0.18,-0.06,0.18, WOLF_FUR),
        m_box_mesh("hind_right",    0.18,-0.50, 0.06,  0.28,-0.06,0.18, WOLF_FUR),
        m_box_mesh("tail",         -0.06,-0.02, 0.22,  0.06,0.28,0.54, WOLF_FUR),
        m_box_mesh("tail_tip",     -0.04, 0.26, 0.46,  0.04,0.42,0.58, WOLF_DARK),
    ]
    write_ron("wolf.ron","Wolf",0.90,55.0,8.5,
        "id:12 | Speed 14, HP 18 | Fast quadruped, grey fur, amber eyes, pack hunter", m)

def build_ogre():
    """id:20 — Might 18, HP 60, can_regenerate. Massive brute with huge club."""
    m = (
        biped_legs(c(0.42,0.38,0.26), sy=1.10) +
        biped_torso(SKIN_OGRE, w=0.40, h=0.90) +
        # pot belly
        [m_box_mesh("belly", -0.36,0.05,-0.12, 0.36,0.46,0.26, c(0.54,0.49,0.34))] +
        m_fur_mantle(c(0.38,0.28,0.16)) +
        # massive arms
        [m_box_mesh("left_arm",   -0.64,0.08,-0.14, -0.38,0.76,0.10, SKIN_OGRE),
         m_box_mesh("right_arm",   0.38,0.08,-0.14,  0.64,0.76,0.10, SKIN_OGRE),
         m_box_mesh("left_fist",  -0.68,-0.08,-0.12, -0.40,0.12,0.08, c(0.44,0.40,0.28)),
         m_box_mesh("right_fist",  0.40,-0.08,-0.12,  0.68,0.12,0.08, c(0.44,0.40,0.28))] +
        biped_head(SKIN_OGRE, w=0.26, h=0.36, yb=0.92) +
        [m_box_mesh("brow", -0.27,1.22,-0.08, 0.27,1.28,0.10, c(0.44,0.40,0.26))] +
        tusk_pair(TUSK) +
        eye_pair(EYE_RED, y=1.12, ox=0.12) +
        # massive club
        [m_box_mesh("club_shaft", -0.74,-0.30,-0.09, -0.64,0.74,-0.03, WOOD),
         m_box_mesh("club_knob",  -0.88,0.64,-0.15, -0.50,0.92,0.07, c(0.26,0.16,0.08))] +
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
        [m_quad_mesh("rags_front",
            [-0.28,0.08,0.122],[0.28,0.08,0.122],
            [-0.32,-0.22,0.118],[0.32,-0.22,0.118],
            c(0.38,0.33,0.22,0.82))] +
        biped_arms(ZOMBIE_DARK, reach=0.50) +
        # one arm outstretched lower (shambling pose)
        [m_box_mesh("left_arm_low", -0.52,-0.10,-0.10, -0.26,0.22,0.06, ZOMBIE_DARK)] +
        biped_head(ZOMBIE_GREY, w=0.18, h=0.30) +
        eye_pair(c(0.82,0.58,0.08), y=0.98, ox=0.08) +
        # gaping jaw
        [m_box_mesh("jaw",        -0.12,0.80,-0.02, 0.12,0.86,0.08, c(0.28,0.26,0.20)),
         m_quad_mesh("open_mouth",
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
        m_box_mesh("base",       -0.22,-0.05,-0.20, 0.22,0.22,0.20, LAVA),
        # flame body — 4 stacking layers getting narrower toward top
        m_box_mesh("flame_0",    -0.28, 0.00,-0.24, 0.28,0.38,0.24, FIRE_MID),
        m_box_mesh("flame_1",    -0.24, 0.30,-0.20, 0.24,0.64,0.20, c(1.00,0.60,0.10)),
        m_box_mesh("flame_2",    -0.20, 0.56,-0.16, 0.20,0.88,0.16, c(1.00,0.78,0.14)),
        m_box_mesh("flame_3",    -0.16, 0.80,-0.12, 0.16,1.06,0.12, FIRE_BRIGHT),
        # bright inner core
        m_box_mesh("core",       -0.12, 0.10,-0.10, 0.12,0.96,0.10, FIRE_BRIGHT),
        # flame head column
        m_box_mesh("flame_head", -0.18, 0.94,-0.14, 0.18,1.22,0.14, c(1.00,0.82,0.18,0.92)),
        # arm tendrils of fire
        m_box_mesh("left_arm",   -0.52, 0.20,-0.12, -0.28,0.74,0.08, FIRE_MID),
        m_box_mesh("right_arm",   0.28, 0.20,-0.12,  0.52,0.74,0.08, FIRE_MID),
        # eyes: white-hot cores
        m_quad_mesh("left_eye",
            [-0.10,1.03,0.141],[-0.06,1.03,0.141],
            [-0.10,1.09,0.141],[-0.06,1.09,0.141],
            c(1.00,1.00,0.85)),
        m_quad_mesh("right_eye",
            [ 0.06,1.03,0.141],[ 0.10,1.03,0.141],
            [ 0.06,1.09,0.141],[ 0.10,1.09,0.141],
            c(1.00,1.00,0.85)),
        # flame tips above head
        m_tri_mesh("tip_0", [-0.20,1.22,0.0],[-0.08,1.22,0.0],[-0.14,1.46,0.0], FIRE_GLOW, ny=1),
        m_tri_mesh("tip_1", [-0.06,1.18,0.0],[ 0.06,1.18,0.0],[ 0.00,1.50,0.0], FIRE_GLOW, ny=1),
        m_tri_mesh("tip_2", [ 0.08,1.22,0.0],[ 0.20,1.22,0.0],[ 0.14,1.46,0.0], FIRE_GLOW, ny=1),
    ] + aura_sparks(FIRE_DARK, count=3)
    write_ron("fire_elemental.ron","FireElemental",1.10,160.0,6.0,
        "id:22 | Fire+Physical immune, HP 70, magic_res 50 | Pure flame creature", m)

def build_dragon():
    """id:30 — Might 25, HP 200, fire immune, 2 attacks, regenerates. Base combat dragon."""
    m = [
        # scaled body (horizontal, front-facing)
        m_box_mesh("body",          -0.38,-0.12,-0.82,  0.38,0.72,0.40, SCALE_RED),
        # lighter belly underside
        m_quad_mesh("belly",
            [-0.36,-0.11,-0.80],[0.36,-0.11,-0.80],
            [-0.36,-0.11, 0.38],[0.36,-0.11, 0.38],
            SCALE_BELLY, nx=0,ny=-1,nz=0),
        # neck
        m_box_mesh("neck",          -0.18, 0.52,-0.90,  0.18,0.96,-0.38, SCALE_RED),
        # head
        m_box_mesh("head",          -0.22, 0.76,-1.10,  0.22,1.12,-0.82, SCALE_RED),
        # snout
        m_box_mesh("snout",         -0.14, 0.76,-1.32,  0.14,0.96,-1.08, SCALE_DARK),
        # lower jaw
        m_box_mesh("jaw",           -0.18, 0.72,-1.12,  0.18,0.80,-0.84, SCALE_DARK),
        # fire glow inside mouth
        m_box_mesh("mouth_glow",    -0.14, 0.76,-1.10,  0.14,0.80,-0.86, c(1.00,0.70,0.10,0.80)),
        # eyes
        m_quad_mesh("left_eye",
            [-0.21,0.96,-0.852],[-0.13,0.96,-0.852],
            [-0.21,1.04,-0.852],[-0.13,1.04,-0.852],
            EYE_ORANGE, nx=0,ny=0,nz=-1),
        m_quad_mesh("right_eye",
            [ 0.13,0.96,-0.852],[ 0.21,0.96,-0.852],
            [ 0.13,1.04,-0.852],[ 0.21,1.04,-0.852],
            EYE_ORANGE, nx=0,ny=0,nz=-1),
        # head horns
        m_tri_mesh("left_horn",
            [-0.19,1.10,-0.91],[-0.11,1.10,-0.91],[-0.15,1.40,-0.99],
            HORN_DARK, ny=1),
        m_tri_mesh("right_horn",
            [ 0.11,1.10,-0.91],[ 0.19,1.10,-0.91],[ 0.15,1.40,-0.99],
            HORN_DARK, ny=1),
        # 4 legs
        m_box_mesh("front_left",    -0.52,-0.54,-0.64, -0.36,-0.10,-0.48, SCALE_RED),
        m_box_mesh("front_right",    0.36,-0.54,-0.64,  0.52,-0.10,-0.48, SCALE_RED),
        m_box_mesh("hind_left",     -0.52,-0.54, 0.14, -0.36,-0.10, 0.30, SCALE_RED),
        m_box_mesh("hind_right",     0.36,-0.54, 0.14,  0.52,-0.10, 0.30, SCALE_RED),
        # front claws
        m_tri_mesh("claw_fl",
            [-0.50,-0.72,-0.62],[-0.44,-0.72,-0.62],[-0.47,-0.88,-0.56],
            BONE, ny=-1,nz=0),
        m_tri_mesh("claw_fr",
            [ 0.44,-0.72,-0.62],[ 0.50,-0.72,-0.62],[ 0.47,-0.88,-0.56],
            BONE, ny=-1,nz=0),
        # tail
        m_box_mesh("tail_base",     -0.20,-0.10, 0.40,  0.20,0.44,0.72, SCALE_RED),
        m_box_mesh("tail_mid",      -0.14,-0.08, 0.72,  0.14,0.34,1.02, SCALE_DARK),
        m_box_mesh("tail_tip",      -0.06,-0.04, 1.02,  0.06,0.18,1.26, SCALE_DARK),
        # tail spikes
        m_tri_mesh("tail_spike_0",
            [-0.04,0.42,0.58],[0.04,0.42,0.58],[0.00,0.60,0.54],
            HORN_DARK, ny=1),
        m_tri_mesh("tail_spike_1",
            [-0.04,0.32,0.88],[0.04,0.32,0.88],[0.00,0.48,0.84],
            HORN_DARK, ny=1),
        # wing membranes
        m_quad_mesh("left_wing",
            [-0.38,0.56,-0.22],[-0.38,0.56, 0.34],
            [-1.30, 0.00,-0.26],[-1.30, 0.00, 0.40],
            WING, nx=-1,ny=0,nz=0),
        m_quad_mesh("right_wing",
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
        [m_quad_mesh("robe_trim_bot",
            [-0.32,0.02,0.122],[0.32,0.02,0.122],
            [-0.32,0.08,0.122],[0.32,0.08,0.122], LICH_TRIM),
         m_quad_mesh("robe_trim_top",
            [-0.32,0.80,0.122],[0.32,0.80,0.122],
            [-0.32,0.86,0.122],[0.32,0.86,0.122], LICH_TRIM)] +
        # wide flowing sleeves
        [m_box_mesh("left_sleeve",  -0.60,0.12,-0.14, -0.28,0.80,0.06, LICH_ROBE),
         m_box_mesh("right_sleeve",  0.28,0.12,-0.14,  0.60,0.80,0.06, LICH_ROBE),
         # skeletal hand ends
         m_box_mesh("left_hand",   -0.62,0.06,-0.08, -0.52,0.18,0.00, BONE),
         m_box_mesh("right_hand",   0.52,0.06,-0.08,  0.62,0.18,0.00, BONE)] +
        # skull
        [m_box_mesh("skull",       -0.17,0.90,-0.16, 0.17,1.18,0.05, BONE)] +
        # hollow eye sockets
        [m_quad_mesh("left_socket",
            [-0.14,0.98,0.052],[-0.08,0.98,0.052],
            [-0.14,1.06,0.052],[-0.08,1.06,0.052],
            c(0.08,0.06,0.12)),
         m_quad_mesh("right_socket",
            [ 0.08,0.98,0.052],[ 0.14,0.98,0.052],
            [ 0.08,1.06,0.052],[ 0.14,1.06,0.052],
            c(0.08,0.06,0.12))] +
        # glowing purple eyes (Drain attack)
        eye_pair(EYE_PURPLE, y=0.99, ox=0.11, ew=0.03, eh=0.06) +
        # crown band + spikes
        [m_box_mesh("crown_band", -0.19,1.17,-0.16, 0.19,1.22,0.06, GOLD)] +
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
"""
Generate procedural mesh RON files for all 54 player characters
from the portrait_prompt.md file.

Each character is built from archetype meshes (body, head, arms, legs)
with race-specific modifications and class-specific equipment.
Color palettes are matched to each character's description.
"""



# ---------------------------------------------------------------------------
# Shared cube helper (returns the 8-vertex box + 6-face index block)
# ---------------------------------------------------------------------------
def box(x0,y0,z0, x1,y1,z1):
    verts = [
        [x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1],
        [x0,y0,z0],[x1,y0,z0],[x1,y1,z0],[x0,y1,z0],
    ]
    idx = [0,1,2,2,3,0, 1,5,6,6,2,1, 7,6,5,5,4,7,
           4,0,3,3,7,4, 4,5,1,1,0,4, 3,2,6,6,7,3]
    return verts, idx

def tri(v0,v1,v2, normal=None):
    n = normal or [0.0,0.0,1.0]
    return [v0,v1,v2], [n,n,n], [0,1,2]

def quad_flat(v0,v1,v2,v3, normal=None):
    n = normal or [0.0,0.0,1.0]
    return [v0,v1,v2,v3],[n,n,n,n],[0,1,3,0,3,2]

# ---------------------------------------------------------------------------
# RON serialisation helpers
# ---------------------------------------------------------------------------
def fmt_v3_char(v):
    return f"({v[0]:.3f}, {v[1]:.3f}, {v[2]:.3f})"


def fmt_color_char(c):
    return f"({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f})"


def mesh_block(name, verts, indices, color,
               normals=None, uvs=None):
    lines = []
    lines.append("        (")
    lines.append(f'            name: Some("{name}"),')
    lines.append("            vertices: [")
    for v in verts:
        lines.append(f"                {fmt_v3_char(v)},")
    lines.append("            ],")
    if normals:
        lines.append("            normals: Some([")
        for n in normals:
            lines.append(f"                {fmt_v3_char(n)},")
        lines.append("            ]),")
    else:
        lines.append("            normals: None,")
    lines.append("            uvs: None,")
    lines.append("            indices: [")
    # print 12 per row
    for i in range(0, len(indices), 12):
        chunk = indices[i:i+12]
        lines.append("                " + ", ".join(str(x) for x in chunk) + ",")
    lines.append("            ],")
    lines.append(f"            color: {fmt_color_char(color)},")
    lines.append("        ),")
    return "\n".join(lines)

def write_ron(filename, name, scale, health, speed, meshes_text):
    body = f"""// {name}
CreatureDefinition(
    name: "{name}",
    scale: {scale:.2f},
    health: {health:.1f},
    speed: {speed:.1f},
    meshes: [
{meshes_text}
    ],
)
"""
    path = os.path.join(OUT, filename)
    with open(path, "w") as f:
        f.write(body)
    return path

# ---------------------------------------------------------------------------
# Common body-part factories
# ---------------------------------------------------------------------------
def legs(color, scale_y=1.0):
    blocks = []
    for side, x0, x1 in [("left",-0.15,-0.05),("right",0.05,0.15)]:
        v,i = box(x0, -1.0*scale_y, -0.10, x1, 0.0, 0.05)
        blocks.append(mesh_block(f"{side}_leg", v, i, color))
    return "\n".join(blocks)

def torso(color, w=0.27, h=0.80):
    v,i = box(-w, 0.0, -0.15, w, h, 0.12)
    return mesh_block("torso", v, i, color)

def arms(color, reach=0.48, w=0.09):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        x0 = sx*0.27; x1 = sx*reach
        if sx < 0: x0,x1 = x1,x0
        v,i = box(x0, 0.10, -0.10, x1, 0.65, 0.05)
        blocks.append(mesh_block(f"{side}_arm", v, i, color))
    return "\n".join(blocks)

def head(color, w=0.17, h=0.32):
    v,i = box(-w, 0.80, -0.17, w, 0.80+h, 0.07)
    return mesh_block("head", v, i, color)

def hair_cap(color, extra_y=0.04):
    v,i = box(-0.18, 1.10, -0.18, 0.18, 1.14+extra_y, 0.08)
    return mesh_block("hair", v, i, color)

def beard(color, length=0.18):
    verts,normals,idx = quad_flat(
        [-0.13, 0.82, 0.07],[0.13, 0.82, 0.07],
        [-0.13, 0.82-length, 0.05],[0.13, 0.82-length, 0.05],
        [0,0,1])
    return mesh_block("beard", verts, idx, color, normals)

def eyes(color):
    blocks = []
    for side, sx in [("left",-0.07),("right",0.07)]:
        v,i = box(sx-0.04, 0.99, 0.07, sx+0.04, 1.05, 0.09)
        blocks.append(mesh_block(f"{side}_eye", v, i, color))
    return "\n".join(blocks)

def hood(color, alpha=1.0):
    c = list(color[:3]) + [alpha]
    v,i = box(-0.20, 1.08, -0.20, 0.20, 1.30, 0.10)
    return mesh_block("hood", v, i, c)

def shoulder_pads(color):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        x0 = sx*0.27; x1 = sx*0.44
        if sx < 0: x0,x1 = x1,x0
        v,i = box(x0, 0.60, -0.12, x1, 0.82, 0.10)
        blocks.append(mesh_block(f"{side}_pauldron", v, i, color))
    return "\n".join(blocks)

def weapon_sword(color, side="right"):
    sx = 1 if side=="right" else -1
    x = sx*0.52
    v,i = box(x-0.03, -0.20, -0.06, x+0.03, 0.90, 0.00)
    return mesh_block("sword", v, i, color)

def weapon_axe(color, side="right"):
    sx = 1 if side=="right" else -1
    x = sx*0.54
    shaft_v,shaft_i = box(x-0.03, -0.20, -0.06, x+0.03, 0.75, 0.00)
    head_v,head_i = box(x-0.14, 0.65, -0.06, x+0.14, 0.82, 0.02)
    # offset head indices
    off = len(shaft_v)
    head_i_off = [idx+off for idx in head_i]
    return (mesh_block("axe_shaft", shaft_v, shaft_i, color) + "\n" +
            mesh_block("axe_head",  head_v,  head_i_off, color))

def weapon_bow(color):
    v,i = box(0.48, 0.0, -0.06, 0.52, 1.0, -0.02)
    return mesh_block("bow", v, i, color)

def weapon_staff(color, orb_color=None):
    orb_color = orb_color or [0.5,0.8,1.0,0.85]
    sv,si = box(0.52, 0.0, -0.06, 0.56, 1.75, -0.02)
    ov,oi_raw = box(0.44, 1.72, -0.10, 0.64, 1.92, 0.02)
    off = len(sv)
    oi = [x+off for x in oi_raw]
    return (mesh_block("staff", sv, si, color) + "\n" +
            mesh_block("staff_orb", ov, oi, orb_color))

def weapon_dagger(color, side="right"):
    sx = 1 if side=="right" else -1
    x = sx*0.52
    v,i = box(x-0.02, 0.0, -0.04, x+0.02, 0.38, 0.00)
    return mesh_block("dagger", v, i, color)

def quiver(color):
    v,i = box(-0.24, 0.30, -0.22, -0.10, 0.70, -0.14)
    return mesh_block("quiver", v, i, color)

def cloak(color, alpha=0.9):
    c = list(color[:3]) + [alpha]
    verts,normals,idx = quad_flat(
        [-0.30, 0.75, -0.16],[0.30, 0.75, -0.16],
        [-0.35, -0.20, -0.18],[0.35, -0.20, -0.18],
        [0,0,-1])
    return mesh_block("cloak", verts, idx, c, normals)

def horns_swept(color):
    """Two swept-back horns for tieflings/dragonborn."""
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        verts,normals,idx = tri(
            [sx*0.14, 1.18, -0.10],
            [sx*0.06, 1.12, -0.08],
            [sx*0.30, 1.38, -0.20],
            [0,1,0])
        blocks.append(mesh_block(f"{side}_horn", verts, idx, color, normals))
    return "\n".join(blocks)

def horns_ram(color):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        verts,normals,idx = tri(
            [sx*0.16, 1.14, -0.10],
            [sx*0.08, 1.10, -0.06],
            [sx*0.28, 1.05, -0.24],
            [0,1,0])
        blocks.append(mesh_block(f"{side}_horn", verts, idx, color, normals))
    return "\n".join(blocks)

def pointed_ears(color):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        verts,normals,idx = tri(
            [sx*0.17, 1.02, -0.08],
            [sx*0.17, 0.94, -0.08],
            [sx*0.28, 0.98, -0.10],
            [sx,0,0])
        blocks.append(mesh_block(f"{side}_ear", verts, idx, color, normals))
    return "\n".join(blocks)

def tusks(color):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        verts,normals,idx = tri(
            [sx*0.06, 0.80, 0.07],
            [sx*0.02, 0.80, 0.07],
            [sx*0.04, 0.72, 0.10],
            [0,-1,0])
        blocks.append(mesh_block(f"{side}_tusk", verts, idx, color, normals))
    return "\n".join(blocks)

def scales_overlay(color):
    """Decorative scale shimmer strips across torso."""
    blocks = []
    for i, y in enumerate([0.15, 0.35, 0.55]):
        verts,normals,idx = quad_flat(
            [-0.26, y, 0.13],[0.26, y, 0.13],
            [-0.26, y+0.06, 0.13],[0.26, y+0.06, 0.13],
            [0,0,1])
        blocks.append(mesh_block(f"scale_row_{i}", verts, idx, color, normals))
    return "\n".join(blocks)

def holy_symbol(color):
    cross_v,cross_i = box(-0.03, 0.45, 0.13, 0.03, 0.68, 0.15)
    hbar_v, hbar_i_raw = box(-0.09, 0.53, 0.13, 0.09, 0.60, 0.15)
    off = len(cross_v)
    hbar_i = [x+off for x in hbar_i_raw]
    return (mesh_block("holy_cross_v",  cross_v, cross_i, color) + "\n" +
            mesh_block("holy_cross_h",  hbar_v,  hbar_i,  color))

def aura_particles(color, alpha=0.55):
    c = list(color[:3]) + [alpha]
    blocks = []
    positions = [(-0.35,0.55,0.18),(0.30,0.70,0.18),(-0.10,1.10,0.16)]
    for k,(px,py,pz) in enumerate(positions):
        verts,normals,idx = tri(
            [px-0.06,py,pz],[px+0.06,py,pz],[px,py+0.12,pz],
            [0,0,1])
        blocks.append(mesh_block(f"aura_{k}", verts, idx, c, normals))
    return "\n".join(blocks)

def warpaint(color):
    """Simple stripe warpaint across face."""
    verts,normals,idx = quad_flat(
        [-0.15, 0.96, 0.07],[0.15, 0.96, 0.07],
        [-0.15, 1.00, 0.07],[0.15, 1.00, 0.07],
        [0,0,1])
    return mesh_block("warpaint", verts, idx, color, normals)

def fur_mantle(color):
    v,i = box(-0.35, 0.60, -0.16, 0.35, 0.82, 0.16)
    return mesh_block("fur_mantle", v, i, color)

def crown_or_circlet(color):
    v,i = box(-0.19, 1.12, -0.18, 0.19, 1.17, 0.08)
    return mesh_block("circlet", v, i, color)

def spellbook(color):
    v,i = box(-0.58,-0.05,-0.08, -0.42, 0.20, 0.08)
    return mesh_block("spellbook", v, i, color)

def goggles(color):
    v,i = box(-0.14, 1.07, 0.07, 0.14, 1.14, 0.11)
    return mesh_block("goggles", v, i, color)

def wand(color, orb_color=None):
    orb_color = orb_color or [0.2,0.1,0.05,1.0]
    wv,wi = box(0.50,-0.05,-0.04, 0.54, 0.50, 0.00)
    ov,oi_raw = box(0.46, 0.48,-0.06, 0.58, 0.60, 0.04)
    off = len(wv)
    oi = [x+off for x in oi_raw]
    return (mesh_block("wand",     wv, wi, color) + "\n" +
            mesh_block("wand_orb", ov, oi, orb_color))

def spectacles(color):
    v,i = box(-0.14, 0.98, 0.07, 0.14, 1.04, 0.10)
    return mesh_block("spectacles", v, i, color)

def antler_crown(color):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        shaft_v,shaft_i = box(sx*0.06, 1.14, -0.10, sx*0.10, 1.32, -0.06)
        tine_v,tine_i_raw = box(sx*0.08, 1.26, -0.12, sx*0.18, 1.30, -0.08)
        off = len(shaft_v)
        tine_i = [x+off for x in tine_i_raw]
        blocks.append(mesh_block(f"{side}_antler_shaft", shaft_v, shaft_i, color))
        blocks.append(mesh_block(f"{side}_antler_tine",  tine_v,  tine_i,  color))
    return "\n".join(blocks)

def tattoo_marks(color):
    verts,normals,idx = quad_flat(
        [-0.12, 0.85, 0.07],[0.12, 0.85, 0.07],
        [-0.10, 0.95, 0.07],[0.10, 0.95, 0.07],
        [0,0,1])
    return mesh_block("tattoos", verts, idx, color, normals)

def pigtails(color):
    blocks = []
    for side, sx in [("left",-1),("right",1)]:
        v,i = box(sx*0.16, 0.90, -0.14, sx*0.22, 1.12, -0.08)
        blocks.append(mesh_block(f"{side}_pigtail", v, i, color))
    return "\n".join(blocks)

def wizard_hat(color):
    brim_v,brim_i = box(-0.24, 1.12, -0.24, 0.24, 1.17, 0.12)
    cone_verts = [
        [-0.20,1.17,-0.20],[0.20,1.17,-0.20],
        [0.20,1.17, 0.08],[-0.20,1.17, 0.08],
        [0.00,1.60,-0.06],
    ]
    cone_idx = [0,3,2, 0,2,1, 0,1,4, 1,2,4, 2,3,4, 3,0,4]
    return (mesh_block("hat_brim", brim_v, brim_i, color) + "\n" +
            mesh_block("hat_cone", cone_verts, cone_idx, color))

def belt_and_pouches(color):
    belt_v,belt_i = box(-0.28, -0.02, -0.01, 0.28, 0.05, 0.13)
    p1v,p1i_raw = box( 0.14,-0.14,-0.01,  0.26,-0.01,0.12)
    p2v,p2i_raw = box(-0.26,-0.14,-0.01, -0.14,-0.01,0.12)
    off1 = len(belt_v); off2 = off1 + len(p1v)
    p1i = [x+off1 for x in p1i_raw]
    p2i = [x+off2 for x in p2i_raw]
    return (mesh_block("belt",    belt_v, belt_i, color) + "\n" +
            mesh_block("pouch_r", p1v, p1i, color) + "\n" +
            mesh_block("pouch_l", p2v, p2i, color))

# ---------------------------------------------------------------------------
# Colour palettes
# ---------------------------------------------------------------------------
C = {
    # skin tones
    "skin_fair":     [0.90, 0.80, 0.74, 1.0],
    "skin_tan":      [0.78, 0.63, 0.50, 1.0],
    "skin_dark":     [0.40, 0.28, 0.20, 1.0],
    "skin_olive":    [0.65, 0.58, 0.42, 1.0],
    "skin_orc":      [0.36, 0.50, 0.28, 1.0],
    "skin_orc_grey": [0.42, 0.52, 0.40, 1.0],
    "skin_tiefling_crimson": [0.72, 0.18, 0.15, 1.0],
    "skin_tiefling_indigo":  [0.28, 0.20, 0.55, 1.0],
    "skin_tiefling_blue":    [0.15, 0.22, 0.55, 1.0],
    "skin_halfling": [0.85, 0.74, 0.62, 1.0],
    "skin_gnome":    [0.82, 0.72, 0.60, 1.0],
    "skin_gnome_tan":[0.72, 0.60, 0.45, 1.0],
    "skin_dwarf":    [0.80, 0.65, 0.55, 1.0],
    "skin_elf":      [0.92, 0.86, 0.80, 1.0],
    "skin_elf_pale": [0.96, 0.92, 0.88, 1.0],
    "scales_bronze": [0.72, 0.52, 0.25, 1.0],
    "scales_copper": [0.75, 0.45, 0.20, 1.0],
    # hair
    "hair_red":      [0.70, 0.20, 0.10, 1.0],
    "hair_auburn":   [0.55, 0.25, 0.12, 1.0],
    "hair_orange":   [0.82, 0.40, 0.08, 1.0],
    "hair_blonde":   [0.85, 0.75, 0.35, 1.0],
    "hair_silver":   [0.85, 0.85, 0.90, 1.0],
    "hair_platinum": [0.95, 0.95, 0.98, 1.0],
    "hair_black":    [0.12, 0.10, 0.10, 1.0],
    "hair_dark":     [0.22, 0.17, 0.13, 1.0],
    "hair_brown":    [0.42, 0.28, 0.16, 1.0],
    "hair_chestnut": [0.50, 0.28, 0.14, 1.0],
    "hair_ginger":   [0.78, 0.38, 0.12, 1.0],
    "hair_white":    [0.96, 0.96, 0.96, 1.0],
    "hair_pink":     [0.92, 0.55, 0.70, 1.0],
    "hair_purple":   [0.42, 0.20, 0.55, 1.0],
    # armour / cloth
    "plate_steel":   [0.72, 0.74, 0.76, 1.0],
    "plate_gold":    [0.82, 0.72, 0.28, 1.0],
    "plate_mithril": [0.80, 0.88, 0.95, 1.0],
    "leather_dark":  [0.28, 0.22, 0.16, 1.0],
    "leather_brown": [0.48, 0.35, 0.22, 1.0],
    "leather_green": [0.28, 0.42, 0.22, 1.0],
    "leather_black": [0.14, 0.12, 0.12, 1.0],
    "chainmail":     [0.58, 0.60, 0.62, 1.0],
    "fur_brown":     [0.50, 0.38, 0.24, 1.0],
    "fur_white":     [0.88, 0.85, 0.80, 1.0],
    "cloth_crimson": [0.72, 0.12, 0.12, 1.0],
    "cloth_gold":    [0.75, 0.62, 0.18, 1.0],
    "cloth_blue":    [0.18, 0.28, 0.62, 1.0],
    "cloth_deepblue":[0.12, 0.18, 0.48, 1.0],
    "cloth_violet":  [0.38, 0.14, 0.58, 1.0],
    "cloth_dark":    [0.14, 0.12, 0.18, 1.0],
    "cloth_white":   [0.94, 0.94, 0.96, 1.0],
    "cloth_green":   [0.22, 0.48, 0.24, 1.0],
    "cloth_dawn":    [0.88, 0.68, 0.55, 1.0],
    "cloth_silver":  [0.76, 0.78, 0.82, 1.0],
    "robe_patchwork":[0.42, 0.32, 0.48, 1.0],
    # weapons / accessories
    "iron":          [0.52, 0.52, 0.55, 1.0],
    "gold_metal":    [0.88, 0.78, 0.30, 1.0],
    "wood_dark":     [0.32, 0.22, 0.12, 1.0],
    "wood_light":    [0.62, 0.48, 0.30, 1.0],
    "horn_dark":     [0.22, 0.18, 0.14, 1.0],
    "obsidian":      [0.12, 0.10, 0.14, 1.0],
    "orb_blue":      [0.50, 0.78, 1.00, 0.85],
    "orb_purple":    [0.65, 0.35, 0.90, 0.85],
    "orb_green":     [0.35, 0.85, 0.45, 0.85],
    "glowing_white": [1.00, 1.00, 0.90, 1.0],
    "glowing_gold":  [1.00, 0.88, 0.40, 1.0],
    "glowing_red":   [1.00, 0.30, 0.20, 1.0],
    "glowing_purple":[0.78, 0.40, 1.00, 1.0],
    "tusk_ivory":    [0.95, 0.90, 0.80, 1.0],
    "bone_white":    [0.92, 0.90, 0.84, 1.0],
    "shadow":        [0.10, 0.08, 0.14, 0.80],
}

# ---------------------------------------------------------------------------
# Character definitions – (filename, display_name, scale, hp, speed, mesh_fn)
# ---------------------------------------------------------------------------
def build(parts):
    return "\n".join(p for p in parts if p)

# ── FEMALE DRAGONBORN SORCERER ───────────────────────────────────────────
def female_dragonborn_sorcerer():
    p = build([
        legs(C["scales_bronze"]),
        torso(C["cloth_crimson"]),
        scales_overlay([0.82,0.62,0.30,0.70]),
        arms(C["scales_bronze"]),
        head(C["scales_bronze"]),
        eyes(C["glowing_red"]),
        horns_swept(C["horn_dark"]),
        weapon_staff(C["wood_dark"], C["orb_purple"]),
        aura_particles([0.80,0.30,0.10]),
    ])
    return "female_dragonborn_sorcerer.ron", "FemaleDragonbornSorcerer", 0.98, 85, 5.5, p

# ── FEMALE DWARF BARBARIAN ───────────────────────────────────────────────
def female_dwarf_barbarian():
    p = build([
        legs(C["fur_brown"]),
        torso(C["chainmail"]),
        fur_mantle(C["fur_brown"]),
        arms(C["chainmail"]),
        head(C["skin_dwarf"]),
        hair_cap(C["hair_red"]),
        beard(C["hair_red"], 0.10),
        eyes([0.25,0.18,0.12,1.0]),
        warpaint([0.55,0.12,0.08,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "female_dwarf_barbarian.ron", "FemaleDwarfBarbarian", 0.88, 110, 4.5, p

# ── FEMALE DWARF PALADIN ─────────────────────────────────────────────────
def female_dwarf_paladin():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads(C["plate_gold"]),
        arms(C["plate_steel"]),
        head(C["skin_dwarf"]),
        hair_cap(C["hair_brown"]),
        eyes([0.30,0.22,0.16,1.0]),
        holy_symbol(C["gold_metal"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "female_dwarf_paladin.ron", "FemaleDwarfPaladin", 0.88, 100, 3.8, p

# ── FEMALE ELF ARCHER (platinum) ─────────────────────────────────────────
def female_elf_archer_platinum():
    p = build([
        legs(C["leather_green"]),
        torso(C["leather_green"]),
        arms(C["leather_green"]),
        head(C["skin_elf_pale"]),
        hair_cap(C["hair_platinum"]),
        pointed_ears(C["skin_elf_pale"]),
        eyes([0.20,0.65,0.28,1.0]),
        weapon_bow(C["wood_light"]),
        quiver(C["leather_brown"]),
    ])
    return "female_elf_archer_platinum.ron", "FemaleElfArcherPlatinum", 1.0, 75, 7.0, p

# ── FEMALE ELF ARCHER (brown hair) ───────────────────────────────────────
def female_elf_archer_brown():
    p = build([
        legs(C["leather_brown"]),
        torso(C["leather_brown"]),
        arms(C["leather_brown"]),
        head(C["skin_elf"]),
        hair_cap(C["hair_brown"]),
        pointed_ears(C["skin_elf"]),
        eyes([0.30,0.50,0.28,1.0]),
        weapon_bow(C["wood_dark"]),
        quiver(C["leather_dark"]),
    ])
    return "female_elf_archer_brown.ron", "FemaleElfArcherBrown", 1.0, 75, 7.0, p

# ── FEMALE ELF ASSASSIN ───────────────────────────────────────────────────
def female_elf_assassin():
    p = build([
        legs(C["leather_black"]),
        torso(C["leather_black"]),
        arms(C["leather_black"]),
        head(C["skin_orc_grey"]),
        hood(C["leather_black"], 0.92),
        tusks(C["tusk_ivory"]),
        eyes([0.85,0.85,0.85,1.0]),
        weapon_dagger(C["iron"]),
        cloak(C["leather_black"]),
    ])
    return "female_elf_assassin.ron", "FemaleElfAssassin", 1.0, 80, 8.0, p

# ── FEMALE ELF PALADIN ────────────────────────────────────────────────────
def female_elf_paladin():
    p = build([
        legs(C["plate_mithril"]),
        torso(C["plate_mithril"]),
        shoulder_pads(C["plate_mithril"]),
        arms(C["plate_mithril"]),
        head(C["skin_elf_pale"]),
        hair_cap(C["hair_blonde"]),
        pointed_ears(C["skin_elf_pale"]),
        eyes([0.20,0.50,0.85,1.0]),
        crown_or_circlet(C["gold_metal"]),
        holy_symbol(C["glowing_gold"]),
        weapon_sword(C["glowing_white"]),
        aura_particles(C["glowing_gold"], 0.45),
    ])
    return "female_elf_paladin.ron", "FemaleElfPaladin", 1.0, 90, 5.5, p

# ── FEMALE ELF SORCERER ───────────────────────────────────────────────────
def female_elf_sorcerer():
    p = build([
        legs(C["cloth_violet"]),
        torso(C["cloth_violet"]),
        arms(C["cloth_violet"]),
        head(C["skin_elf_pale"]),
        hair_cap(C["hair_silver"]),
        pointed_ears(C["skin_elf_pale"]),
        eyes(C["glowing_purple"]),
        weapon_staff(C["wood_dark"], C["orb_purple"]),
        aura_particles([0.65,0.30,0.90], 0.50),
    ])
    return "female_elf_sorcerer.ron", "FemaleElfSorcerer", 1.0, 70, 5.5, p

# ── FEMALE GNOME DRUID ────────────────────────────────────────────────────
def female_gnome_druid():
    p = build([
        legs(C["cloth_green"]),
        torso(C["cloth_green"]),
        arms(C["leather_brown"]),
        head(C["skin_gnome"]),
        hair_cap([0.55,0.38,0.72,1.0]),   # colorful wild hair
        eyes([0.35,0.70,0.45,1.0]),
        antler_crown(C["wood_dark"]),
        weapon_staff(C["wood_light"], C["orb_green"]),
        aura_particles([0.35,0.75,0.35], 0.40),
    ])
    return "female_gnome_druid.ron", "FemaleGnomeDruid", 0.78, 65, 5.0, p

# ── FEMALE GNOME ROGUE (goggles) ─────────────────────────────────────────
def female_gnome_rogue_goggles():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        arms(C["leather_dark"]),
        head(C["skin_gnome"]),
        hair_cap(C["hair_brown"]),
        eyes([0.62,0.42,0.20,1.0]),
        goggles([0.30,0.30,0.30,1.0]),
        belt_and_pouches(C["leather_dark"]),
        weapon_dagger(C["iron"]),
    ])
    return "female_gnome_rogue_goggles.ron", "FemaleGnomeRogueGoggles", 0.78, 70, 8.5, p

# ── FEMALE GNOME ROGUE (auburn hood) ─────────────────────────────────────
def female_gnome_rogue_auburn():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        arms(C["leather_dark"]),
        head(C["skin_gnome"]),
        hood(C["leather_dark"], 0.95),
        hair_cap(C["hair_auburn"]),
        eyes([0.32,0.62,0.28,1.0]),
        belt_and_pouches(C["leather_brown"]),
        weapon_dagger(C["iron"]),
    ])
    return "female_gnome_rogue_auburn.ron", "FemaleGnomeRogueAuburn", 0.78, 70, 8.5, p

# ── FEMALE GNOME WARLOCK ─────────────────────────────────────────────────
def female_gnome_warlock():
    p = build([
        legs(C["robe_patchwork"]),
        torso(C["robe_patchwork"]),
        arms(C["robe_patchwork"]),
        head(C["skin_gnome_tan"]),
        pigtails(C["hair_pink"]),
        eyes([0.80,0.40,0.70,1.0]),
        wand([0.12,0.08,0.08,1.0], [0.14,0.08,0.06,1.0]),
        aura_particles([0.20,0.10,0.30], 0.65),
    ])
    return "female_gnome_warlock.ron", "FemaleGnomeWarlock", 0.78, 65, 5.5, p

# ── FEMALE HALF-ELF ARCHER ───────────────────────────────────────────────
def female_halfelf_archer():
    p = build([
        legs(C["leather_brown"]),
        torso(C["leather_brown"]),
        arms(C["leather_brown"]),
        head(C["skin_tan"]),
        hair_cap(C["hair_red"]),
        pointed_ears(C["skin_tan"]),
        eyes([0.35,0.48,0.62,1.0]),
        weapon_bow(C["wood_dark"]),
        quiver(C["leather_dark"]),
    ])
    return "female_halfelf_archer.ron", "FemaleHalfElfArcher", 1.0, 75, 7.0, p

# ── FEMALE HALF-ELF ROGUE ────────────────────────────────────────────────
def female_halfelf_rogue():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        cloak(C["cloth_dark"], 0.88),
        arms(C["leather_dark"]),
        head(C["skin_tan"]),
        hair_cap(C["hair_chestnut"]),
        pointed_ears(C["skin_tan"]),
        eyes([0.52,0.42,0.22,1.0]),
        weapon_dagger(C["iron"], "left"),
        weapon_dagger(C["iron"], "right"),
    ])
    return "female_halfelf_rogue.ron", "FemaleHalfElfRogue", 1.0, 78, 8.2, p

# ── FEMALE HALF-ELF WARLOCK ──────────────────────────────────────────────
def female_halfelf_warlock():
    p = build([
        legs(C["cloth_dark"]),
        torso(C["cloth_dark"]),
        arms(C["cloth_dark"]),
        head(C["skin_tiefling_blue"]),
        hair_cap(C["hair_purple"]),
        pointed_ears(C["skin_tiefling_blue"]),
        horns_swept([0.20,0.16,0.24,1.0]),
        eyes([0.85,0.60,0.20,1.0]),
        aura_particles([0.40,0.18,0.60], 0.60),
    ])
    return "female_halfelf_warlock.ron", "FemaleHalfElfWarlock", 1.0, 72, 5.5, p

# ── FEMALE HALFLING MAGE ─────────────────────────────────────────────────
def female_halfling_mage():
    p = build([
        legs(C["cloth_deepblue"]),
        torso(C["cloth_deepblue"]),
        arms(C["cloth_deepblue"]),
        head(C["skin_halfling"]),
        hair_cap(C["hair_chestnut"]),
        eyes([0.55,0.42,0.18,1.0]),
        spellbook(C["leather_brown"]),
        weapon_staff(C["wood_dark"], C["orb_blue"]),
        aura_particles([0.60,0.75,1.00], 0.40),
    ])
    return "female_halfling_mage.ron", "FemaleHalflingMage", 0.80, 65, 5.0, p

# ── FEMALE HUMAN BARBARIAN (auburn) ──────────────────────────────────────
def female_human_barbarian_auburn():
    p = build([
        legs(C["leather_brown"]),
        torso(C["fur_brown"]),
        fur_mantle(C["fur_white"]),
        arms(C["skin_fair"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_auburn"]),
        eyes([0.40,0.32,0.22,1.0]),
        warpaint([0.65,0.15,0.10,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "female_human_barbarian_auburn.ron", "FemaleHumanBarbarianAuburn", 1.0, 110, 5.5, p

# ── FEMALE HUMAN BARBARIAN (dark) ────────────────────────────────────────
def female_human_barbarian_dark():
    p = build([
        legs(C["leather_dark"]),
        torso(C["fur_brown"]),
        fur_mantle(C["fur_brown"]),
        arms(C["skin_dark"]),
        head(C["skin_dark"]),
        hair_cap(C["hair_orange"]),
        eyes([0.25,0.50,0.22,1.0]),
        warpaint([0.75,0.50,0.10,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "female_human_barbarian_dark.ron", "FemaleHumanBarbarianDark", 1.0, 110, 5.5, p

# ── FEMALE HUMAN CLERIC ───────────────────────────────────────────────────
def female_human_cleric():
    p = build([
        legs(C["cloth_white"]),
        torso(C["cloth_white"]),
        arms(C["cloth_white"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_blonde"]),
        eyes([0.35,0.50,0.72,1.0]),
        crown_or_circlet(C["gold_metal"]),
        holy_symbol(C["glowing_gold"]),
        aura_particles(C["glowing_gold"], 0.35),
    ])
    return "female_human_cleric.ron", "FemaleHumanCleric", 1.0, 80, 4.5, p

# ── FEMALE HUMAN KNIGHT (v1) ─────────────────────────────────────────────
def female_human_knight_v1():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads(C["plate_steel"]),
        arms(C["plate_steel"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_brown"]),
        eyes([0.35,0.42,0.52,1.0]),
        weapon_sword(C["iron"]),
    ])
    return "female_human_knight_v1.ron", "FemaleHumanKnightV1", 1.0, 100, 4.5, p

# ── FEMALE HUMAN KNIGHT (v2 ponytail) ────────────────────────────────────
def female_human_knight_v2():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads([0.68,0.70,0.72,1.0]),
        arms(C["plate_steel"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_brown"]),
        eyes([0.38,0.48,0.60,1.0]),
        weapon_sword(C["iron"]),
    ])
    return "female_human_knight_v2.ron", "FemaleHumanKnightV2", 1.0, 100, 4.5, p

# ── FEMALE HUMAN PALADIN ─────────────────────────────────────────────────
def female_human_paladin():
    p = build([
        legs(C["plate_gold"]),
        torso(C["plate_gold"]),
        shoulder_pads(C["plate_gold"]),
        arms(C["plate_gold"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_blonde"]),
        eyes([0.25,0.48,0.78,1.0]),
        crown_or_circlet(C["gold_metal"]),
        holy_symbol(C["glowing_gold"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "female_human_paladin.ron", "FemaleHumanPaladin", 1.0, 95, 4.5, p

# ── FEMALE HUMAN WARLOCK ─────────────────────────────────────────────────
def female_human_warlock():
    p = build([
        legs(C["cloth_dark"]),
        torso(C["cloth_dark"]),
        cloak(C["shadow"]),
        arms(C["cloth_dark"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_black"]),
        eyes(C["glowing_purple"]),
        tattoo_marks([0.22,0.14,0.40,1.0]),
        aura_particles([0.40,0.10,0.60], 0.55),
    ])
    return "female_human_warlock.ron", "FemaleHumanWarlock", 1.0, 72, 5.5, p

# ── FEMALE ORC ARCHER (olive) ────────────────────────────────────────────
def female_orc_archer_olive():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        arms(C["skin_orc"]),
        head(C["skin_orc"]),
        hair_cap(C["hair_black"]),
        tusks(C["tusk_ivory"]),
        eyes([0.72,0.52,0.12,1.0]),
        weapon_bow(C["wood_dark"]),
        quiver(C["leather_dark"]),
    ])
    return "female_orc_archer_olive.ron", "FemaleOrcArcherOlive", 1.05, 80, 6.5, p

# ── FEMALE ORC ARCHER (green) ────────────────────────────────────────────
def female_orc_archer_green():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        arms(C["skin_orc"]),
        head(C["skin_orc"]),
        hair_cap(C["hair_dark"]),
        tusks(C["tusk_ivory"]),
        eyes([0.55,0.45,0.15,1.0]),
        weapon_bow(C["wood_dark"]),
        quiver(C["leather_dark"]),
    ])
    return "female_orc_archer_green.ron", "FemaleOrcArcherGreen", 1.05, 80, 6.5, p

# ── FEMALE ORC ASSASSIN ───────────────────────────────────────────────────
def female_orc_assassin():
    p = build([
        legs(C["leather_black"]),
        torso(C["leather_black"]),
        cloak(C["leather_black"]),
        arms(C["skin_orc_grey"]),
        head(C["skin_orc_grey"]),
        hood(C["leather_black"]),
        tusks(C["tusk_ivory"]),
        eyes([0.80,0.80,0.78,1.0]),
        weapon_dagger(C["iron"]),
    ])
    return "female_orc_assassin.ron", "FemaleOrcAssassin", 1.05, 82, 8.0, p

# ── FEMALE ORC BARBARIAN ──────────────────────────────────────────────────
def female_orc_barbarian():
    p = build([
        legs(C["fur_brown"]),
        torso(C["fur_brown"]),
        fur_mantle(C["fur_brown"]),
        arms(C["skin_orc"]),
        head(C["skin_orc"]),
        hair_cap(C["hair_dark"]),
        tusks(C["tusk_ivory"]),
        eyes([0.82,0.78,0.18,1.0]),
        warpaint([0.72,0.40,0.08,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "female_orc_barbarian.ron", "FemaleOrcBarbarian", 1.05, 115, 5.0, p

# ── FEMALE ORC PALADIN ────────────────────────────────────────────────────
def female_orc_paladin():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads(C["plate_steel"]),
        arms(C["plate_steel"]),
        head(C["skin_orc_grey"]),
        hair_cap(C["hair_black"]),
        tusks(C["tusk_ivory"]),
        eyes([0.38,0.35,0.30,1.0]),
        holy_symbol(C["glowing_gold"]),
        weapon_axe(C["glowing_white"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "female_orc_paladin.ron", "FemaleOrcPaladin", 1.10, 105, 4.0, p

# ── FEMALE TIEFLING BARBARIAN ─────────────────────────────────────────────
def female_tiefling_barbarian():
    p = build([
        legs(C["leather_dark"]),
        torso(C["cloth_dark"]),
        shoulder_pads([0.28,0.18,0.18,1.0]),
        fur_mantle(C["hair_black"]),
        arms(C["skin_tiefling_crimson"]),
        head(C["skin_tiefling_crimson"]),
        hair_cap(C["hair_red"]),
        horns_swept(C["obsidian"]),
        eyes([0.72,0.18,0.12,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "female_tiefling_barbarian.ron", "FemaleTieflingBarbarian", 1.0, 110, 5.5, p

# ── FEMALE TIEFLING CLERIC ────────────────────────────────────────────────
def female_tiefling_cleric():
    p = build([
        legs(C["cloth_dawn"]),
        torso(C["cloth_dawn"]),
        arms(C["cloth_dawn"]),
        head(C["skin_tiefling_indigo"]),
        hair_cap(C["hair_silver"]),
        horns_ram([0.25,0.20,0.28,1.0]),
        eyes([0.70,0.38,0.82,1.0]),
        holy_symbol(C["glowing_white"]),
        aura_particles([0.90,0.90,1.00], 0.40),
    ])
    return "female_tiefling_cleric.ron", "FemaleTieflingCleric", 1.0, 80, 4.5, p

# ════════════════════════════════════════════════════════════════════════════
# MALE CHARACTERS
# ════════════════════════════════════════════════════════════════════════════

def male_dragonborn_sorcerer():
    p = build([
        legs(C["scales_copper"]),
        torso(C["cloth_crimson"]),
        scales_overlay([0.80,0.50,0.22,0.65]),
        arms(C["scales_copper"]),
        head(C["scales_copper"]),
        eyes(C["glowing_red"]),
        horns_swept(C["horn_dark"]),
        weapon_staff(C["wood_dark"], C["orb_purple"]),
        aura_particles([0.85,0.35,0.10], 0.55),
    ])
    return "male_dragonborn_sorcerer.ron", "MaleDragonbornSorcerer", 1.05, 85, 5.5, p

def male_dwarf_barbarian():
    p = build([
        legs(C["chainmail"]),
        torso(C["chainmail"]),
        fur_mantle(C["fur_brown"]),
        arms(C["chainmail"]),
        head(C["skin_dwarf"]),
        beard(C["hair_red"], 0.28),
        eyes([0.38,0.28,0.18,1.0]),
        warpaint([0.55,0.10,0.08,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "male_dwarf_barbarian.ron", "MaleDwarfBarbarian", 0.88, 115, 4.2, p

def male_dwarf_cleric():
    p = build([
        legs(C["cloth_white"]),
        torso(C["cloth_white"]),
        arms(C["cloth_white"]),
        head(C["skin_dwarf"]),
        beard(C["hair_white"], 0.30),
        eyes([0.35,0.35,0.38,1.0]),
        crown_or_circlet(C["gold_metal"]),
        holy_symbol(C["glowing_gold"]),
        weapon_staff(C["wood_dark"], C["glowing_gold"]),
        aura_particles(C["glowing_gold"], 0.35),
    ])
    return "male_dwarf_cleric.ron", "MaleDwarfCleric", 0.88, 85, 3.5, p

def male_dwarf_knight():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads(C["plate_steel"]),
        arms(C["plate_steel"]),
        head(C["skin_dwarf"]),
        beard([0.38,0.28,0.18,1.0], 0.22),
        eyes([0.30,0.28,0.24,1.0]),
        weapon_sword(C["iron"]),
    ])
    return "male_dwarf_knight.ron", "MaleDwarfKnight", 0.88, 105, 3.8, p

def male_dwarf_paladin():
    p = build([
        legs(C["plate_gold"]),
        torso(C["plate_gold"]),
        shoulder_pads(C["plate_gold"]),
        arms(C["plate_gold"]),
        head(C["skin_dwarf"]),
        beard(C["hair_brown"], 0.32),
        eyes([0.32,0.28,0.22,1.0]),
        holy_symbol(C["glowing_gold"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "male_dwarf_paladin.ron", "MaleDwarfPaladin", 0.88, 100, 3.8, p

def male_elf_archer_blonde():
    p = build([
        legs(C["leather_green"]),
        torso(C["leather_green"]),
        arms(C["leather_green"]),
        head(C["skin_elf_pale"]),
        hair_cap(C["hair_blonde"]),
        pointed_ears(C["skin_elf_pale"]),
        eyes([0.25,0.55,0.30,1.0]),
        weapon_bow(C["wood_light"]),
        quiver(C["leather_brown"]),
    ])
    return "male_elf_archer_blonde.ron", "MaleElfArcherBlonde", 1.0, 75, 7.2, p

def male_elf_archer_silver():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        arms(C["leather_dark"]),
        head(C["skin_elf_pale"]),
        hair_cap(C["hair_silver"]),
        pointed_ears(C["skin_elf_pale"]),
        eyes([0.28,0.58,0.30,1.0]),
        weapon_bow(C["wood_light"]),
        quiver(C["leather_brown"]),
    ])
    return "male_elf_archer_silver.ron", "MaleElfArcherSilver", 1.0, 75, 7.2, p

def male_elf_assassin():
    p = build([
        legs(C["leather_black"]),
        torso(C["leather_black"]),
        cloak(C["cloth_dark"]),
        arms(C["leather_black"]),
        head(C["skin_elf"]),
        hood(C["cloth_dark"]),
        hair_cap(C["hair_dark"]),
        pointed_ears(C["skin_elf"]),
        eyes([0.22,0.22,0.28,1.0]),
        weapon_dagger(C["iron"]),
    ])
    return "male_elf_assassin.ron", "MaleElfAssassin", 1.0, 80, 8.5, p

def male_elf_druid():
    p = build([
        legs(C["cloth_green"]),
        torso(C["leather_brown"]),
        arms(C["cloth_green"]),
        head(C["skin_elf"]),
        hair_cap(C["hair_brown"]),
        pointed_ears(C["skin_elf"]),
        eyes([0.30,0.55,0.32,1.0]),
        tattoo_marks([0.18,0.45,0.22,1.0]),
        antler_crown(C["wood_dark"]),
        weapon_staff(C["wood_light"], C["orb_green"]),
    ])
    return "male_elf_druid.ron", "MaleElfDruid", 1.0, 72, 5.5, p

def male_elf_paladin():
    p = build([
        legs(C["plate_gold"]),
        torso(C["plate_gold"]),
        shoulder_pads(C["plate_gold"]),
        arms(C["plate_gold"]),
        cloak(C["cloth_white"]),
        head(C["skin_elf_pale"]),
        hair_cap(C["hair_silver"]),
        pointed_ears(C["skin_elf_pale"]),
        eyes([0.35,0.55,0.72,1.0]),
        holy_symbol(C["glowing_gold"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "male_elf_paladin.ron", "MaleElfPaladin", 1.0, 90, 5.5, p

def male_gnome_rogue():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        arms(C["leather_dark"]),
        head(C["skin_gnome"]),
        hair_cap(C["hair_ginger"]),
        beard(C["hair_ginger"], 0.12),
        eyes([0.28,0.48,0.72,1.0]),
        belt_and_pouches(C["leather_dark"]),
        weapon_dagger(C["iron"], "right"),
        weapon_dagger(C["iron"], "left"),
    ])
    return "male_gnome_rogue.ron", "MaleGnomeRogue", 0.78, 72, 8.5, p

def male_gnome_sorcerer():
    p = build([
        legs(C["cloth_violet"]),
        torso(C["cloth_violet"]),
        arms(C["cloth_violet"]),
        head(C["skin_gnome"]),
        beard([0.38,0.30,0.22,1.0], 0.16),
        eyes([0.55,0.42,0.20,1.0]),
        wizard_hat(C["cloth_deepblue"]),
        weapon_staff(C["wood_dark"], C["orb_blue"]),
        aura_particles(C["orb_blue"][:3], 0.50),
    ])
    return "male_gnome_sorcerer.ron", "MaleGnomeSorcerer", 0.78, 68, 5.0, p

def male_gnome_warlock_spiked():
    p = build([
        legs(C["cloth_dark"]),
        torso(C["cloth_dark"]),
        arms(C["cloth_dark"]),
        head(C["skin_gnome"]),
        hair_cap(C["hair_black"]),
        eyes([0.32,0.82,0.32,1.0]),
        horns_swept([0.18,0.12,0.20,1.0]),
        wand([0.14,0.10,0.08,1.0], C["orb_green"]),
        aura_particles([0.20,0.70,0.25], 0.60),
    ])
    return "male_gnome_warlock_spiked.ron", "MaleGnomeWarlockSpiked", 0.78, 65, 5.5, p

def male_gnome_warlock_spectacles():
    p = build([
        legs(C["cloth_violet"]),
        torso(C["cloth_violet"]),
        arms(C["cloth_violet"]),
        head(C["skin_gnome"]),
        hair_cap(C["hair_white"]),
        beard(C["hair_white"], 0.14),
        eyes([0.55,0.40,0.72,1.0]),
        spectacles([0.30,0.28,0.32,1.0]),
        tattoo_marks([0.55,0.30,0.72,0.70]),
        aura_particles([0.55,0.28,0.80], 0.55),
    ])
    return "male_gnome_warlock_spectacles.ron", "MaleGnomeWarlockSpectacles", 0.78, 65, 5.5, p

def male_halfelf_rogue():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        cloak(C["cloth_green"]),
        arms(C["leather_dark"]),
        head(C["skin_tan"]),
        hair_cap(C["hair_dark"]),
        pointed_ears(C["skin_tan"]),
        eyes([0.62,0.48,0.22,1.0]),
        weapon_dagger(C["iron"], "right"),
        weapon_dagger(C["iron"], "left"),
    ])
    return "male_halfelf_rogue.ron", "MaleHalfElfRogue", 1.0, 78, 8.5, p

def male_halfelf_warlock():
    p = build([
        legs(C["cloth_dark"]),
        torso(C["cloth_dark"]),
        cloak([0.12,0.10,0.18,0.85]),
        arms(C["cloth_dark"]),
        head(C["skin_fair"]),
        hair_cap(C["hair_black"]),
        pointed_ears(C["skin_fair"]),
        eyes(C["glowing_purple"]),
        tattoo_marks([0.18,0.10,0.35,0.80]),
        weapon_dagger([0.35,0.28,0.55,1.0]),
        aura_particles([0.45,0.20,0.70], 0.60),
    ])
    return "male_halfelf_warlock.ron", "MaleHalfElfWarlock", 1.0, 72, 5.5, p

def male_halfling_mage():
    p = build([
        legs(C["cloth_violet"]),
        torso(C["cloth_violet"]),
        arms(C["cloth_violet"]),
        head(C["skin_halfling"]),
        beard(C["hair_white"], 0.08),
        eyes([0.42,0.35,0.18,1.0]),
        spectacles([0.35,0.32,0.35,1.0]),
        weapon_staff(C["wood_dark"], C["orb_blue"]),
        aura_particles(C["orb_blue"][:3], 0.40),
    ])
    return "male_halfling_mage.ron", "MaleHalflingMage", 0.80, 65, 4.8, p

def male_human_barbarian():
    p = build([
        legs(C["leather_brown"]),
        torso(C["fur_brown"]),
        fur_mantle(C["fur_brown"]),
        arms(C["skin_tan"]),
        head(C["skin_tan"]),
        hair_cap(C["hair_black"]),
        beard(C["hair_black"], 0.20),
        eyes([0.28,0.22,0.16,1.0]),
        warpaint([0.62,0.14,0.10,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "male_human_barbarian.ron", "MaleHumanBarbarian", 1.05, 115, 5.5, p

def male_human_knight():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads(C["plate_steel"]),
        arms(C["plate_steel"]),
        head(C["skin_tan"]),
        hair_cap(C["hair_dark"]),
        eyes([0.35,0.38,0.42,1.0]),
        weapon_sword(C["iron"]),
    ])
    return "male_human_knight.ron", "MaleHumanKnight", 1.0, 105, 4.5, p

def male_human_rogue():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        cloak(C["cloth_dark"]),
        arms(C["leather_dark"]),
        head(C["skin_tan"]),
        hair_cap(C["hair_dark"]),
        beard(C["hair_dark"], 0.08),
        eyes([0.38,0.30,0.20,1.0]),
        weapon_dagger(C["iron"]),
    ])
    return "male_human_rogue.ron", "MaleHumanRogue", 1.0, 80, 8.2, p

def male_orc_archer():
    p = build([
        legs(C["leather_dark"]),
        torso(C["leather_dark"]),
        shoulder_pads([0.25,0.22,0.18,1.0]),
        arms(C["skin_orc"]),
        head(C["skin_orc"]),
        hair_cap(C["hair_black"]),
        tusks(C["tusk_ivory"]),
        eyes([0.78,0.70,0.20,1.0]),
        weapon_bow(C["wood_dark"]),
        quiver(C["leather_dark"]),
    ])
    return "male_orc_archer.ron", "MaleOrcArcher", 1.10, 85, 6.5, p

def male_orc_barbarian():
    p = build([
        legs(C["fur_brown"]),
        torso(C["fur_brown"]),
        fur_mantle(C["fur_brown"]),
        arms(C["skin_orc"]),
        head(C["skin_orc"]),
        hair_cap(C["hair_dark"]),
        tusks(C["tusk_ivory"]),
        eyes([0.72,0.60,0.14,1.0]),
        warpaint([0.65,0.30,0.08,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "male_orc_barbarian.ron", "MaleOrcBarbarian", 1.10, 120, 5.0, p

def male_orc_paladin():
    p = build([
        legs(C["plate_steel"]),
        torso(C["plate_steel"]),
        shoulder_pads(C["plate_steel"]),
        arms(C["plate_steel"]),
        head(C["skin_orc"]),
        eyes([0.72,0.62,0.18,1.0]),
        tattoo_marks([0.38,0.58,0.28,0.80]),
        holy_symbol(C["glowing_gold"]),
        weapon_sword(C["glowing_white"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "male_orc_paladin.ron", "MaleOrcPaladin", 1.10, 110, 4.0, p

def male_tiefling_barbarian():
    p = build([
        legs(C["leather_dark"]),
        torso([0.20,0.14,0.16,1.0]),
        fur_mantle(C["hair_black"]),
        arms(C["skin_tiefling_crimson"]),
        head(C["skin_tiefling_crimson"]),
        hair_cap(C["hair_black"]),
        horns_swept(C["obsidian"]),
        eyes([0.82,0.25,0.18,1.0]),
        warpaint([0.85,0.18,0.10,1.0]),
        weapon_axe(C["iron"]),
    ])
    return "male_tiefling_barbarian.ron", "MaleTieflingBarbarian", 1.05, 115, 5.5, p

def male_tiefling_cleric():
    p = build([
        legs(C["cloth_white"]),
        torso(C["cloth_white"]),
        arms(C["cloth_white"]),
        head([0.60,0.18,0.15,1.0]),   # deep crimson
        hair_cap(C["hair_black"]),
        horns_swept(C["horn_dark"]),
        eyes([0.88,0.72,0.18,1.0]),
        holy_symbol(C["glowing_gold"]),
        aura_particles(C["glowing_gold"], 0.40),
    ])
    return "male_tiefling_cleric.ron", "MaleTieflingCleric", 1.0, 80, 4.5, p

# ---------------------------------------------------------------------------
# Collect and generate all files
# ---------------------------------------------------------------------------
all_characters = [
    female_dragonborn_sorcerer(),
    female_dwarf_barbarian(),
    female_dwarf_paladin(),
    female_elf_archer_platinum(),
    female_elf_archer_brown(),
    female_elf_assassin(),
    female_elf_paladin(),
    female_elf_sorcerer(),
    female_gnome_druid(),
    female_gnome_rogue_goggles(),
    female_gnome_rogue_auburn(),
    female_gnome_warlock(),
    female_halfelf_archer(),
    female_halfelf_rogue(),
    female_halfelf_warlock(),
    female_halfling_mage(),
    female_human_barbarian_auburn(),
    female_human_barbarian_dark(),
    female_human_cleric(),
    female_human_knight_v1(),
    female_human_knight_v2(),
    female_human_paladin(),
    female_human_warlock(),
    female_orc_archer_olive(),
    female_orc_archer_green(),
    female_orc_assassin(),
    female_orc_barbarian(),
    female_orc_paladin(),
    female_tiefling_barbarian(),
    female_tiefling_cleric(),
    male_dragonborn_sorcerer(),
    male_dwarf_barbarian(),
    male_dwarf_cleric(),
    male_dwarf_knight(),
    male_dwarf_paladin(),
    male_elf_archer_blonde(),
    male_elf_archer_silver(),
    male_elf_assassin(),
    male_elf_druid(),
    male_elf_paladin(),
    male_gnome_rogue(),
    male_gnome_sorcerer(),
    male_gnome_warlock_spiked(),
    male_gnome_warlock_spectacles(),
    male_halfelf_rogue(),
    male_halfelf_warlock(),
    male_halfling_mage(),
    male_human_barbarian(),
    male_human_knight(),
    male_human_rogue(),
    male_orc_archer(),
    male_orc_barbarian(),
    male_orc_paladin(),
    male_tiefling_barbarian(),
    male_tiefling_cleric(),
]

for fname, display, scale, hp, spd, meshes in all_characters:
    write_ron(fname, display, scale, hp, spd, meshes)

print(f"Generated {len(all_characters)} RON files in {OUT}/")


def main():
    print("\n🎨 Generating ALL meshes into %s..." % OUT)

    # Detailed meshes
    write_creature("wolf_detailed.ron", get_id("wolf_detailed.ron"), "Wolf", build_detailed_wolf(), 0.9)
    write_creature("skeleton_detailed.ron", get_id("skeleton_detailed.ron"), "Skeleton", build_detailed_skeleton(), 1.0)

    # Dire Wolves
    dire_meshes = build_wolf_base(body_color=DIRE_FUR, dark_color=DIRE_DARK, eye_color=DIRE_EYE, body_scale=1.2, bulk_scale=1.15, snout_length=1.1, ear_height=1.0, include_scars=False)
    write_creature("direwolf.ron", get_id("direwolf.ron"), "DireWolf", dire_meshes, 1.1)

    leader_meshes = build_wolf_base(body_color=ALPHA_FUR, dark_color=ALPHA_DARK, eye_color=ALPHA_EYE, body_scale=1.4, bulk_scale=1.3, snout_length=1.15, ear_height=1.1, include_scars=True)
    write_creature("direwolf_leader.ron", get_id("direwolf_leader.ron"), "DireWolfLeader", leader_meshes, 1.15)

    # Override write_ron for gen_monsters2 and generate_characters
    def write_ron_monsters(filename, name, scale, health, speed, note, meshes):
        write_creature(filename, get_id(filename), name, meshes, scale)

    global write_ron
    write_ron = write_ron_monsters

    # Monsters
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

    # Characters
    chars = [obj for name, obj in globals().copy().items() if (name.startswith("female_") or name.startswith("male_")) and callable(obj)]
    for fn in chars:
        res = fn()
        if res:
            filename, name, scale, hp, spd, m_text = res
            meshes_list = m_text.split("\n        ),")
            clean_meshes = [m + "\n        )," for m in meshes_list if m.strip()]
            write_creature(filename, get_id(filename), name, clean_meshes, scale)

    print(f"\n✅ Complete! Wrote files to {OUT}")

if __name__ == '__main__':
    main()
