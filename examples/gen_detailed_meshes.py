"""
ENHANCED MESH GENERATOR
=======================
Creates highly detailed, recognizable procedural meshes with:
- Segmented curved surfaces (not just boxes)
- Anatomical accuracy (proper proportions)
- Defining features (claws, teeth, facial details)
- Multiple mesh segments for natural shapes
"""

import os, math
OUT = "/home/claude/enhanced_meshes"
os.makedirs(OUT, exist_ok=True)

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

print("\n🎨 Generating DETAILED meshes...")
print("=" * 60)

wolf_meshes = build_detailed_wolf()
write_creature("wolf_detailed.ron", 12, "Wolf", wolf_meshes, 0.9)

skeleton_meshes = build_detailed_skeleton()
write_creature("skeleton_detailed.ron", 11, "Skeleton", skeleton_meshes, 1.0)

print(f"\n✅ Complete! Created highly detailed, recognizable meshes.")
