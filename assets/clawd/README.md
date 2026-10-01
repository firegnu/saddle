# Curated Clawd poses

Original character and gesture references: Anthropic, <https://claude.dev/>, extracted 2026-10-01. `sources.json` retains the selected source URLs, SHA-256 hashes and original frame counts. The running animations are hand-timed grid adaptations, not frame-for-frame Lottie conversions.

The user selected 12 motions: walking, turning, looking, waving, thinking, coffee, notebook, headphones, watch, snooze, sunglasses and swaying. `frames.bin` contains only these clips. The approved rest pose is preserved, with deliberate foot/arm/eye changes and small props instead of independently resampling moving outlines. Actions enter and leave the rest pose. All 422 frames fit a 16-column / 3-row canvas, above the agent border; the floor and pane geometry remain fixed.

Regenerate with `python3 scripts/prepare-clawd.py` (standard library only). Key poses and their hold durations live in that script. Python is only an asset preparation tool; builds and runtime use the checked-in binary. Ordinary text/block glyphs remain font dependent; no image protocol or terminal-specific code is required.

Binary format: `CLWD3`, four bytes for columns / rows / palette count / clip count, RGB palette triples, then clips. Each clip is a one-byte name length, ASCII name, little-endian u16 frame count and row-major frame cells. Each cell is three bytes: glyph, foreground palette index, background palette index. Glyphs 0–15 are quadrant masks (TL/TR/BL/BR); 16 = `▪` (open eye), 17 = `–` (closed eye), 18 = `z`, 19 = `─` (glasses bridge), 20 = `■` (inset sunglasses lens). Palette index zero is transparent.
