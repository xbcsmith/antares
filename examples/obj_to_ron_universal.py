#!/usr/bin/env python3
"""
Universal OBJ to RON Converter
===============================
Converts ANY Wavefront OBJ file to Bevy 0.17 RON procedural mesh format.
Handles multiple meshes, automatic triangulation, and intelligent color assignment.

Usage:
    python3 obj_to_ron_universal.py input.obj output.ron creature_id creature_name [scale]

Examples:
    python3 obj_to_ron_universal.py zara.obj zara.ron 1008 ApprenticeZara 0.01
    python3 obj_to_ron_universal.py minotaur.obj minotaur.ron 2001 Minotaur 0.01
"""

import os
import sys
import re
from collections import defaultdict

# ─────────────────────────────────────────────────────────────────────────
# COLOR PALETTE
# ─────────────────────────────────────────────────────────────────────────

COLORS = {
    # Skin tones
    'skin': (0.92, 0.85, 0.78, 1.0),
    'skin_dark': (0.88, 0.80, 0.72, 1.0),
    
    # Fur/Hide
    'fur_brown': (0.45, 0.30, 0.20, 1.0),
    'fur_dark': (0.35, 0.25, 0.18, 1.0),
    'fur_gray': (0.55, 0.52, 0.50, 1.0),
    
    # Hair
    'hair_pink': (0.98, 0.55, 0.72, 1.0),
    'hair_brown': (0.45, 0.30, 0.20, 1.0),
    'hair_blonde': (0.95, 0.85, 0.60, 1.0),
    'hair_black': (0.15, 0.12, 0.10, 1.0),
    
    # Eyes
    'eye_white': (0.98, 0.98, 0.95, 1.0),
    'eye_iris_amber': (0.78, 0.55, 0.25, 1.0),
    'eye_iris_blue': (0.30, 0.60, 0.85, 1.0),
    'eye_iris_green': (0.35, 0.75, 0.45, 1.0),
    'eye_pupil': (0.08, 0.08, 0.08, 1.0),
    'tearline': (0.85, 0.90, 0.95, 0.3),
    
    # Mouth/Teeth
    'tongue': (0.95, 0.50, 0.55, 1.0),
    'teeth': (0.98, 0.96, 0.94, 1.0),
    
    # Eyebrows
    'eyebrow': (0.40, 0.25, 0.15, 1.0),
    
    # Clothing
    'cloth_white': (0.90, 0.88, 0.92, 1.0),
    'cloth_gray': (0.85, 0.82, 0.88, 1.0),
    'cloth_purple': (0.55, 0.35, 0.75, 1.0),
    
    # Materials
    'horn_bone': (0.92, 0.88, 0.82, 1.0),
    'metal_iron': (0.60, 0.62, 0.65, 1.0),
    'metal_gold': (0.95, 0.80, 0.35, 1.0),
    'leather': (0.55, 0.35, 0.25, 1.0),
    
    # Default
    'default': (0.8, 0.8, 0.8, 1.0),
}

