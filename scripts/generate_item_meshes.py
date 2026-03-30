#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0
"""
generate_item_meshes.py
=======================
Developer convenience tool — generates CreatureDefinition RON asset files
for every dropped-item mesh type used by antares.

Each output file contains one or more flat-lying mesh parts appropriate for
the item category.  Multi-part items (sword blade + crossguard, staff shaft +
orb, bow limb + string, etc.) are emitted as separate MeshDefinition entries
inside the same CreatureDefinition, matching the hand-authored files committed
to campaigns/tutorial/assets/items/.

IDs start at 9000 to avoid collision with creature / NPC / template IDs:
  Weapon types        9001–9099
  Armor types         9100–9199
  Consumable types    9200–9299
  Accessory types     9300–9399
  Ammo types          9400–9499
  Quest types         9500–9599

Usage
-----
    # Default: write full manifest to campaigns/tutorial/assets/items/
    python examples/generate_item_meshes.py

    # Test fixtures only: write to data/test_campaign/assets/items/
    python examples/generate_item_meshes.py --test-fixtures

    # Custom root directory (sub-dirs created automatically)
    python examples/generate_item_meshes.py --output-dir /tmp/items

The script is idempotent — re-running it overwrites existing files with
freshly generated geometry.  All generated .ron files are committed to the
repository; this script is *not* a build step.
"""

import argparse
import math
import os

# ---------------------------------------------------------------------------
# RON formatting helpers
# ---------------------------------------------------------------------------

UP_NORMAL = [0.0, 1.0, 0.0]


def fv(v):
    """Format a 3-vector as a RON tuple string."""
    return f"({v[0]:.4f}, {v[1]:.4f}, {v[2]:.4f})"


def fc(c):
    """Format a 4-component RGBA color as a RON tuple string."""
    return f"({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f})"


def fmat(color, metallic, roughness, emissive=None):
    """Return a MaterialDefinition RON block as a string."""
    if emissive:
        em_line = f"emissive: Some(({emissive[0]:.2f}, {emissive[1]:.2f}, {emissive[2]:.2f})),"
    else:
        em_line = "emissive: None,"
    return (
        "Some((\n"
        f"                base_color: {fc(color)},\n"
        f"                metallic: {metallic:.2f},\n"
        f"                roughness: {roughness:.2f},\n"
        f"                {em_line}\n"
        "                alpha_mode: Opaque,\n"
        "            ))"
    )


def emit_mesh(name, verts, indices, color, normals=None, material=None):
    """Return a complete MeshDefinition RON block as a string."""
    normals = normals or [UP_NORMAL] * len(verts)
    lines = [
        "        (",
        f'            name: Some("{name}"),',
        f'            vertices: [{", ".join(fv(v) for v in verts)}],',
        f'            indices: [{", ".join(str(i) for i in indices)}],',
        f'            normals: Some([{", ".join(fv(n) for n in normals)}]),',
        "            uvs: None,",
        f"            color: {fc(color)},",
        "            lod_levels: None,",
        "            lod_distances: None,",
        f"            material: {material if material is not None else 'None'},",
        "            texture_path: None,",
        "        ),",
    ]
    return "\n".join(lines)


def emit_transform(tx=0.0, ty=0.0, tz=0.0):
    """Return a MeshTransform RON block as a string."""
    return (
        "        (\n"
        f"            translation: ({tx:.1f}, {ty:.1f}, {tz:.1f}),\n"
        "            rotation: (0.0, 0.0, 0.0),\n"
        "            scale: (1.0, 1.0, 1.0),\n"
        "        ),"
    )


def write_item_ron(out_dir, filename, creature_id, name, mesh_parts, scale,
                   transforms=None):
    """
    Write a complete CreatureDefinition RON file.

    Parameters
    ----------
    out_dir      : destination directory (created if absent)
    filename     : e.g. "long_sword.ron"
    creature_id  : integer ID for the CreatureDefinition
    name         : e.g. "ItemMeshLongSword"
    mesh_parts   : list of MeshDefinition RON strings (one per sub-mesh)
    scale        : overall creature scale
    transforms   : optional list of (tx, ty, tz) tuples, one per mesh part;
                   defaults to identity for every part
    """
    if transforms is None:
        transforms = [(0.0, 0.0, 0.0)] * len(mesh_parts)

    mesh_str = "\n".join(mesh_parts)
    transform_str = "\n".join(emit_transform(*t) for t in transforms)

    body = (
        f"(\n"
        f"    id: {creature_id},\n"
        f'    name: "{name}",\n'
        f"    meshes: [\n{mesh_str}\n    ],\n"
        f"    mesh_transforms: [\n{transform_str}\n    ],\n"
        f"    scale: {scale:.4f},\n"
        f"    color_tint: None,\n"
        f")\n"
    )

    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, filename)
    with open(path, "w") as fh:
        fh.write(body)
    print(f"  ✓ {filename}  (id={creature_id}, parts={len(mesh_parts)})")


