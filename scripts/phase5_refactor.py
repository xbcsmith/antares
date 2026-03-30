#!/usr/bin/env python3
"""
Phase 5.1 Refactoring Script: MeshSpawnContext parameter struct adoption.

This script performs the mechanical refactoring of ~30 spawn_* functions in
procedural_meshes.rs to use MeshSpawnContext, and updates all callers in
map.rs, furniture_rendering.rs, events.rs, and test code.

Run from the antares project root:
    python3 scripts/phase5_refactor.py

After running, execute:
    cargo fmt --all
    cargo check --all-targets --all-features
    cargo clippy --all-targets --all-features -- -D warnings
    cargo nextest run --all-features
"""

import os
import re
import sys

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def read_file(path):
    with open(path, 'r') as f:
        return f.read()

def write_file(path, content):
    with open(path, 'w') as f:
        f.write(content)

# ---------------------------------------------------------------------------
# Phase 5.1a – procedural_meshes.rs
# ---------------------------------------------------------------------------

def refactor_procedural_meshes():
    path = 'src/game/systems/procedural_meshes.rs'
    src = read_file(path)

    # ---- 1. Remove ALL #[allow(clippy::too_many_arguments)] ---------------
    # But keep any that are in doc-comment lines (/// ...)
    src = re.sub(
        r'^(\s*)#\[allow\(clippy::too_many_arguments\)\]\n',
        '',
        src,
        flags=re.MULTILINE,
    )

    # ---- 2. Refactor function signatures ----------------------------------
    # Pattern: the 4 "context" params that should become ctx
    CTX_PARAMS = (
        r'\s*commands:\s*&mut\s*Commands\s*,\s*\n'
        r'\s*materials:\s*&mut\s*ResMut<Assets<StandardMaterial>>\s*,\s*\n'
        r'\s*meshes:\s*&mut\s*ResMut<Assets<Mesh>>\s*,\s*\n'
    )
    # cache param (may appear anywhere in the param list, often last-ish)
    CACHE_PARAM = r'\s*cache:\s*&mut\s*ProceduralMeshCache\s*,?\s*\n'
    # unused cache variant
    UCACHE_PARAM = r'\s*_cache:\s*&mut\s*ProceduralMeshCache\s*,?\s*\n'

    # --- 2a. Functions whose first 3 params are commands/materials/meshes
    #         and have cache somewhere later. Replace first 3 with ctx param
    #         and remove cache param.

    # Replace the first 3 params block with ctx
    src = re.sub(
        r'((?:pub\s+)?fn\s+spawn_\w+\(\n)'
        r'\s*commands:\s*&mut\s*Commands\s*,\s*\n'
        r'\s*materials:\s*&mut\s*ResMut<Assets<StandardMaterial>>\s*,\s*\n'
        r'\s*meshes:\s*&mut\s*ResMut<Assets<Mesh>>\s*,\s*\n',
        r'\1    ctx: &mut MeshSpawnContext,\n',
        src,
    )

    # Remove cache param lines
    # Pattern: line with just cache: &mut ProceduralMeshCache,
    src = re.sub(
        r'\n\s*_?cache:\s*&mut\s*ProceduralMeshCache\s*,?(?=\s*\n)',
        '',
        src,
    )

    # Also remove stray comment that was associated with _cache
    src = re.sub(
        r'\n\s*// Unused for now unless we cache barrel parts',
        '',
        src,
    )

    # ---- 3. Refactor function BODIES --------------------------------------
    # We need to prefix bare uses of commands/materials/meshes/cache with ctx.
    # Strategy: find each spawn_* function body and do replacements inside it.

    def refactor_body(match_obj):
        """Given a match of (fn signature ... {)(body)(closing })
        do the ctx. prefixing in the body."""
        body = match_obj.group(0)

        # Only operate on functions that have ctx param
        if 'ctx: &mut MeshSpawnContext' not in body:
            return body

        # Split into signature and body at first {
        brace_idx = body.index('{')
        sig = body[:brace_idx + 1]
        rest = body[brace_idx + 1:]

        # Replace bare `commands.` with `ctx.commands.`
        # Negative lookbehind: not preceded by `ctx.` or a word char
        rest = re.sub(r'(?<!\w)(?<!ctx\.)commands\.', 'ctx.commands.', rest)
        rest = re.sub(r'(?<!\w)(?<!ctx\.)commands\b(?!\s*[:\.])', 'ctx.commands', rest)

        # Replace bare `materials.` and bare `materials` used as args
        rest = re.sub(r'(?<!\w)(?<!ctx\.)materials\.', 'ctx.materials.', rest)
        rest = re.sub(r'(?<!\w)(?<!ctx\.)materials\b(?!\s*[:\.])', 'ctx.materials', rest)

        # Replace bare `meshes.` and bare `meshes` used as args
        rest = re.sub(r'(?<!\w)(?<!ctx\.)meshes\.', 'ctx.meshes.', rest)
        rest = re.sub(r'(?<!\w)(?<!ctx\.)meshes\b(?!\s*[:\.])', 'ctx.meshes', rest)

        # Replace bare `cache.` with `ctx.cache.`
        # Careful: don't match `procedural_cache.` or similar
        rest = re.sub(r'(?<!\w)(?<!ctx\.)cache\.', 'ctx.cache.', rest)
        rest = re.sub(r'(?<!\w)(?<!ctx\.)cache\b(?!\s*[:\.])', 'ctx.cache', rest)

        # Fix any double-prefix issues
        rest = rest.replace('ctx.ctx.', 'ctx.')

        return sig + rest

    # Match each function that has ctx param — we process the entire fn body
    # by matching balanced braces
    lines = src.split('\n')
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # Detect start of a function with ctx param
        if re.match(r'\s*(pub\s+)?fn\s+spawn_\w+\(', line):
            # Collect the full function (from fn to matching })
            fn_lines = [line]
            j = i + 1
            # First find the opening {
            depth = line.count('{') - line.count('}')
            while j < len(lines) and depth == 0:
                fn_lines.append(lines[j])
                depth += lines[j].count('{') - lines[j].count('}')
                j += 1
            # Now find the matching }
            while j < len(lines) and depth > 0:
                fn_lines.append(lines[j])
                depth += lines[j].count('{') - lines[j].count('}')
                j += 1

            fn_text = '\n'.join(fn_lines)

            if 'ctx: &mut MeshSpawnContext' in fn_text:
                fn_text = refactor_body(type('', (), {'group': lambda self, x=0: fn_text})())

            new_lines.extend(fn_text.split('\n'))
            i = j
        else:
            new_lines.append(line)
            i += 1

    src = '\n'.join(new_lines)

    # ---- 4. Fix internal cross-calls between spawn functions --------------

    # spawn_furniture calls spawn_bench, spawn_table, etc. with old args.
    # Pattern: spawn_bench(\n                commands, materials, meshes, position, map_id, config, cache, rotation_y,\n            )
    # These should become: spawn_bench(ctx, position, map_id, config, rotation_y)

    # Generic pattern for internal calls to refactored functions
    # Match: spawn_FUNC(\n  ctx.commands, ctx.materials, ctx.meshes, ...remaining..., ctx.cache, ...\n)
    # and simplify to: spawn_FUNC(ctx, ...remaining without cache...)

    # After body refactoring, internal calls look like:
    # spawn_bench(\n    ctx.commands, ctx.materials, ctx.meshes, position, map_id, config, ctx.cache, rotation_y,\n)
    # We need to replace ctx.commands, ctx.materials, ctx.meshes with just ctx
    # and remove ctx.cache

    def fix_internal_call(m):
        """Fix an internal spawn_X(...) call that has ctx.commands as first arg."""
        fn_name = m.group(1)
        args_text = m.group(2)

        # Split args
        args = []
        depth = 0
        current = ''
        for ch in args_text:
            if ch in '(<[':
                depth += 1
            elif ch in ')>]':
                depth -= 1
            elif ch == ',' and depth == 0:
                args.append(current.strip())
                current = ''
                continue
            current += ch
        if current.strip():
            args.append(current.strip())

        # Filter out ctx.commands, ctx.materials, ctx.meshes, ctx.cache
        new_args = ['ctx']
        for arg in args:
            stripped = arg.strip().rstrip(',')
            if stripped in ('ctx.commands', 'ctx.materials', 'ctx.meshes', 'ctx.cache', '&mut ctx.cache'):
                continue
            new_args.append(stripped)

        return f'{fn_name}(\n                {", ".join(new_args)},\n            )'

    # Match calls like: spawn_bench(\n    ctx.commands, ...\n)
    # This is tricky because the args span multiple lines.
    # Let's do it line-by-line instead.

    # Actually, let's use a different approach: find single-line calls first
    # Pattern: spawn_X(ctx.commands, ctx.materials, ctx.meshes, ..., ctx.cache, ...)

    # For single-line calls inside spawn_furniture
    def fix_single_line_call(m):
        prefix = m.group(1)
        fn = m.group(2)
        args = m.group(3)

        # Parse args
        arg_list = []
        depth = 0
        current = ''
        for ch in args:
            if ch in '(<[':
                depth += 1
            elif ch in ')>]':
                depth -= 1
            elif ch == ',' and depth == 0:
                arg_list.append(current.strip())
                current = ''
                continue
            current += ch
        if current.strip():
            arg_list.append(current.strip())

        new_args = ['ctx']
        for a in arg_list:
            s = a.strip().rstrip(',')
            if s in ('ctx.commands', 'ctx.materials', 'ctx.meshes', 'ctx.cache', '&mut ctx.cache'):
                continue
            new_args.append(s)

        return f'{prefix}{fn}({", ".join(new_args)})'

    # Fix multi-line internal calls: spawn_X(\n  ctx.commands, ctx.materials, ctx.meshes, args..., ctx.cache, ...\n)
    # Pattern: spawn_FUNC(\n followed by lines of args ending with \n            )

    # Let's handle this with a line-by-line approach for spawn_furniture body
    lines = src.split('\n')
    new_lines = []
    i = 0
    in_spawn_furniture = False
    furniture_depth = 0

    while i < len(lines):
        line = lines[i]

        # Track if we're inside spawn_furniture
        if 'fn spawn_furniture(' in line:
            in_spawn_furniture = True
            furniture_depth = 0

        if in_spawn_furniture:
            furniture_depth += line.count('{') - line.count('}')
            if furniture_depth <= 0 and '{' not in line and '}' in line:
                in_spawn_furniture = False

        # Check for internal call pattern inside spawn_furniture or spawn_door_with_frame
        if (in_spawn_furniture or 'spawn_door_with_frame' in ''.join(lines[max(0,i-20):i])):
            # Check if this line starts a spawn_X call with ctx.commands
            call_match = re.match(r'(\s*)(spawn_\w+)\(\s*$', line)
            if call_match and i + 1 < len(lines) and 'ctx.commands' in lines[i+1]:
                # Collect all arg lines until closing )
                call_lines = [line]
                j = i + 1
                paren_depth = 1
                while j < len(lines) and paren_depth > 0:
                    call_lines.append(lines[j])
                    paren_depth += lines[j].count('(') - lines[j].count(')')
                    j += 1

                # Parse the call
                call_text = '\n'.join(call_lines)
                indent = call_match.group(1)
                fn_name = call_match.group(2)

                # Extract args between outer parens
                first_paren = call_text.index('(')
                last_paren = call_text.rindex(')')
                args_text = call_text[first_paren+1:last_paren]

                # Parse args
                arg_list = []
                depth = 0
                current = ''
                for ch in args_text:
                    if ch in '(<[':
                        depth += 1
                    elif ch in ')>]':
                        depth -= 1
                    elif ch == ',' and depth == 0:
                        arg_list.append(current.strip())
                        current = ''
                        continue
                    current += ch
                if current.strip():
                    arg_list.append(current.strip())

                new_args = ['ctx']
                for a in arg_list:
                    s = a.strip().rstrip(',')
                    if s in ('ctx.commands', 'ctx.materials', 'ctx.meshes',
                             'ctx.cache', '&mut ctx.cache', '&mut *ctx.cache'):
                        continue
                    if s:
                        new_args.append(s)

                # Rebuild call
                # Check if the original had a trailing comma after )
                suffix = ''
                after_paren = call_text[last_paren+1:].strip()

                if len(new_args) <= 3:
                    new_call = f'{indent}{fn_name}({", ".join(new_args)})'
                else:
                    arg_lines = [f'{indent}    {a},' for a in new_args]
                    new_call = f'{indent}{fn_name}(\n' + '\n'.join(arg_lines) + f'\n{indent})'

                new_lines.append(new_call)
                i = j
                continue

        new_lines.append(line)
        i += 1

    src = '\n'.join(new_lines)

    # ---- 5. Fix doc comments referencing old params -----------------------
    # Replace doc comment lines about commands/materials/meshes/cache params
    # with a single ctx line. This is a best-effort cosmetic change.

    # Pattern: blocks of /// * `commands` - ...\n/// * `materials` - ...\n/// * `meshes` - ...
    src = re.sub(
        r"(/// # Arguments\n///\n)"
        r"/// \* `commands` - [^\n]*\n"
        r"/// \* `materials` - [^\n]*\n"
        r"/// \* `meshes` - [^\n]*\n",
        r"\1/// * `ctx` - Shared mutable spawn context (commands, materials, meshes, cache)\n",
        src,
    )
    # Remove /// * `cache` lines
    src = re.sub(r"/// \* `cache` - [^\n]*\n", '', src)

    write_file(path, src)
    print(f"  Refactored {path}")

