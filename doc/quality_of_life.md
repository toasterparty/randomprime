# Quality of Life

Randomprime exposes various options all under the `"qol_"` prefix/category which serve to improve player quality of life when playing the game many times over. Many of these options come down to player preference.

## "Game Breaking" Fixes `"qolGameBreaking": true`

*Fixes issues originating from the base game or which directly conflict with randomized play. Also reverts "patches" applied to followup releases of the game.*

### NTSC-00
- Fix for research core access soft-lock
- Fix gravity chamber grapple point not respawning after room is unloaded

### Post-NTSC-00
- Remove invisible wall over arboretum gate
- Remove inventory check for thermal visor in ruined courtyard
- Remove door lock in main quarry
- Remove door lock and bendenzium rock in geothermal core

### Sequence break-induced Soft-Locks
- Fix suntower deleting flaahgra item
- Fix research lab aether wall not being destroyed after reseach core item
- Fix observatory puzzle not being solveable after research core item
- Fix observatory door lock soft-lock
- Fix mines security station door lock soft-lock
- Fix central dynamo crash
- Fix soft-lock in hive totem

### Randomizer-Induced Bugfixes
- Fix rooms with bad default spawn points
    - Missile Station Mines
    - Piston Tunnel
    - Ruined Fountain
    - Landing Site (small only)
    - Chozo -> Magmoor Elevator room (small only)
- Patch Metroid Prime (Exo + Essence) to remain dead permanently after killing once
- Fix processing center access crash
- Fix door in Ventillation Shaft Section B being permanently locked on 2nd pass
- Fix crash in Elite Quarters when obtaining a new suit before Omega Pirate death cutscene (multiworld)

## Cosmetic Improvements `"qolCosmetic": true`
- remove all of the item aquisition cutscenes (e.g. Space Jump)
- remove all but 1 of the file select background videos so that during races, everyone spawns into the game at the same RTA
- remove all but 1 of the attract videos to make copying the game to your wii faster
- skip item acquisition pop-up message
- make the morph ball HUD says `X/Y` instead of just `X`
- If impact crater is skipped, go straight to credits instead of watching the escape sequence
- Make the vines in arboretum still stay on the ghost layer so that players know to shoot the rune before scanning

## Logical Quality of Life

*There are several single-option quality of life options affecting logic which can be selected:*

- `"phazonEliteWithoutDynamo"`: Provide access to the Phazon Elite in Elite Research without the maze item
- `"mainPlazaDoor"`: Enable the etank ledge door in Main Plaza
- `"backwardsLabs"`: Scan the barrier control panel in research lab aether from both sides
- `"backwardsFrigate"`: Open the first powered door in frigate from the backside, even if unpowred.
- `"backwardsUpperMines"`: Automatically turn off main quarry barrier from the back
- `"backwardsLowerMines"`: Remove processing center access locks, remove elite quarters access lock, scan through Metroid Quarantine B barrier from both sides, turn off Metroid Quarantine A barrier when approaching from the back, scan through the elite control barrier from both sides

## Cutscene Remove `"qolCutscenes": "<level>"`

There are 4 cutscene levels to choose from:
- `"original"`: Same as vanilla
- `"competitive"`: Removes cutscenes (none of which reposition the player), that do not have any weird gameplay side-effects.
- `"minor"`: Removes cutscenes (none of which reposition the player), some of which moderately effect how the game is played after the cutscene is triggered.
- `"major"`: Removes as many cutscenes as possible without breaking the game or common exploits. Some cutscenes don't even speed up the game when removed. This is more of a novel setting.

## Scan Point Quality of Life
*In many rooms where you could not previously, you can now identify what an item is using scan visor. For example, the item in upper phendrana shorelines now has an extra scan point outside the tower near the item which mirrors the item's description.*