# ---------------------------------------------------------------------------
# Color constants — mirror item_mesh.rs exactly
# ---------------------------------------------------------------------------

COLOR_STEEL       = [0.75, 0.75, 0.78, 1.0]
COLOR_CROSSGUARD  = [0.60, 0.60, 0.64, 1.0]
COLOR_IRON        = [0.50, 0.50, 0.52, 1.0]
COLOR_WOOD        = [0.55, 0.35, 0.15, 1.0]
COLOR_WOOD_DARK   = [0.58, 0.38, 0.20, 1.0]
COLOR_LEATHER     = [0.72, 0.53, 0.30, 1.0]
COLOR_SILVER      = [0.82, 0.83, 0.85, 1.0]
COLOR_SILVER_DIM  = [0.75, 0.76, 0.78, 1.0]
COLOR_PLATE_DIM   = [0.20, 0.20, 0.22, 1.0]
COLOR_GOLD        = [1.00, 0.84, 0.00, 1.0]
COLOR_RED         = [0.85, 0.10, 0.10, 1.0]
COLOR_BLUE        = [0.10, 0.30, 0.90, 1.0]
COLOR_GREEN       = [0.10, 0.75, 0.20, 1.0]
COLOR_YELLOW      = [0.95, 0.80, 0.05, 1.0]
COLOR_PARCHMENT   = [0.95, 0.90, 0.72, 1.0]
COLOR_AMMO        = [0.90, 0.88, 0.70, 1.0]
COLOR_FLETCHING   = [0.70, 0.20, 0.20, 1.0]
COLOR_QUEST       = [0.85, 0.15, 0.85, 1.0]
COLOR_ORB         = [0.50, 0.60, 0.90, 1.0]
COLOR_STONE       = [0.60, 0.58, 0.56, 1.0]

EMISSIVE_MAGIC    = [0.40, 0.40, 0.60]
EMISSIVE_ORB      = [0.10, 0.15, 0.45]
EMISSIVE_QUEST    = [0.50, 0.00, 0.50]

BASE_SCALE              = 0.35
TWO_HANDED_SCALE_MULT   = 1.60   # 0.35 × 1.60 = 0.56
ARMOR_MED_SCALE_MULT    = 1.10   # 0.35 × 1.10 = 0.385
ARMOR_HEAVY_SCALE_MULT  = 1.30   # 0.35 × 1.30 = 0.455
SMALL_SCALE_MULT        = 0.55   # 0.35 × 0.55 = 0.1925


# ---------------------------------------------------------------------------
# Geometry builders — one function per logical item type.
# Each returns (list_of_mesh_strings, list_of_transform_tuples).
# ---------------------------------------------------------------------------

def _sword_geometry(blade_half_len, crossguard_half_w, crossguard_half_h,
                    blade_half_w=None):
    """
    Generic sword/dagger: diamond blade + rectangular crossguard.

    The blade tip is at +Z, pommel at -Z.  Crossguard spans ±crossguard_half_w
    on the X axis, centred on the blade–grip join.

    blade_half_len  : half the blade length along Z
    crossguard_half_w : half the crossguard width on X
    crossguard_half_h : half the crossguard depth on Z
    blade_half_w    : half the blade width on X (defaults to 12 % of blade_half_len)
    """
    if blade_half_w is None:
        blade_half_w = blade_half_len * 0.12

    pommel_z = -blade_half_len * 0.30

    blade_verts = [
        [0.0, 0.0,  blade_half_len],  # 0 tip
        [ blade_half_w, 0.0, 0.0],    # 1 right
        [-blade_half_w, 0.0, 0.0],    # 2 left
        [0.0, 0.0,  pommel_z],        # 3 pommel
    ]
    blade_idx = [0, 1, 3, 0, 3, 2]
    blade_mesh = emit_mesh(
        "blade", blade_verts, blade_idx, COLOR_STEEL,
        material=fmat(COLOR_STEEL, 0.60, 0.30),
    )

    cg_hw = crossguard_half_w
    cg_hh = crossguard_half_h
    cg_verts = [
        [-cg_hw, 0.0,  cg_hh],  # 0
        [ cg_hw, 0.0,  cg_hh],  # 1
        [ cg_hw, 0.0, -cg_hh],  # 2
        [-cg_hw, 0.0, -cg_hh],  # 3
    ]
    cg_idx = [0, 1, 2, 0, 2, 3]
    cg_mesh = emit_mesh(
        "crossguard", cg_verts, cg_idx, COLOR_CROSSGUARD,
        material=fmat(COLOR_CROSSGUARD, 0.65, 0.35),
    )

    meshes = [blade_mesh, cg_mesh]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def build_sword():
    """Generic sword — medium blade with proportional crossguard."""
    return _sword_geometry(
        blade_half_len=0.4125,
        crossguard_half_w=0.110,
        crossguard_half_h=0.020,
    )