# ---------------------------------------------------------------------------
# Phase 5.1b – Update callers in map.rs
# ---------------------------------------------------------------------------

def refactor_map_rs():
    path = 'src/game/systems/map.rs'
    src = read_file(path)

    # The spawn_map function calls procedural_meshes::spawn_tree,
    # spawn_shrub, spawn_portal, spawn_sign, spawn_furniture.
    # These calls pass &mut commands, &mut materials, &mut meshes, ..., procedural_cache.
    #
    # We need to wrap each call in a block that creates a MeshSpawnContext.
    # spawn_shrub was NOT refactored (7 params, no #[allow]), so leave it alone.

    # Add import for MeshSpawnContext
    src = src.replace(
        'use super::procedural_meshes;',
        'use super::procedural_meshes::{self, MeshSpawnContext};',
    )
    # If the import doesn't exist in that form, try another
    if 'MeshSpawnContext' not in src:
        src = src.replace(
            'use super::procedural_meshes',
            'use super::procedural_meshes::{self, MeshSpawnContext}',
        )

    # Helper: wrap a multi-line call in a MeshSpawnContext block
    def wrap_with_ctx(src, old_call_pattern, new_call_fn):
        """Find and replace a spawn call with a ctx-wrapped version."""
        lines = src.split('\n')
        new_lines = []
        i = 0
        while i < len(lines):
            match = re.search(old_call_pattern, lines[i])
            if match:
                # Collect the full call
                call_lines = [lines[i]]
                j = i + 1
                depth = lines[i].count('(') - lines[i].count(')')
                while j < len(lines) and depth > 0:
                    call_lines.append(lines[j])
                    depth += lines[j].count('(') - lines[j].count(')')
                    j += 1

                call_text = '\n'.join(call_lines)
                result = new_call_fn(call_text, match)
                if result is not None:
                    new_lines.extend(result.split('\n'))
                    i = j
                    continue

            new_lines.append(lines[i])
            i += 1

        return '\n'.join(new_lines)

    # For spawn_tree calls: replace with ctx-wrapped version
    def rewrite_spawn_tree(call_text, match):
        # Extract indentation
        indent_match = re.match(r'(\s*)', call_text)
        indent = indent_match.group(1)

        # Parse out the args
        first_paren = call_text.index('(')
        last_paren = call_text.rindex(')')
        # After ), there might be ;
        suffix = call_text[last_paren+1:].strip()

        args_text = call_text[first_paren+1:last_paren]

        arg_list = []
        depth = 0
        current = ''
        for ch in args_text:
            if ch in '(<[':
                depth += 1
            elif ch in ')>]':
                depth -= 1
            elif ch == ',' and depth == 0:
                arg_list.append(current.strip())
                current = ''
                continue
            current += ch
        if current.strip():
            arg_list.append(current.strip())

        # Remove &mut commands, &mut materials, &mut meshes, procedural_cache
        new_args = ['&mut ctx']
        skip_vals = {'&mut commands', '&mut materials', '&mut meshes',
                     'procedural_cache', '&mut procedural_cache'}
        for a in arg_list:
            s = a.strip()
            if s in skip_vals:
                continue
            new_args.append(s)

        # Build the wrapped call
        inner_indent = indent + '    '
        arg_str = ',\n'.join(f'{inner_indent}{a}' for a in new_args)

        result = (
            f'{indent}{{\n'
            f'{inner_indent}let mut ctx = MeshSpawnContext {{\n'
            f'{inner_indent}    commands: &mut commands,\n'
            f'{inner_indent}    materials: &mut materials,\n'
            f'{inner_indent}    meshes: &mut meshes,\n'
            f'{inner_indent}    cache: procedural_cache,\n'
            f'{inner_indent}}};\n'
            f'{inner_indent}procedural_meshes::spawn_tree(\n'
            f'{arg_str},\n'
            f'{inner_indent}){suffix}\n'
            f'{indent}}}'
        )
        return result

    # Apply spawn_tree rewrites
    src = wrap_with_ctx(src, r'procedural_meshes::spawn_tree\(', rewrite_spawn_tree)

    # For spawn_portal, spawn_sign, spawn_furniture - similar pattern
    def make_rewriter(fn_name):
        def rewriter(call_text, match):
            indent_match = re.match(r'(\s*)', call_text)
            indent = indent_match.group(1)

            first_paren = call_text.index('(')
            last_paren = call_text.rindex(')')
            suffix = call_text[last_paren+1:].strip()

            args_text = call_text[first_paren+1:last_paren]

            arg_list = []
            depth = 0
            current = ''
            for ch in args_text:
                if ch in '(<[':
                    depth += 1
                elif ch in ')>]':
                    depth -= 1
                elif ch == ',' and depth == 0:
                    arg_list.append(current.strip())
                    current = ''
                    continue
                current += ch
            if current.strip():
                arg_list.append(current.strip())

            skip_vals = {'&mut commands', '&mut materials', '&mut meshes',
                         'procedural_cache', '&mut procedural_cache'}
            new_args = ['&mut ctx']
            for a in arg_list:
                s = a.strip()
                if s in skip_vals:
                    continue
                new_args.append(s)

            inner_indent = indent + '    '
            arg_str = ',\n'.join(f'{inner_indent}{a}' for a in new_args)

            result = (
                f'{indent}{{\n'
                f'{inner_indent}let mut ctx = MeshSpawnContext {{\n'
                f'{inner_indent}    commands: &mut commands,\n'
                f'{inner_indent}    materials: &mut materials,\n'
                f'{inner_indent}    meshes: &mut meshes,\n'
                f'{inner_indent}    cache: procedural_cache,\n'
                f'{inner_indent}}};\n'
                f'{inner_indent}procedural_meshes::{fn_name}(\n'
                f'{arg_str},\n'
                f'{inner_indent}){suffix}\n'
                f'{indent}}}'
            )
            return result
        return rewriter

    src = wrap_with_ctx(src, r'procedural_meshes::spawn_portal\(', make_rewriter('spawn_portal'))
    src = wrap_with_ctx(src, r'procedural_meshes::spawn_sign\(', make_rewriter('spawn_sign'))
    src = wrap_with_ctx(src, r'procedural_meshes::spawn_furniture\(', make_rewriter('spawn_furniture'))

    write_file(path, src)
    print(f"  Refactored {path}")