def get_color_for_mesh(mesh_name):
    """Intelligently assign color based on mesh name."""
    name_lower = mesh_name.lower()
    
    # Minotaur/Monster specific
    if 'horn' in name_lower:
        return COLORS['horn_bone']
    if 'fur' in name_lower or 'hide' in name_lower:
        return COLORS['fur_brown']
    
    # Body parts
    if 'body' in name_lower or 'torso' in name_lower:
        return COLORS['skin']
    if 'head' in name_lower or 'face' in name_lower or 'skull' in name_lower:
        return COLORS['skin']
    
    # Hair
    if 'hair' in name_lower:
        if 'pink' in name_lower:
            return COLORS['hair_pink']
        elif 'blonde' in name_lower or 'yellow' in name_lower:
            return COLORS['hair_blonde']
        elif 'black' in name_lower or 'dark' in name_lower:
            return COLORS['hair_black']
        else:
            return COLORS['hair_brown']
    
    # Eyebrows (check BEFORE eyes since "eyebrows" contains "eye")
    if 'brow' in name_lower or 'eyebrow' in name_lower:
        return COLORS['eyebrow']
    
    # Eyes
    if 'eye' in name_lower:
        if 'white' in name_lower or 'sclera' in name_lower:
            return COLORS['eye_white']
        elif 'pupil' in name_lower:
            return COLORS['eye_pupil']
        elif 'iris' in name_lower:
            return COLORS['eye_iris_amber']
        else:
            return COLORS['eye_white']
    
    # Tearline
    if 'tear' in name_lower:
        return COLORS['tearline']
    
    # Mouth
    if 'tongue' in name_lower:
        return COLORS['tongue']
    if 'teeth' in name_lower or 'tooth' in name_lower:
        return COLORS['teeth']
    
    # Clothing
    if 'bra' in name_lower:
        return COLORS['cloth_white']
    if 'underwear' in name_lower or 'panties' in name_lower or 'bottom' in name_lower:
        return COLORS['cloth_gray']
    if 'cloth' in name_lower or 'fabric' in name_lower or 'robe' in name_lower:
        if 'purple' in name_lower:
            return COLORS['cloth_purple']
        return COLORS['cloth_gray']
    
    # Materials
    if 'metal' in name_lower or 'armor' in name_lower or 'plate' in name_lower:
        if 'gold' in name_lower:
            return COLORS['metal_gold']
        return COLORS['metal_iron']
    if 'leather' in name_lower:
        return COLORS['leather']
    
    # Default
    return COLORS['default']

# ─────────────────────────────────────────────────────────────────────────
# RON OUTPUT
# ─────────────────────────────────────────────────────────────────────────

def fv(v): 
    return f"({v[0]:.4f}, {v[1]:.4f}, {v[2]:.4f})"

def fc(c): 
    return f"({c[0]:.2f}, {c[1]:.2f}, {c[2]:.2f}, {c[3]:.2f})"

def emit_mesh(name, verts, indices, color, normals=None):
    """Create RON mesh string."""
    if len(indices) % 3 != 0:
        raise ValueError(f"Mesh {name}: {len(indices)} indices not divisible by 3")
    
    L = ["        ("]
    L.append(f'            name: Some("{name}"),')
    L.append(f'            vertices: [{", ".join(fv(v) for v in verts)}],')
    L.append(f'            indices: [{", ".join(str(i) for i in indices)}],')
    
    if normals:
        L.append(f'            normals: Some([{", ".join(fv(n) for n in normals)}]),')
    else:
        L.append('            normals: None,')
    
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

def write_creature(filepath, creature_id, name, meshes, scale):
    """Write final RON file."""
    mesh_str = "\n".join(meshes)
    transforms = "\n".join([emit_transform() for _ in meshes])
    body = (f"(\n    id: {creature_id},\n    name: \"{name}\",\n"
            f"    meshes: [\n{mesh_str}\n    ],\n"
            f"    mesh_transforms: [\n{transforms}\n    ],\n"
            f"    scale: {scale},\n    color_tint: None,\n)\n")
    with open(filepath, "w") as f:
        f.write(body)
    print(f"  ✓ Written: {filepath} ({len(meshes)} meshes)")

# ─────────────────────────────────────────────────────────────────────────
# OBJ PARSER
# ─────────────────────────────────────────────────────────────────────────