def build_dagger():
    """Short blade with narrow crossguard."""
    return _sword_geometry(
        blade_half_len=0.2725,
        crossguard_half_w=0.070,
        crossguard_half_h=0.015,
    )


def build_short_sword():
    """Between dagger and sword length."""
    return _sword_geometry(
        blade_half_len=0.2900,
        crossguard_half_w=0.090,
        crossguard_half_h=0.018,
    )


def build_long_sword():
    """Longer blade, wider crossguard."""
    return _sword_geometry(
        blade_half_len=0.4125,
        crossguard_half_w=0.130,
        crossguard_half_h=0.022,
    )


def build_great_sword():
    """Two-handed sword: longest blade and widest crossguard."""
    return _sword_geometry(
        blade_half_len=0.5000,
        crossguard_half_w=0.160,
        crossguard_half_h=0.025,
        blade_half_w=0.060,
    )


def build_club():
    """
    Club: rectangular wooden handle + hexagonal wooden head.
    The head sits above the handle along +Z.
    """
    # Handle — thin rectangle centred at origin
    h_hw = 0.030
    h_top = 0.150
    h_bot = -0.280
    handle_verts = [
        [-h_hw, 0.0,  h_top],  # 0
        [ h_hw, 0.0,  h_top],  # 1
        [ h_hw, 0.0,  h_bot],  # 2
        [-h_hw, 0.0,  h_bot],  # 3
    ]
    handle_idx = [0, 1, 2, 0, 2, 3]
    handle = emit_mesh(
        "handle", handle_verts, handle_idx, COLOR_WOOD,
        material=fmat(COLOR_WOOD, 0.00, 0.70),
    )

    # Head — fan hexagon centred at (0, 0, 0.2062) relative to creature origin
    head_r  = 0.12
    head_cz = 0.2062
    head_top = head_cz + head_r * 1.5   # ≈ 0.375
    head_bot = head_cz - head_r * 1.5   # ≈ 0.025 → use actual measured values
    head_verts = [
        [0.0,      0.0,  0.3750],   # 0 top
        [ head_r,  0.0,  0.2375],   # 1 right-top
        [ head_r,  0.0,  0.0625],   # 2 right-bot
        [0.0,      0.0, -0.0750],   # 3 bottom
        [-head_r,  0.0,  0.0625],   # 4 left-bot
        [-head_r,  0.0,  0.2375],   # 5 left-top
        [0.0,      0.0,  head_cz],  # 6 centre fan hub
    ]
    head_idx = [6, 0, 1,  6, 1, 2,  6, 2, 3,  6, 3, 4,  6, 4, 5,  6, 5, 0]
    head = emit_mesh(
        "head", head_verts, head_idx, COLOR_WOOD_DARK,
        material=fmat(COLOR_WOOD_DARK, 0.30, 0.50),
    )

    meshes = [handle, head]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def build_staff():
    """
    Staff: rectangular wooden shaft + magical orb tip.
    Orb is translated to the top of the shaft along +Z.
    """
    half_len = 0.480
    half_w   = 0.035
    shaft_verts = [
        [-half_w, 0.0,  half_len],  # 0
        [ half_w, 0.0,  half_len],  # 1
        [ half_w, 0.0, -half_len],  # 2
        [-half_w, 0.0, -half_len],  # 3
    ]
    shaft_idx = [0, 1, 2, 0, 2, 3]
    shaft = emit_mesh(
        "shaft", shaft_verts, shaft_idx, COLOR_WOOD,
        material=fmat(COLOR_WOOD, 0.00, 0.70),
    )

    # Orb — 8-sided regular polygon fan, radius 0.11
    orb_r = 0.11
    orb_n = 8
    orb_verts = []
    for i in range(orb_n):
        angle = i * 2.0 * math.pi / orb_n
        orb_verts.append([orb_r * math.cos(angle), 0.0, orb_r * math.sin(angle)])
    orb_verts.append([0.0, 0.0, 0.0])  # centre at index orb_n
    centre = orb_n
    orb_idx = []
    for i in range(orb_n):
        orb_idx.extend([centre, i, (i + 1) % orb_n])
    orb = emit_mesh(
        "orb_tip", orb_verts, orb_idx, COLOR_ORB,
        material=fmat(COLOR_ORB, 0.20, 0.30, emissive=EMISSIVE_ORB),
    )

    meshes = [shaft, orb]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, half_len)]
    return meshes, transforms


