# NPC Sprite Sheet Generation Prompts

This document contains detailed AI image generation prompts for the NPCs defined in `campaigns/tutorial/data/npcs.ron`.

**Target Specifications:**
- **Style**: 32-bit Classic RPG style (SNES/PS1 era), clean lines, vibrant but grounded colors.
- **Format**: Sprite sheet grid.
- **Character Base**: 32x48 pixels per frame.
- **Sheet Content**: Each sheet MUST be a strict grid containing:
  - Idle frames (Front, Back, Left Side, Right Side)
  - Walking animation cycle frames (Front, Back, Left, Right)
- **Background**: Solid white or magenta background (for easy removal).
- **Perspective**: 2.5D Top-Down RPG.

---

## 1. Village Elder (Town Square)
**ID**: `tutorial_elder_village`
**Context**: The wise elder of the village, providing quests and lore.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Village Elder character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: An elderly man with a long, flowing white beard and thinning grey hair. He wears dignified, earth-toned robes (moss green and loam brown) with a simple wooden staff in his right hand. He has a slightly hunched posture indicating age but retains an air of wisdom and authority.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 2. Innkeeper (Town Square)
**ID**: `tutorial_innkeeper_town`
**Context**: Proprietor of the Cozy Inn, welcoming and warm.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a friendly Innkeeper character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A stout, cheerful man with rosy cheeks and a thick brown mustache. He wears a pristine white apron over a beige tunic and brown trousers. He is holding a wooden tankard or a cleaning cloth. His appearance is welcoming, warm, and hospitable.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 3. Merchant (Town Square)
**ID**: `tutorial_merchant_town`
**Context**: Traveling merchant selling goods.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Traveling Merchant character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A middle-aged man carrying a comically large, overflowing backpack strapped to his back, filled with rolled scrolls, potions, and equipment. He wears a wide-brimmed feathered hat and a colorful traveler's outfit (teal and mustard yellow). He looks eager to make a sale.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 4. High Priestess (Town Square)
**ID**: `tutorial_priestess_town`
**Context**: Head priestess providing healing and blessings.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a High Priestess character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A woman of serene bearing, wearing elaborate flowing robes of white and gold with blue trim. She wears a tall ceremonial headdress or circlet. She holds her hands clasped in front of her or raised in blessing. Her expression is calm and benevolent.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 5. Arcturus (Wizard)
**ID**: `tutorial_wizard_arcturus`
**Context**: Seasoned local guide and wizard.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Veteran Wizard named Arcturus. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: An old, rugged wizard who looks more like a field researcher than a tower scholar. He wears worn, patch-reinforced robes of grey-blue and a battered wizard's hat. He carries a glowing crystal staff that serves as both a weapon and a walking stick. He looks determined and experienced.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 6. Arcturus' Brother
**ID**: `tutorial_wizard_arcturus_brother`
**Context**: Local villager and gossip.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Village Gossip character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: He shares facial features with Arcturus (the wizard) but is dressed in common villager attire—a simple brown tunic, green vest, and trousers. He has a "leaning in to whisper" posture and a slightly sly or overly friendly expression. He looks like a regular townsperson with no magical gear.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 7. Lost Ranger
**ID**: `tutorial_ranger_lost`
**Context**: A ranger who has lost his way.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Lost Ranger character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A scout wearing leather armor and a forest-green hooded cloak. He has a longbow strapped to his back. He dominates a confused body language, perhaps looking at a map or scratching his head. His gear looks travel-worn.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 8. Mountain Village Elder
**ID**: `tutorial_elder_village2`
**Context**: The grumpy elder of the mountain pass village.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Grumpy Mountain Elder character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: Distinct from the town elder, this man wears thick, heavy furs and wool clothing suitable for a cold climate. He has a scowling, curmudgeonly expression and leans heavily on a thick, gnarled cane. His beard is shorter and thicker. He looks hardy but irritable.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 9. Mountain Innkeeper
**ID**: `tutorial_innkeeper_town2`
**Context**: Proprietor of the mountain pass inn.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Mountain Innkeeper character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A sturdy woman dressed in warm winter clothing—a thick woolen dress with a fur-lined vest. She has a no-nonsense, hardy demeanor. She carries a tray with a steaming bowl of stew. Her colors are warm reds and browns to contrast with accurate snowy surroundings.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 10. Mountain Merchant
**ID**: `tutorial_merchant_town2`
**Context**: Traveling merchant in the mountains.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Mountain Merchant character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A heavily bundled-up merchant wearing a thick parka and goggles or a scarf over their face. They are pulling a small sled or cart laden with winter supplies (blankets, firewood, tools). The outfit should look weather-proof and bulky.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 11. Mountain High Priest
**ID**: `tutorial_priest_town2`
**Context**: Head cleric of the mountain temple.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Mountain High Priest character. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A male priest wearing formal but heavy religious vestments suited for the cold—thick velvet robes of deep crimson with gold embroidery. He holds a large, open tome or scripture book. He stands tall and austere, projecting spiritual authority.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Walking animation side view (4 frames).
> Ensure the character fits within a 32x48 pixel base footprint for each cell. NO anti-aliasing or blurring. Crisp pixel art only.

