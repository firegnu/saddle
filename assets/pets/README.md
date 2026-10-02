# Pet packs

One text file per pet. They are compiled into saddle; nothing is read from disk at runtime, and no other tool is needed to change them. `src/mascot.rs` parses them and its tests check every built-in pack.

A pet lives in the spare space right of the tabs, above the pane border, on a canvas of 16 columns by 3 rows of terminal cells. The floor is the bottom of the canvas and the height never changes. It walks back and forth and stops now and then to play one action.

## Format

A pack is TOML.

- `step_ticks`: ticks per cell of travel while walking. Twelve ticks are one second. The walking clip must be a whole number of steps long, and should change pose on the same ticks, so the body and the feet move together.
- `mirror`: `true` flips every pose left to right when the pet heads left. Use it for a pet drawn facing right.
- `[palette]`: one letter per color, `"#rrggbb"`. `.` always means transparent.
- `[poses.<name>]`: one still picture.
  - `art`: six rows of 32 letters. Each cell is 2x2 "bricks", drawn with quarter-block glyphs. A cell holds at most two colors, counting transparent.
  - `cells` (optional): `[column, row, glyph, fg, bg]` replaces one whole cell with a glyph one column wide. The glyph's shape takes `fg` and the rest of the cell takes `bg`; either may be `.`. This is how eyes are drawn, and how the eighth blocks (`▁▂▃▄▅▆▇`, `▏▎▍▌▋▊▉`) cut a cell at one eighth precision for curves, thin lines and small props.
- `[[clip]]`: `name`, and `frames` as `[pose, ticks]` pairs.
  - `walking` loops while the pet moves.
  - `turning` plays at each end of the lane. The pet changes heading halfway through it.
  - `<name>-left`, when present, is used instead of `<name>` while the pet heads left, and must last as long. It takes the place of mirroring for that clip.
  - Every other clip is an action, picked at random between walks.

Only block glyphs and plain characters are used, so a pack looks the same in any terminal. Characters whose look depends on the font (`♥`, `♪`, `○`) are best avoided in new art.

## Clawd (`clawd.toml`)

Original character and gesture references: Anthropic, <https://claude.dev/>, extracted 2026-10-01. `clawd-sources.json` keeps the reference URLs, SHA-256 hashes and original frame counts. The poses are hand-timed grid adaptations, not frame-for-frame conversions; the pirate costume and choreography are original additions requested by the user.

The approved rest pose is kept in every action: actions enter and leave it, the waist stays planted, and glances slide only the head and arm rows so a skin margin stays outside both eyes. Props follow the originals in scale: small, held at the hand or set by the feet, each in its own color, cut with eighth blocks where a whole brick would be too coarse. Its eyes sit to one side of their cells, so a mirror image would not line up; walking and turning to the left are drawn as their own clips and actions are not mirrored.

## Cat (`cat.toml`)

An original design for saddle: a small gray cat with a large head, a face turned to the viewer and a pink nose. It is drawn facing right and mirrored. The sleeping pose rounds its back with eighth blocks and breathes.