def build_bow():
    """
    Bow: curved-arc limb body + taut bowstring.
    """
    arc_h  = 0.42   # half-length of the arc along Z
    curve  = 0.13   # how far the outer limb bows out on X

    limb_verts = [
        [ curve,  0.0,  arc_h],    # 0 top-outer tip
        [ 0.04,   0.0,  arc_h],    # 1 top-inner
        [ 0.0,    0.0,  0.0],      # 2 grip centre
        [ 0.04,   0.0, -arc_h],    # 3 bot-inner
        [ curve,  0.0, -arc_h],    # 4 bot-outer tip
        [-0.015,  0.0, -arc_h],    # 5 bot-back
        [-0.015,  0.0,  arc_h],    # 6 top-back
    ]
    limb_idx = [0, 1, 6,  1, 2, 6,  2, 3, 5,  3, 4, 5]
    limb = emit_mesh(
        "limb", limb_verts, limb_idx, COLOR_WOOD,
        material=fmat(COLOR_WOOD, 0.00, 0.60),
    )

    # Bowstring — thin tapered diamond
    s_hw = 0.015
    string_verts = [
        [ 0.0, 0.0,  arc_h],   # 0 top
        [ s_hw, 0.0, 0.0],     # 1 mid-right
        [ 0.0, 0.0, -arc_h],   # 2 bot
        [-s_hw, 0.0, 0.0],     # 3 mid-left
    ]
    string_idx = [0, 1, 3,  1, 2, 3]
    string_color = [0.88, 0.82, 0.60, 1.0]
    string = emit_mesh(
        "string", string_verts, string_idx, string_color,
        material=fmat(string_color, 0.00, 0.80),
    )

    meshes = [limb, string]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def _armor_trapezoid(color, metallic, roughness, top_hw=0.20, bot_hw=0.14, half_h=0.28):
    """Trapezoid breastplate silhouette."""
    verts = [
        [-top_hw, 0.0,  half_h],
        [ top_hw, 0.0,  half_h],
        [ bot_hw, 0.0, -half_h],
        [-bot_hw, 0.0, -half_h],
    ]
    idx = [0, 1, 2,  0, 2, 3]
    return emit_mesh(
        "body", verts, idx, color,
        material=fmat(color, metallic, roughness),
    )


def build_leather_armor():
    """Single trapezoid leather breastplate."""
    part = _armor_trapezoid(COLOR_LEATHER, metallic=0.00, roughness=0.80)
    return [part], [(0.0, 0.0, 0.0)]


def build_chain_mail():
    """Single trapezoid chain-mail breastplate."""
    part = _armor_trapezoid(COLOR_IRON, metallic=0.50, roughness=0.50)
    meshes = [emit_mesh(
        "chain",
        [[-0.200, 0.0, 0.280], [0.200, 0.0, 0.280],
         [0.140, 0.0, -0.280], [-0.140, 0.0, -0.280]],
        [0, 1, 2,  0, 2, 3],
        COLOR_IRON,
        material=fmat(COLOR_IRON, 0.50, 0.50),
    )]
    return meshes, [(0.0, 0.0, 0.0)]