---

## 12. Dying Goblin
**ID**: `monster_goblin_dying`
**Context**: A dying/injured goblin.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Goblin character in a dying state. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A small creature with moss-green skin, large pointy ears, and sharp yellow teeth. He has a bloodied face and a pained expression. He wears a ragged brown loincloth.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Standing, clutching wound).
> - **Row 2**: Walking animation front view (heavy limp).
> - **Row 3**: Collapsed/Dying pose (lying on ground, face down/up) - ESSENTIAL frames.
> - **Row 4**: Dead pose (static body).
> Ensure the character fits within a 32x48 pixel base footprint. NO anti-aliasing. Crisp pixel art.

---

## 13. Dragon
**ID**: `monster_dragon_boss`
**Context**: A majestic dragon boss.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Majestic Dragon. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A massive dragon with shimmering emerald/gold scales and a regal crown of horns. It has piercing intelligent eyes and a long powerful tail. The design should convey wisdom and immense power.
>
> **Sheet Layout**: rigorous grid layout on a white background. **Grid Size Note**: This character is LARGE (64x64 or 96x96 pixels per cell).
> - **Row 1**: Idle poses (Head weaving, wings folded).
> - **Row 2**: Walking/Stomping animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Attack animation (Breath weapon or Claw swipe).
> Ensure the character is consistent in scale. NO anti-aliasing. Crisp pixel art.

---

## 14. Lich
**ID**: `monster_lich_boss`
**Context**: A malevolent skeletal lich.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Skeletal Lich. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A skeletal figure wearing tattered, regal purple and black robes. He has a long white beard hanging from his bony jaw and bushy white eyebrows. He wears a pointed wizard's hat. His eye sockets glow with piercing cold blue light.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Floating or standing, holding staff).
> - **Row 2**: Walking/Floating animation front view (4 frames).
> - **Row 3**: Walking/Floating animation back view (4 frames).
> - **Row 4**: Casting spell animation (raising hands, magical energy).
> Ensure the character fits within a 32x48 pixel base footprint. NO anti-aliasing. Crisp pixel art.

---

## 15. Orc Warrior
**ID**: `monster_orc_warrior`
**Context**: A fierce orc warrior.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Fierce Orc Warrior. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: A hulking warrior with grey-green skin and thick dark fur pelts on his shoulders. He has prominent tusks and a fierce, battle-ready expression. He wields a crude but deadly axe or sword.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Attack/Combat Stance (brandishing weapon).
> Ensure the character fits within a 32x48 pixel base footprint. NO anti-aliasing. Crisp pixel art.

---

## 16. Skeleton Warrior
**ID**: `monster_skeleton_warrior`
**Context**: A skeletal warrior.

> **Prompt**:
> Generate a 32-bit classic RPG pixel art sprite sheet for a Skeleton Warrior. The style should be reminiscent of SNES/PS1 era RPGs with clean lines and vibrant but grounded colors.
>
> **Character Detail**: An animated human skeleton with visible bleached bones. He carries a rusted iron longsword and perhaps a cracked round shield. His expression is a fixed, fierce rictus grin.
>
> **Sheet Layout**: rigorous grid layout on a white background.
> - **Row 1**: Idle poses (Front view, Back view, Side view).
> - **Row 2**: Walking animation front view (4 frames).
> - **Row 3**: Walking animation back view (4 frames).
> - **Row 4**: Attack/Combat Stance (swinging sword).
> Ensure the character fits within a 32x48 pixel base footprint. NO anti-aliasing. Crisp pixel art.
