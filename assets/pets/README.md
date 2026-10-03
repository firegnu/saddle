# Pet packs

Two text files per pet: `<name>.toml` draws it with block glyphs, which every terminal shows, and `<name>-image.toml` draws it as pixel images for terminals with the Kitty graphics protocol (Ghostty, kitty, WezTerm and others). Saddle asks the terminal at startup and uses the images when it answers, the glyphs otherwise. The packs are compiled into saddle; nothing is read from disk at runtime. `src/mascot.rs` parses them and its tests check every built-in pack.

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

An image pack sets `size = [width, height]` and draws each pose as `pixels`: `height` rows of `width` palette letters, one per pixel, `.` transparent, with no `art` or `cells`. Saddle scales the picture with nearest-neighbour sampling to fit the 16x3 cell canvas, keeping its shape, standing on the floor in the middle. Clips, `step_ticks` and `mirror` work as above.

## Clawd (`clawd.toml`)

Original character and gesture references: Anthropic, <https://claude.dev/>, extracted 2026-10-01. `clawd-sources.json` keeps the reference URLs, SHA-256 hashes and original frame counts. The poses are hand-timed grid adaptations, not frame-for-frame conversions; the pirate costume and choreography are original additions requested by the user.

The approved rest pose is kept in every action: actions enter and leave it, the waist stays planted, and glances slide only the head and arm rows so a skin margin stays outside both eyes. Props follow the originals in scale: small, held at the hand or set by the feet, each in its own color, cut with eighth blocks where a whole brick would be too coarse. Its eyes sit to one side of their cells, so a mirror image would not line up; walking and turning to the left are drawn as their own clips and actions are not mirrored. Walking moves only the legs: relative to where it stands, the head, eyes, arms and body keep still on every frame of the walk.

## Cat (`cat.toml`)

An original design for saddle: a small orange cat with a large head, a face turned to the viewer and a pink nose. It is drawn facing right and mirrored. The sleeping pose rounds its back with eighth blocks and breathes. Its actions: sit, groom, look, stretch, yarn and sleep.
## Clawd as images (`clawd-image.toml`)

Converted frame by frame from the claude.dev references in `clawd-sources.json` by `examples/clawd_pixels.rs`; regenerate it rather than edit it:

```sh
cargo run --example clawd_pixels -- <reference dir> > assets/pets/clawd-image.toml
```

The references are pixel art on a 50-unit grid at 12 frames per second, like saddle, so each frame is sampled exactly and keeps its tick. The pack is the bottom 23 of the 37 pixel rows, 55x23 pixels across the canvas, and holds the 20 actions that fit in that height. The reference walk bobs the whole body; the pack keeps the standing three-quarter body of the turn and takes only the legs from each frame of the walk cycle. The turn faces the viewer and blinks. Every clip, actions included, is flipped when Clawd heads left.

## Cat as images (`cat-image.toml`)

An original orange cat at Clawd's pixel size, drawn for pictures rather than copied from `cat.toml`: a dark brown outline, a round head about three fifths of its height, big eyes with a white shine, a white muzzle with a small mouth, pink cheeks and inner ears, and short legs. It has the poses and clip timing of `cat.toml`. While it walks only the legs move; the head, body and tail keep still. Previews and the earlier concepts: `docs/调研/图片猫猫候选.md`.

## Capybara (`capybara.toml`, `capybara-image.toml`)

An original design for saddle: a sleepy, unhurried capybara in side view, with a round barrel body, short legs, a small round ear, a broad blunt snout with a dark nose, half-lidded eyes and pink cheeks. Both packs face right and are mirrored, and they share clip names and timing. It walks a cell every 6 ticks, slower than the cat, and only its legs move while it walks. Its actions: daze, munch a leaf, nap, balance an orange and let a bird land on its back.

The image pack is 55x24 pixels with the outline and colors of `cat-image.toml`. The picture is limited by the canvas width, so it shows at the same scale as Clawd and the cat; the extra row is headroom for the orange and the bird. The block pack has no outline and fewer details: the orange appears and leaves without falling, because one cell cannot hold the orange, the gap below it and the head. Previews: `docs/调研/卡皮巴拉宠物.md`.