class OBJParser:
    def __init__(self, filepath):
        self.vertices = []  # Global vertex list
        self.normals = []   # Global normal list
        self.objects = []   # List of (name, faces)
        self.current_object = None
        self.current_faces = []
        
        self.parse(filepath)
    
    def parse(self, filepath):
        """Parse OBJ file."""
        print(f"\n📖 Parsing {filepath}...")
        
        with open(filepath, 'r') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                
                parts = line.split()
                if not parts:
                    continue
                
                cmd = parts[0]
                
                if cmd == 'v':
                    # Vertex: v x y z
                    x, y, z = float(parts[1]), float(parts[2]), float(parts[3])
                    self.vertices.append([x, y, z])
                
                elif cmd == 'vn':
                    # Normal: vn x y z
                    x, y, z = float(parts[1]), float(parts[2]), float(parts[3])
                    self.normals.append([x, y, z])
                
                elif cmd == 'g' or cmd == 'o':
                    # New object/group
                    self.save_current_object()
                    # Object name is everything after 'g' or 'o'
                    self.current_object = ' '.join(parts[1:]) if len(parts) > 1 else 'default'
                    self.current_faces = []
                
                elif cmd == 'f':
                    # Face: f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 [v4/vt4/vn4]
                    face = []
                    for vertex_str in parts[1:]:
                        # Parse v/vt/vn or v//vn or v/vt or v
                        indices = vertex_str.split('/')
                        v_idx = int(indices[0]) - 1  # OBJ is 1-indexed
                        face.append(v_idx)
                    
                    # Triangulate based on face vertex count
                    if len(face) == 3:
                        # Already a triangle
                        self.current_faces.append(face)
                    elif len(face) == 4:
                        # Quad - split into two triangles
                        self.current_faces.append([face[0], face[1], face[2]])
                        self.current_faces.append([face[0], face[2], face[3]])
                    elif len(face) > 4:
                        # N-gon - fan triangulation
                        for i in range(1, len(face) - 1):
                            self.current_faces.append([face[0], face[i], face[i + 1]])
        
        # Save final object
        self.save_current_object()
        
        print(f"  ✓ Parsed {len(self.vertices)} vertices")
        print(f"  ✓ Parsed {len(self.normals)} normals")
        print(f"  ✓ Found {len(self.objects)} object groups")
    
    def save_current_object(self):
        """Save current object to list."""
        if self.current_object and self.current_faces:
            self.objects.append((self.current_object, self.current_faces))
            self.current_faces = []
    
    def convert_to_meshes(self):
        """Convert OBJ objects to RON meshes with intelligent coloring."""
        meshes = []
        
        for obj_name, faces in self.objects:
            # Find unique vertices used by this object
            used_vertices = set()
            for face in faces:
                used_vertices.update(face)
            
            # Create vertex mapping (global index -> local index)
            vertex_map = {}
            local_verts = []
            for global_idx in sorted(used_vertices):
                if global_idx < len(self.vertices):
                    vertex_map[global_idx] = len(local_verts)
                    local_verts.append(self.vertices[global_idx])
            
            # Remap face indices to local
            local_indices = []
            for face in faces:
                for v in face:
                    if v in vertex_map:
                        local_indices.append(vertex_map[v])
            
            # Skip empty meshes
            if not local_verts or not local_indices:
                continue
            
            # Clean object name for mesh
            clean_name = re.sub(r'[^a-zA-Z0-9_]', '_', obj_name)
            clean_name = re.sub(r'_+', '_', clean_name).strip('_')
            if not clean_name:
                clean_name = f"mesh_{len(meshes)}"
            
            # Get appropriate color for this mesh
            color = get_color_for_mesh(obj_name)
            color_name = [k for k, v in COLORS.items() if v == color][0]
            
            # Create mesh
            try:
                mesh_str = emit_mesh(clean_name, local_verts, local_indices, color)
                meshes.append(mesh_str)
                print(f"    → {clean_name}: {len(local_verts)} verts, {len(local_indices)//3} tris ({color_name})")
            except ValueError as e:
                print(f"    ✗ Skipping {clean_name}: {e}")
        
        return meshes

# ─────────────────────────────────────────────────────────────────────────
# MAIN
# ─────────────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 5:
        print(__doc__)
        sys.exit(1)
    
    input_path = sys.argv[1]
    output_path = sys.argv[2]
    creature_id = int(sys.argv[3])
    creature_name = sys.argv[4]
    scale = float(sys.argv[5]) if len(sys.argv) > 5 else 0.01
    
    print("\n" + "=" * 70)
    print("🔄 Universal OBJ to RON Converter")
    print("=" * 70)
    print(f"Input:  {input_path}")
    print(f"Output: {output_path}")
    print(f"ID: {creature_id}, Name: {creature_name}, Scale: {scale}")
    
    # Parse OBJ
    parser = OBJParser(input_path)
    
    # Convert to meshes with colors
    print("\n🎨 Converting to meshes with colors...")
    meshes = parser.convert_to_meshes()
    
    if not meshes:
        print("\n❌ No meshes generated!")
        sys.exit(1)
    
    # Write RON file
    print(f"\n💾 Writing {output_path}...")
    write_creature(output_path, creature_id, creature_name, meshes, scale)
    
    print(f"\n✅ Conversion complete!")

if __name__ == "__main__":
    main()
