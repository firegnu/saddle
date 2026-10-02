# Pet packs

One text file per pet. They are compiled into saddle; nothing is read from disk at runtime, and no other tool is needed to change them. `src/mascot.rs` parses them and its tests check every built-in pack.

## Format

A pack is TOML.

- `[palette]`: one letter per color, `"#rrggbb"`. `.` always means transparent.
- `[poses.<name>]`: one still picture on a canvas of 16 columns by 3 rows of terminal cells.
  - `art`: six rows of 32 letters. Each cell is 2x2 "bricks", drawn with quarter-block glyphs. A cell holds at most two colors, counting transparent.
  - `cells` (optional): `[column, row, glyph, fg, bg]` replaces one whole cell with a glyph one column wide. Used for eyes and for small details the bricks cannot draw.
- `[[clip]]`: `name`, and `frames` as `[pose, ticks]` pairs. Twelve ticks are one second. `walking` loops while the pet moves, `turning` plays at each end of its lane, and every other clip is an action picked at random between walks.

The canvas sits in the spare space right of the tabs, above the pane border. The floor is the bottom of the canvas and the height never changes.

## Clawd

Original character and gesture references: Anthropic, <https://claude.dev/>, extracted 2026-10-01. `clawd-sources.json` keeps the reference URLs, SHA-256 hashes and original frame counts. The poses are hand-timed grid adaptations, not frame-for-frame conversions; the pirate costume and choreography are original additions requested by the user.

The approved rest pose is kept in every action: actions enter and leave it, the waist and four feet stay planted, and glances slide only the head and arm rows so a skin margin stays outside both eyes.
