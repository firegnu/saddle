# Clawd reference frames

Original artwork and animation source: Anthropic, <https://claude.dev/>, extracted 2026-10-01. `sources.json` records the source URLs, SHA-256 hashes and original frame counts. These are adaptations of the reference artwork, not newly authored characters.

`frames.bin` contains all 37 Lottie clips plus the decoded kite animation, sampled at 12 fps into an 18-column / 7-row terminal canvas. It preserves a character body of roughly 8 columns / 3 rows. Two colors and one Unicode quadrant glyph describe each cell. The transparent palette entry preserves the underlying surface; opaque cells replace underlying text/borders. Sampling inevitably loses details smaller than the terminal grid.

Regenerate with `scripts/prepare-clawd.py /path/to/extracted-reference` in a disposable Python environment containing `rlottie-python[full]==1.3.8` and `Pillow==12.3.0`. The input directory has the extraction manifest and original JSON animations. Python and Lottie are asset-preparation dependencies only; normal builds and runtime use the checked-in binary.

Binary format: `CLWD1`, four bytes for columns / rows / palette count / clip count, RGB palette triples, then clips. Each clip is a one-byte name length, ASCII name, little-endian u16 frame count and frame cells in row-major order. A cell is three bytes: quadrant mask (TL/TR/BL/BR bits), foreground palette index, background palette index. Palette index zero is transparent.