# ---------------------------------------------------------------------------
# Phase 5.1c – Update callers in furniture_rendering.rs
# ---------------------------------------------------------------------------

def refactor_furniture_rendering():
    path = 'src/game/systems/furniture_rendering.rs'
    src = read_file(path)

    # Add MeshSpawnContext to imports
    if 'MeshSpawnContext' not in src:
        src = re.sub(
            r'(use super::procedural_meshes::\{[^}]*)\}',
            lambda m: m.group(1) + ', MeshSpawnContext}',
            src,
        )
        if 'MeshSpawnContext' not in src:
            # Try adding a new import
            src = src.replace(
                'use super::procedural_meshes::',
                'use super::procedural_meshes::{MeshSpawnContext, ',
            )

    # Refactor spawn_furniture_with_rendering:
    # - Remove #[allow(clippy::too_many_arguments)]
    src = re.sub(
        r'#\[allow\(clippy::too_many_arguments\)\]\n(\s*pub fn spawn_furniture_with_rendering)',
        r'\1',
        src,
    )

    # Replace its param list: remove commands, materials, meshes, cache; add ctx
    src = re.sub(
        r'(pub fn spawn_furniture_with_rendering\(\n)'
        r'\s*commands:\s*&mut\s*Commands\s*,\s*\n'
        r'\s*materials:\s*&mut\s*ResMut<Assets<StandardMaterial>>\s*,\s*\n'
        r'\s*meshes:\s*&mut\s*ResMut<Assets<Mesh>>\s*,\s*\n',
        r'\1    ctx: &mut MeshSpawnContext,\n',
        src,
    )

    # Remove cache param
    src = re.sub(
        r'\n\s*cache:\s*&mut\s*ProceduralMeshCache\s*,?\n',
        '\n',
        src,
    )

    # In the body: replace spawn_X(commands, materials, meshes, ..., cache, ...)
    # with spawn_X(ctx, ...)
    # And replace bare commands. with ctx.commands.

    # Process the function body
    lines = src.split('\n')
    new_lines = []
    in_fn = False
    fn_depth = 0

    for line in lines:
        if 'fn spawn_furniture_with_rendering(' in line:
            in_fn = True
            fn_depth = 0

        if in_fn:
            fn_depth += line.count('{') - line.count('}')

            # Replace bare commands usage
            if 'commands.entity' in line and 'ctx.commands' not in line:
                line = line.replace('commands.entity', 'ctx.commands.entity')
            if 'commands.spawn' in line and 'ctx.commands' not in line:
                line = line.replace('commands.spawn', 'ctx.commands.spawn')

            # Replace spawn function calls that still have old args
            # Pattern: spawn_X(\n  commands,\n  materials,\n  ...
            # We'll handle these in a second pass

            if fn_depth <= 0 and '}' in line:
                in_fn = False

        new_lines.append(line)

    src = '\n'.join(new_lines)

    # Now fix the internal spawn calls. These are like:
    # spawn_throne(\n    commands,\n    materials,\n    meshes,\n    position,\n    map_id,\n    ...,\n    cache,\n    rotation_y,\n)
    # Becomes: spawn_throne(ctx, position, map_id, ..., rotation_y)

    lines = src.split('\n')
    new_lines = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Check for spawn_X( call pattern
        call_match = re.match(r'(\s*)(spawn_\w+)\(\s*$', line)
        if call_match and i + 1 < len(lines):
            next_line = lines[i+1].strip()
            if next_line in ('commands,', 'ctx.commands,'):
                # This is an old-style call, collect it
                call_lines = [line]
                j = i + 1
                paren_depth = 1
                while j < len(lines) and paren_depth > 0:
                    call_lines.append(lines[j])
                    paren_depth += lines[j].count('(') - lines[j].count(')')
                    j += 1

                indent = call_match.group(1)
                fn_name = call_match.group(2)
                call_text = '\n'.join(call_lines)

                # Parse args
                first_p = call_text.index('(')
                last_p = call_text.rindex(')')
                args_text = call_text[first_p+1:last_p]

                arg_list = []
                depth = 0
                current = ''
                for ch in args_text:
                    if ch in '(<[{':
                        depth += 1
                    elif ch in ')>]}':
                        depth -= 1
                    elif ch == ',' and depth == 0:
                        arg_list.append(current.strip())
                        current = ''
                        continue
                    current += ch
                if current.strip():
                    arg_list.append(current.strip())

                skip = {'commands', 'materials', 'meshes', 'cache',
                        'ctx.commands', 'ctx.materials', 'ctx.meshes', 'ctx.cache',
                        '&mut cache', '&mut ctx.cache'}
                new_args = ['ctx']
                for a in arg_list:
                    s = a.strip()
                    if s in skip:
                        continue
                    if s:
                        new_args.append(s)

                # Rebuild
                suffix = call_text[last_p+1:].strip()
                if len(new_args) <= 4:
                    new_call = f'{indent}{fn_name}({", ".join(new_args)}){suffix}'
                else:
                    inner = indent + '    '
                    arg_lines = [f'{inner}{a},' for a in new_args]
                    new_call = f'{indent}{fn_name}(\n' + '\n'.join(arg_lines) + f'\n{indent}){suffix}'

                new_lines.append(new_call)
                i = j
                continue

        new_lines.append(line)
        i += 1

    src = '\n'.join(new_lines)

    write_file(path, src)
    print(f"  Refactored {path}")

# ---------------------------------------------------------------------------
# Phase 5.1d – Update callers of spawn_furniture_with_rendering in events.rs
# ---------------------------------------------------------------------------

def refactor_events_rs():
    path = 'src/game/systems/events.rs'
    if not os.path.exists(path):
        print(f"  Skipped {path} (not found)")
        return

    src = read_file(path)

    # Check if spawn_furniture_with_rendering is called here
    if 'spawn_furniture_with_rendering' not in src:
        print(f"  Skipped {path} (no calls to update)")
        return

    # Add MeshSpawnContext import if needed
    if 'MeshSpawnContext' not in src:
        src = re.sub(
            r'(use super::procedural_meshes::\{[^}]*)\}',
            lambda m: m.group(1) + ', MeshSpawnContext}',
            src,
        )

    write_file(path, src)
    print(f"  Updated {path}")

# ---------------------------------------------------------------------------
# Phase 5.1e – Fix tests in procedural_meshes.rs
# ---------------------------------------------------------------------------

def fix_tests_procedural_meshes():
    path = 'src/game/systems/procedural_meshes.rs'
    src = read_file(path)

    # Find test system functions that call spawn_door_frame or spawn_door_with_frame
    # These create inner system fns that need MeshSpawnContext

    # Pattern: fn spawn_frame_system(\n  mut commands: Commands,\n  mut materials: ...\n  mut meshes: ...\n) {\n  let mut cache = ...\n  ...  spawn_door_frame(\n    &mut commands,\n    ...

    # Replace the old pattern with ctx-based pattern
    # This is specific to the 3 test functions

    def fix_test_system(src, old_fn_call, fn_name):
        """Fix a test system function that calls a spawn function."""
        # Find the test system function
        pattern = (
            r'(fn \w+_system\(\n'
            r'\s*mut commands: Commands,\n'
            r'\s*mut materials: ResMut<Assets<StandardMaterial>>,\n'
            r'\s*mut meshes: ResMut<Assets<Mesh>>,\n'
            r'\s*\) \{\n)'
            r'(\s*let mut cache = ProceduralMeshCache::default\(\);\n)'
        )

        # We'll do a line-by-line approach for better control
        lines = src.split('\n')
        new_lines = []
        i = 0

        while i < len(lines):
            line = lines[i]

            # Look for pattern: spawn_door_frame(\n  &mut commands,...
            if f'{fn_name}(' in line and '&mut commands' in (lines[i+1] if i+1 < len(lines) else ''):
                # This is a call that needs fixing
                # Collect the call
                call_lines = [line]
                j = i + 1
                depth = line.count('(') - line.count(')')
                while j < len(lines) and depth > 0:
                    call_lines.append(lines[j])
                    depth += lines[j].count('(') - lines[j].count(')')
                    j += 1

                call_text = '\n'.join(call_lines)
                indent_match = re.match(r'(\s*)', line)
                indent = indent_match.group(1)

                # Parse args
                first_p = call_text.index('(')
                last_p = call_text.rindex(')')
                args_text = call_text[first_p+1:last_p]

                arg_list = []
                depth = 0
                current = ''
                for ch in args_text:
                    if ch in '(<[{':
                        depth += 1
                    elif ch in ')>]}':
                        depth -= 1
                    elif ch == ',' and depth == 0:
                        arg_list.append(current.strip())
                        current = ''
                        continue
                    current += ch
                if current.strip():
                    arg_list.append(current.strip())

                skip = {'&mut commands', '&mut materials', '&mut meshes',
                        '&mut cache', 'cache', '&mut *cache'}
                new_args = ['&mut ctx']
                for a in arg_list:
                    s = a.strip()
                    if s in skip:
                        continue
                    if s:
                        new_args.append(s)

                # Add ctx construction before the call
                new_lines.append(f'{indent}let mut ctx = MeshSpawnContext {{')
                new_lines.append(f'{indent}    commands: &mut commands,')
                new_lines.append(f'{indent}    materials: &mut materials,')
                new_lines.append(f'{indent}    meshes: &mut meshes,')
                new_lines.append(f'{indent}    cache: &mut cache,')
                new_lines.append(f'{indent}}};')

                # Rebuild call
                suffix = call_text[last_p+1:].strip()
                inner = indent + '    '
                arg_lines = [f'{inner}{a},' for a in new_args]
                new_call = f'{indent}{fn_name}(\n' + '\n'.join(arg_lines) + f'\n{indent}){suffix}'
                new_lines.append(new_call)

                i = j
                continue

            new_lines.append(line)
            i += 1

        return '\n'.join(new_lines)

    src = fix_test_system(src, 'spawn_door_frame', 'spawn_door_frame')
    src = fix_test_system(src, 'spawn_door_with_frame', 'spawn_door_with_frame')

    write_file(path, src)
    print(f"  Fixed tests in {path}")

# ---------------------------------------------------------------------------
# Phase 5.3 – Type aliases for complex queries
# ---------------------------------------------------------------------------

def add_type_aliases():
    """Add type aliases for complex Bevy queries to eliminate type_complexity."""

    files_to_check = [
        'src/game/systems/combat.rs',
        'src/game/systems/combat_visual.rs',
        'src/game/systems/creature_meshes.rs',
        'src/game/systems/hud.rs',
        'src/game/systems/menu.rs',
        'src/game/systems/sprite_uv_update.rs',
    ]

    for path in files_to_check:
        if not os.path.exists(path):
            continue
        src = read_file(path)
        count = src.count('#[allow(clippy::type_complexity)]')
        if count > 0:
            print(f"  {path}: {count} type_complexity suppressions (manual review needed)")

# ---------------------------------------------------------------------------
# Phase 5.2 – Extract sub-renderers (too_many_lines)
# ---------------------------------------------------------------------------

def check_too_many_lines():
    """Report too_many_lines suppressions for manual extraction."""
    import glob

    count = 0
    for path in glob.glob('src/game/systems/*.rs'):
        src = read_file(path)
        n = src.count('#[allow(clippy::too_many_lines)]')
        if n > 0:
            print(f"  {path}: {n} too_many_lines suppressions")
            count += n

    print(f"  Total too_many_lines suppressions: {count}")

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    # Ensure we're in the project root
    if not os.path.exists('src/game/systems/procedural_meshes.rs'):
        print("ERROR: Run this script from the antares project root")
        sys.exit(1)

    print("Phase 5.1a: Refactoring procedural_meshes.rs...")
    refactor_procedural_meshes()

    print("Phase 5.1b: Updating map.rs callers...")
    refactor_map_rs()

    print("Phase 5.1c: Updating furniture_rendering.rs callers...")
    refactor_furniture_rendering()

    print("Phase 5.1d: Updating events.rs callers...")
    refactor_events_rs()

    print("Phase 5.1e: Fixing tests...")
    fix_tests_procedural_meshes()

    print("\nPhase 5.2: Checking too_many_lines suppressions...")
    check_too_many_lines()

    print("\nPhase 5.3: Checking type_complexity suppressions...")
    add_type_aliases()

    print("\n--- Done! Now run: ---")
    print("  cargo fmt --all")
    print("  cargo check --all-targets --all-features")
    print("  cargo clippy --all-targets --all-features -- -D warnings")
    print("  cargo nextest run --all-features")

if __name__ == '__main__':
    main()