def build_plate_mail():
    """
    Plate mail: body plate + wide shoulder pauldrons.
    """
    # Body
    body_verts = [
        [-0.180, 0.0,  0.240],  # 0 chest-left
        [ 0.180, 0.0,  0.240],  # 1 chest-right
        [ 0.150, 0.0, -0.240],  # 2 hip-right
        [-0.150, 0.0, -0.240],  # 3 hip-left
    ]
    body_idx = [0, 1, 2,  0, 2, 3]
    body = emit_mesh(
        "body", body_verts, body_idx, COLOR_SILVER,
        material=fmat(COLOR_SILVER, 0.75, 0.25),
    )

    # Shoulders — U-shaped pauldron bar
    s_verts = [
        [-0.320, 0.0,  0.240],  # 0 outer-left-top
        [ 0.320, 0.0,  0.240],  # 1 outer-right-top
        [ 0.320, 0.0,  0.080],  # 2 outer-right-bot
        [ 0.180, 0.0,  0.080],  # 3 inner-right-top
        [ 0.180, 0.0,  0.000],  # 4 inner-right-bot
        [-0.180, 0.0,  0.000],  # 5 inner-left-bot
        [-0.180, 0.0,  0.080],  # 6 inner-left-top
        [-0.320, 0.0,  0.080],  # 7 outer-left-bot
    ]
    s_idx = [0, 1, 7,  1, 2, 7,  2, 6, 7,  2, 3, 6,  3, 5, 6,  3, 4, 5]
    shoulders = emit_mesh(
        "shoulders", s_verts, s_idx, COLOR_SILVER_DIM,
        material=fmat(COLOR_SILVER_DIM, 0.75, 0.28),
    )

    meshes = [body, shoulders]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def build_shield():
    """Kite-shield silhouette."""
    hw = 0.180
    ht = 0.260
    verts = [
        [ 0.0,        0.0,  ht],            # 0 top
        [ hw,         0.0,  0.050],          # 1 right
        [ hw * 0.60,  0.0, -ht * 0.60],     # 2 bot-right
        [ 0.0,        0.0, -ht],             # 3 tip
        [-hw * 0.60,  0.0, -ht * 0.60],     # 4 bot-left
        [-hw,         0.0,  0.050],          # 5 left
    ]
    idx = [0, 1, 5,  1, 2, 4,  1, 4, 5,  2, 3, 4]
    part = emit_mesh(
        "shield", verts, idx, COLOR_IRON,
        material=fmat(COLOR_IRON, 0.40, 0.50),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_helmet():
    """
    Helmet: pentagon dome + rectangular visor bar.
    """
    r = 0.22
    dome_verts = []
    for i in range(5):
        angle = math.pi / 2.0 + i * 2.0 * math.pi / 5.0
        dome_verts.append([r * math.cos(angle), 0.0, r * math.sin(angle)])
    dome_verts.append([0.0, 0.0, 0.0])  # centre at index 5
    dome_idx = []
    for i in range(5):
        dome_idx.extend([5, i, (i + 1) % 5])
    dome = emit_mesh(
        "dome", dome_verts, dome_idx, COLOR_SILVER,
        material=fmat(COLOR_SILVER, 0.55, 0.38),
    )

    v_hw = 0.180
    v_hh = 0.030
    visor_verts = [
        [-v_hw, 0.0,  v_hh],
        [ v_hw, 0.0,  v_hh],
        [ v_hw, 0.0, -v_hh],
        [-v_hw, 0.0, -v_hh],
    ]
    visor_idx = [0, 1, 2,  0, 2, 3]
    visor = emit_mesh(
        "visor", visor_verts, visor_idx, COLOR_PLATE_DIM,
        material=fmat(COLOR_PLATE_DIM, 0.30, 0.50),
    )

    meshes = [dome, visor]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def build_boots():
    """L-shaped boot silhouette."""
    verts = [
        [-0.060, 0.0,  0.200],  # 0 ankle-top-left
        [ 0.060, 0.0,  0.200],  # 1 ankle-top-right
        [ 0.060, 0.0, -0.050],  # 2 ankle-bot-right
        [ 0.180, 0.0, -0.050],  # 3 toe-right-top
        [ 0.180, 0.0, -0.180],  # 4 toe-right-bot
        [-0.060, 0.0, -0.180],  # 5 toe-left-bot
    ]
    idx = [0, 1, 5,  1, 2, 5,  2, 4, 5,  2, 3, 4]
    part = emit_mesh(
        "boots", verts, idx, COLOR_LEATHER,
        material=fmat(COLOR_LEATHER, 0.00, 0.80),
    )
    return [part], [(0.0, 0.0, 0.0)]


def _potion(color, emissive=None):
    """Hexagonal disc potion silhouette with fan triangulation."""
    r = 0.10
    n = 6
    verts = []
    for i in range(n):
        angle = i * 2.0 * math.pi / n
        verts.append([r * math.cos(angle), 0.0, r * math.sin(angle)])
    verts.append([0.0, 0.0, 0.0])  # centre at index n
    centre = n
    idx = []
    for i in range(n):
        idx.extend([centre, i, (i + 1) % n])
    part = emit_mesh(
        "potion", verts, idx, color,
        material=fmat(color, 0.00, 0.30, emissive=emissive),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_health_potion():
    return _potion(COLOR_RED, emissive=EMISSIVE_MAGIC)


def build_mana_potion():
    return _potion(COLOR_BLUE, emissive=EMISSIVE_MAGIC)


def build_cure_potion():
    return _potion(COLOR_GREEN, emissive=EMISSIVE_MAGIC)


def build_attribute_potion():
    return _potion(COLOR_YELLOW, emissive=EMISSIVE_MAGIC)


def _ring_torus(name, color, metallic=0.80, roughness=0.15, outer_r=0.16, inner_r=0.07, n=12):
    """
    Flat torus approximation: two concentric n-gons joined by quad strips.

    outer_r  : outer radius
    inner_r  : inner radius
    n        : number of segments
    """
    outer_verts = []
    inner_verts = []
    for i in range(n):
        angle = i * 2.0 * math.pi / n
        c, s = math.cos(angle), math.sin(angle)
        outer_verts.append([outer_r * c, 0.0, outer_r * s])
        inner_verts.append([inner_r * c, 0.0, inner_r * s])

    verts = outer_verts + inner_verts   # outer: 0..n-1, inner: n..2n-1
    idx = []
    for i in range(n):
        o0 = i
        o1 = (i + 1) % n
        i0 = n + i
        i1 = n + (i + 1) % n
        # Two triangles per quad strip segment
        idx.extend([o0, o1, i0,  o1, i1, i0])

    part = emit_mesh(
        name, verts, idx, color,
        material=fmat(color, metallic, roughness),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_ring():
    return _ring_torus("band", COLOR_GOLD, outer_r=0.16, inner_r=0.07, n=12)


def build_amulet():
    """Octagon disc amulet."""
    r = 0.10
    n = 8
    verts = []
    for i in range(n):
        angle = i * 2.0 * math.pi / n
        verts.append([r * math.cos(angle), 0.0, r * math.sin(angle)])
    verts.append([0.0, 0.0, 0.0])  # centre
    centre = n
    idx = []
    for i in range(n):
        idx.extend([centre, i, (i + 1) % n])
    part = emit_mesh(
        "amulet", verts, idx, COLOR_GOLD,
        material=fmat(COLOR_GOLD, 0.80, 0.15),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_belt():
    """Thin horizontal rectangle belt."""
    hw = 0.220
    hh = 0.040
    verts = [
        [-hw, 0.0,  hh],
        [ hw, 0.0,  hh],
        [ hw, 0.0, -hh],
        [-hw, 0.0, -hh],
    ]
    idx = [0, 1, 2,  0, 2, 3]
    part = emit_mesh(
        "belt", verts, idx, COLOR_LEATHER,
        material=fmat(COLOR_LEATHER, 0.10, 0.70),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_cloak():
    """Wide teardrop cloak silhouette."""
    hw = 0.280
    ht = 0.320
    verts = [
        [ 0.0,        0.0,  ht * 0.50],   # 0 collar-centre
        [-hw,         0.0,  ht * 0.30],   # 1 collar-left
        [-hw * 0.80,  0.0, -ht],           # 2 hem-left
        [ 0.0,        0.0, -ht * 1.05],   # 3 hem-centre
        [ hw * 0.80,  0.0, -ht],           # 4 hem-right
        [ hw,         0.0,  ht * 0.30],   # 5 collar-right
    ]
    idx = [0, 5, 1,  1, 5, 4,  1, 4, 2,  2, 4, 3]
    part = emit_mesh(
        "cloak", verts, idx, COLOR_LEATHER,
        material=fmat(COLOR_LEATHER, 0.00, 0.90),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_arrow():
    """
    Arrow: thin rectangular shaft + V-shaped fletching at tail.
    """
    hl   = 0.240   # half shaft length
    s_hw = 0.018   # shaft half-width

    shaft_verts = [
        [ 0.0,   0.0,  hl],
        [ s_hw,  0.0,  0.0],
        [ 0.0,   0.0, -hl],
        [-s_hw,  0.0,  0.0],
    ]
    shaft_idx = [0, 1, 3,  1, 2, 3]
    shaft = emit_mesh(
        "shaft", shaft_verts, shaft_idx, COLOR_AMMO,
        material=fmat(COLOR_AMMO, 0.10, 0.60),
    )

    # Fletching — V fan at the tail end
    fl_hw = 0.070
    fl_tip_z  = -hl         # tip of shaft
    fl_root_z = -hl * 0.50  # where vanes meet the shaft
    fl_mid_z  = -hl * 0.83  # middle of the vane
    fletch_verts = [
        [  0.0,   0.0,  fl_root_z],   # 0 root
        [  fl_hw, 0.0,  fl_tip_z],    # 1 right tip
        [  0.0,   0.0,  fl_mid_z],    # 2 right-mid (fold point)
        [ -fl_hw, 0.0,  fl_tip_z],    # 3 left tip
    ]
    fletch_idx = [0, 1, 2,  0, 2, 3]
    fletch = emit_mesh(
        "fletching", fletch_verts, fletch_idx, COLOR_FLETCHING,
        material=fmat(COLOR_FLETCHING, 0.00, 0.80),
    )

    meshes = [shaft, fletch]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def build_bolt():
    """Crossbow bolt — short fat diamond."""
    hl = 0.220
    hw = 0.025
    verts = [
        [ 0.0,  0.0,  hl],
        [ hw,   0.0,  0.0],
        [ 0.0,  0.0, -hl],
        [-hw,   0.0,  0.0],
    ]
    idx = [0, 1, 3,  1, 2, 3]
    part = emit_mesh(
        "bolt", verts, idx, COLOR_AMMO,
        material=fmat(COLOR_AMMO, 0.10, 0.60),
    )
    return [part], [(0.0, 0.0, 0.0)]


def build_stone():
    """Sling stone — small flattened diamond."""
    hl = 0.220
    hw = 0.025
    verts = [
        [ 0.0,  0.0,  hl],
        [ hw,   0.0,  0.0],
        [ 0.0,  0.0, -hl],
        [-hw,   0.0,  0.0],
    ]
    idx = [0, 1, 3,  1, 2, 3]
    part = emit_mesh(
        "stone", verts, idx, COLOR_STONE,
        material=fmat(COLOR_STONE, 0.10, 0.60),
    )
    return [part], [(0.0, 0.0, 0.0)]


def _star_16(name, r_outer, color, emissive=None, metallic=0.20, roughness=0.40):
    """
    16-perimeter-vertex star/sunburst (alternating outer/inner radii) with
    a centre fan hub.  Used for quest seals and key items.
    """
    r_inner = r_outer * 0.707
    n_outer = 8
    verts = []
    for i in range(n_outer):
        outer_angle = math.pi / 2.0 + i * 2.0 * math.pi / n_outer
        inner_angle = outer_angle + math.pi / n_outer
        verts.append([r_outer * math.cos(outer_angle), 0.0,
                      r_outer * math.sin(outer_angle)])
        verts.append([r_inner * math.cos(inner_angle), 0.0,
                      r_inner * math.sin(inner_angle)])
    verts.append([0.0, 0.0, 0.0])  # centre at index 16
    centre = 16
    n_perim = 16
    idx = []
    for i in range(n_perim):
        idx.extend([centre, i, (i + 1) % n_perim])
    return emit_mesh(
        name, verts, idx, color,
        material=fmat(color, metallic, roughness, emissive=emissive),
    )


def build_quest_scroll():
    """
    Quest scroll: hexagonal scroll body + 16-point seal emblem.
    """
    # Scroll body
    hw = 0.060
    hl = 0.180
    body_verts = [
        [ 0.0,  0.0,  hl],          # 0 top
        [ hw,   0.0,  hl * 0.50],   # 1 right-top
        [ hw,   0.0, -hl * 0.50],   # 2 right-bot
        [ 0.0,  0.0, -hl],          # 3 bottom
        [-hw,   0.0, -hl * 0.50],   # 4 left-bot
        [-hw,   0.0,  hl * 0.50],   # 5 left-top
    ]
    body_idx = [0, 1, 5,  1, 2, 4,  1, 4, 5,  2, 3, 4]
    body = emit_mesh(
        "quest_scroll", body_verts, body_idx, COLOR_PARCHMENT,
        material=fmat(COLOR_PARCHMENT, 0.00, 0.80),
    )
    seal = _star_16("seal", r_outer=0.15, color=COLOR_GOLD,
                    emissive=EMISSIVE_QUEST, metallic=0.20, roughness=0.40)

    meshes = [body, seal]
    transforms = [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]
    return meshes, transforms


def build_key_item():
    """Quest key item — 16-point star in quest purple."""
    star = _star_16("key_item", r_outer=0.15, color=COLOR_QUEST,
                    emissive=EMISSIVE_QUEST, metallic=0.20, roughness=0.40)
    return [star], [(0.0, 0.0, 0.0)]


# ---------------------------------------------------------------------------
# Item manifest
# (creature_id, filename, subdirectory, human_name, build_fn, scale)
# ---------------------------------------------------------------------------

MANIFEST = [
    # ── Weapons ──────────────────────────────────────────────────────────────
    (9001, "sword.ron",         "weapons",      "ItemMeshSword",
     build_sword,           BASE_SCALE * 1.15),

    (9002, "dagger.ron",        "weapons",      "ItemMeshDagger",
     build_dagger,          BASE_SCALE * 0.90),

    (9003, "short_sword.ron",   "weapons",      "ItemMeshShortSword",
     build_short_sword,     BASE_SCALE),

    (9004, "long_sword.ron",    "weapons",      "ItemMeshLongSword",
     build_long_sword,      BASE_SCALE * 1.25),

    (9005, "great_sword.ron",   "weapons",      "ItemMeshGreatSword",
     build_great_sword,     BASE_SCALE * TWO_HANDED_SCALE_MULT),

    (9006, "club.ron",          "weapons",      "ItemMeshClub",
     build_club,            BASE_SCALE * 1.15),

    (9007, "staff.ron",         "weapons",      "ItemMeshStaff",
     build_staff,           BASE_SCALE * TWO_HANDED_SCALE_MULT),

    (9008, "bow.ron",           "weapons",      "ItemMeshBow",
     build_bow,             BASE_SCALE * TWO_HANDED_SCALE_MULT),

    # ── Armor ─────────────────────────────────────────────────────────────────
    (9101, "leather_armor.ron", "armor",        "ItemMeshLeatherArmor",
     build_leather_armor,   BASE_SCALE),

    (9102, "chain_mail.ron",    "armor",        "ItemMeshChainMail",
     build_chain_mail,      BASE_SCALE * ARMOR_MED_SCALE_MULT),

    (9103, "plate_mail.ron",    "armor",        "ItemMeshPlateMail",
     build_plate_mail,      BASE_SCALE * ARMOR_HEAVY_SCALE_MULT),

    (9104, "shield.ron",        "armor",        "ItemMeshShield",
     build_shield,          BASE_SCALE),

    (9105, "helmet.ron",        "armor",        "ItemMeshHelmet",
     build_helmet,          BASE_SCALE * ARMOR_MED_SCALE_MULT),

    (9106, "boots.ron",         "armor",        "ItemMeshBoots",
     build_boots,           BASE_SCALE),

    # ── Consumables ──────────────────────────────────────────────────────────
    (9201, "health_potion.ron",    "consumables", "ItemMeshHealthPotion",
     build_health_potion,   BASE_SCALE * SMALL_SCALE_MULT),

    (9202, "mana_potion.ron",      "consumables", "ItemMeshManaPotion",
     build_mana_potion,     BASE_SCALE * SMALL_SCALE_MULT),

    (9203, "cure_potion.ron",      "consumables", "ItemMeshCurePotion",
     build_cure_potion,     BASE_SCALE * SMALL_SCALE_MULT),

    (9204, "attribute_potion.ron", "consumables", "ItemMeshAttributePotion",
     build_attribute_potion, BASE_SCALE * SMALL_SCALE_MULT),

    # ── Accessories ──────────────────────────────────────────────────────────
    (9301, "ring.ron",   "accessories", "ItemMeshRing",
     build_ring,    BASE_SCALE * SMALL_SCALE_MULT),

    (9302, "amulet.ron", "accessories", "ItemMeshAmulet",
     build_amulet,  BASE_SCALE * SMALL_SCALE_MULT),

    (9303, "belt.ron",   "accessories", "ItemMeshBelt",
     build_belt,    BASE_SCALE),

    (9304, "cloak.ron",  "accessories", "ItemMeshCloak",
     build_cloak,   BASE_SCALE),

    # ── Ammo ─────────────────────────────────────────────────────────────────
    (9401, "arrow.ron", "ammo", "ItemMeshArrow",
     build_arrow,   BASE_SCALE * SMALL_SCALE_MULT),

    (9402, "bolt.ron",  "ammo", "ItemMeshBolt",
     build_bolt,    BASE_SCALE * SMALL_SCALE_MULT),

    (9403, "stone.ron", "ammo", "ItemMeshStone",
     build_stone,   BASE_SCALE * SMALL_SCALE_MULT),

    # ── Quest ─────────────────────────────────────────────────────────────────
    (9501, "quest_scroll.ron", "quest", "ItemMeshQuestScroll",
     build_quest_scroll, BASE_SCALE),

    (9502, "key_item.ron",     "quest", "ItemMeshKeyItem",
     build_key_item,     BASE_SCALE),
]

# ---------------------------------------------------------------------------
# Test-fixture subset — minimal set for data/test_campaign/assets/items/
# Must match what data/test_campaign/data/item_mesh_registry.ron references.
# ---------------------------------------------------------------------------

TEST_MANIFEST = [
    # (creature_id, filename, human_name, build_fn, scale)
    (9001, "sword.ron",  "ItemMeshSword",  build_long_sword,     BASE_SCALE * 1.25),
    (9201, "potion.ron", "ItemMeshPotion", build_health_potion,  BASE_SCALE * SMALL_SCALE_MULT),
]


# ---------------------------------------------------------------------------
# Entry points
# ---------------------------------------------------------------------------

def parse_args():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root  = os.path.dirname(script_dir)
    default_out = os.path.join(
        repo_root, "campaigns", "tutorial", "assets", "items"
    )

    parser = argparse.ArgumentParser(
        description="Generate item mesh CreatureDefinition RON files for antares.",
    )
    parser.add_argument(
        "--output-dir",
        default=default_out,
        metavar="DIR",
        help=(
            "Root directory for item RON output.  "
            "Sub-directories (weapons/, armor/, etc.) are created automatically.  "
            f"Default: {default_out}"
        ),
    )
    parser.add_argument(
        "--test-fixtures",
        action="store_true",
        help=(
            "Write the minimal test-fixture subset to "
            "<repo_root>/data/test_campaign/assets/items/ "
            "instead of the full manifest."
        ),
    )
    return parser.parse_args(), repo_root


def main():
    args, repo_root = parse_args()

    if args.test_fixtures:
        out_root = os.path.join(
            repo_root, "data", "test_campaign", "assets", "items"
        )
        print(f"\n🔧 Generating test fixtures → {out_root}")
        for creature_id, filename, human_name, build_fn, scale in TEST_MANIFEST:
            meshes, transforms = build_fn()
            write_item_ron(out_root, filename, creature_id, human_name,
                           meshes, scale, transforms)
        print(f"\n✅ Test fixtures complete ({len(TEST_MANIFEST)} files)")
        return

    out_root = args.output_dir
    print(f"\n🎨 Generating item meshes → {out_root}")

    for creature_id, filename, subdir, human_name, build_fn, scale in MANIFEST:
        meshes, transforms = build_fn()
        sub_dir = os.path.join(out_root, subdir)
        write_item_ron(sub_dir, filename, creature_id, human_name,
                       meshes, scale, transforms)

    print(f"\n✅ Complete — wrote {len(MANIFEST)} item mesh files to {out_root}")


if __name__ == "__main__":
    main()
