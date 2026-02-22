"""
Generate procedural mesh RON files for all 54 player characters
from the portrait_prompt.md file.

Each character is built from archetype meshes (body, head, arms, legs)
with race-specific modifications and class-specific equipment.
Color palettes are matched to each character's description.
"""

import os

OUT = "/home/claude/portraits_ron"
os.makedirs(OUT, exist_ok=True)

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
def fmt_v3(v):
    return f"[{v[0]:.3f}, {v[1]:.3f}, {v[2]:.3f}]"

def fmt_color(c):
    return f"Some([{c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f}])"

def mesh_block(name, verts, indices, color,
               normals=None, uvs=None):
    lines = []
    lines.append("        MeshDefinition(")
    lines.append(f'            name: "{name}",')
    lines.append("            vertices: [")
    for v in verts:
        lines.append(f"                {fmt_v3(v)},")
    lines.append("            ],")
    if normals:
        lines.append("            normals: Some([")
        for n in normals:
            lines.append(f"                {fmt_v3(n)},")
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
    lines.append(f"            color: {fmt_color(color)},")
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
