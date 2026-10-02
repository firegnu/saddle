//! Converts the claude.dev Clawd reference animations into `assets/pets/clawd-image.toml`:
//!
//! ```sh
//! cargo run --example clawd_pixels -- <reference dir> > assets/pets/clawd-image.toml
//! ```
//!
//! The reference directory holds the Lottie files under `animations/`, as extracted on
//! 2026-10-01; each is checked against the SHA-256 in `assets/pets/clawd-sources.json`. They are
//! pixel art: filled polygons on a 50-unit grid, switched on and off by hold keyframes on fill
//! opacity, at 12 frames per second like saddle. Each frame is sampled at its pixel centres, so
//! the conversion is exact, and every frame keeps its own tick.
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt::Write, path::Path};

/// The bottom 23 of the 37 pixel rows fill the three-row lane; the 55 columns cover the canvas.
const WIDTH: usize = 55;
const HEIGHT: usize = 23;
const GRID: f64 = 50.0;
/// Palette letters; the references use 61 colors.
const LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const LEFT: f64 = -14.0;
const TOP: f64 = 700.0;
/// Actions whose every frame fits in the lane, in the order the pack lists them.
const ACTIONS: [&str; 20] = [
    "looking",
    "waving",
    "coffee",
    "notebook",
    "watch",
    "snooze",
    "sunglasses",
    "excited",
    "phone",
    "laptop",
    "meditating",
    "hulahoop",
    "desktop",
    "arcade",
    "watering",
    "berrypicking",
    "blooming",
    "dancing",
    "dancinghappy",
    "swaying",
];

struct Clip {
    frames: u64,
    layers: Vec<Value>,
}
impl Clip {
    fn load(dir: &Path, name: &str, sources: &[Value]) -> Result<Self> {
        let source = sources
            .iter()
            .find(|s| s["name"] == name)
            .with_context(|| format!("{name} is not in clawd-sources.json"))?;
        let url = source["url"].as_str().context("url")?;
        let file = dir.join("animations").join(url.rsplit('/').next().unwrap());
        let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
        let sum: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        ensure!(
            source["sha256"] == sum,
            "{} does not match its SHA-256",
            file.display()
        );
        let lottie: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            lottie["fr"].as_f64() == Some(12.0),
            "{name}: expected 12 frames per second"
        );
        Ok(Self {
            frames: lottie["op"].as_u64().context("op")?,
            layers: lottie["layers"].as_array().context("layers")?.clone(),
        })
    }
    /// The frame as color names per pixel, `rows` pixel rows from `top`; later layers and
    /// earlier shapes are underneath.
    fn frame(&self, t: u64, top: f64, rows: usize) -> Result<Vec<Option<String>>> {
        let mut pixels = vec![None; WIDTH * rows];
        for layer in self.layers.iter().rev() {
            if !(layer["ip"].as_f64().unwrap_or(0.0)..layer["op"].as_f64().unwrap_or(f64::MAX))
                .contains(&(t as f64))
            {
                continue;
            }
            still(&layer["ks"]["p"], &[0.0, 0.0, 0.0])?;
            let groups = layer["shapes"].as_array().context("shapes")?;
            for group in groups.iter().rev() {
                let items = group["it"].as_array().context("group items")?;
                let mut polygons = Vec::new();
                let mut fill = None;
                for item in items {
                    match item["ty"].as_str() {
                        Some("sh") => {
                            ensure!(item["ks"]["a"] == 0, "animated path");
                            let path = &item["ks"]["k"];
                            let curved = ["i", "o"].iter().any(|k| {
                                path[k].as_array().is_some_and(|tangents| {
                                    tangents
                                        .iter()
                                        .filter_map(Value::as_array)
                                        .flatten()
                                        .any(|v| v.as_f64() != Some(0.0))
                                })
                            });
                            ensure!(!curved, "curved path");
                            let points: Vec<(f64, f64)> = path["v"]
                                .as_array()
                                .context("vertices")?
                                .iter()
                                .map(|v| (v[0].as_f64().unwrap(), v[1].as_f64().unwrap()))
                                .collect();
                            polygons.push(points);
                        }
                        Some("fl") => {
                            ensure!(item["c"]["a"] == 0, "animated color");
                            let opacity = held(&item["o"], t)?;
                            let c = &item["c"]["k"];
                            let channel = |i: usize| (c[i].as_f64().unwrap() * 255.0).round() as u8;
                            fill = (opacity > 0.0).then(|| {
                                format!("#{:02x}{:02x}{:02x}", channel(0), channel(1), channel(2))
                            });
                        }
                        Some("tr") => {
                            still(&item["p"], &[0.0, 0.0])?;
                            still(&item["s"], &[100.0, 100.0])?;
                        }
                        other => bail!("unsupported shape {other:?}"),
                    }
                }
                let Some(color) = fill else { continue };
                for (i, pixel) in pixels.iter_mut().enumerate() {
                    let x = LEFT + GRID * ((i % WIDTH) as f64 + 0.5);
                    let y = top + GRID * ((i / WIDTH) as f64 + 0.5);
                    if polygons.iter().filter(|p| inside(p, x, y)).count() % 2 == 1 {
                        *pixel = Some(color.clone());
                    }
                }
            }
        }
        Ok(pixels)
    }
    /// The frame in the lane, refusing one that reaches above it.
    fn lane_frame(&self, t: u64) -> Result<Vec<Option<String>>> {
        let above = (TOP / GRID) as usize;
        ensure!(
            self.frame(t, 0.0, above)?.iter().all(Option::is_none),
            "frame {t} does not fit in the lane"
        );
        self.frame(t, TOP, HEIGHT)
    }
}
fn still(property: &Value, expected: &[f64]) -> Result<()> {
    let value: Vec<f64> = match &property["k"] {
        Value::Array(v) => v.iter().filter_map(Value::as_f64).collect(),
        Value::Null => return Ok(()),
        other => vec![other.as_f64().context("number")?],
    };
    ensure!(
        property["a"] == 0 && value == expected,
        "unsupported transform"
    );
    Ok(())
}
/// A value with hold keyframes at frame `t`.
fn held(property: &Value, t: u64) -> Result<f64> {
    let number = |v: &Value| match v {
        Value::Array(a) => a[0].as_f64(),
        v => v.as_f64(),
    };
    if property["a"] == 0 {
        return number(&property["k"]).context("value");
    }
    let keys = property["k"].as_array().context("keyframes")?;
    let mut value = number(&keys[0]["s"]).context("keyframe")?;
    for (i, key) in keys.iter().enumerate() {
        ensure!(key["h"] == 1 || i + 1 == keys.len(), "eased keyframe");
        if key["t"].as_f64().context("time")? <= t as f64 {
            value = number(&key["s"]).context("keyframe")?;
        }
    }
    Ok(value)
}
fn inside(polygon: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut odd = false;
    let mut j = polygon.len() - 1;
    for (i, &(xi, yi)) in polygon.iter().enumerate() {
        let (xj, yj) = polygon[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// Poses shared between clips by content, and the colors behind the palette letters.
#[derive(Default)]
struct Pack {
    colors: Vec<String>,
    names: HashMap<String, String>,
    poses: String,
    clips: String,
    count: HashMap<String, usize>,
}
impl Pack {
    fn pose(&mut self, clip: &str, pixels: &[Option<String>]) -> Result<String> {
        let mut art = String::new();
        for row in pixels.chunks(WIDTH) {
            for pixel in row {
                art.push(match pixel {
                    None => '.',
                    Some(color) => {
                        let index = match self.colors.iter().position(|l| l == color) {
                            Some(index) => index,
                            None => {
                                self.colors.push(color.clone());
                                self.colors.len() - 1
                            }
                        };
                        LETTERS.chars().nth(index).context("too many colors")?
                    }
                });
            }
            art.push('\n');
        }
        if let Some(name) = self.names.get(&art) {
            return Ok(name.clone());
        }
        let n = self.count.entry(clip.into()).or_default();
        *n += 1;
        let name = format!("{clip}-{n}");
        write!(self.poses, "\n[poses.{name}]\npixels = '''\n{art}'''\n")?;
        self.names.insert(art, name.clone());
        Ok(name)
    }
    fn clip(&mut self, name: &str, frames: &[String]) -> Result<()> {
        let mut runs: Vec<(&str, usize)> = Vec::new();
        for pose in frames {
            match runs.last_mut() {
                Some((last, count)) if last == pose => *count += 1,
                _ => runs.push((pose, 1)),
            }
        }
        let runs: Vec<_> = runs
            .iter()
            .map(|(pose, count)| format!("[\"{pose}\", {count}]"))
            .collect();
        write!(
            self.clips,
            "\n[[clip]]\nname = \"{name}\"\nframes = [{}]\n",
            runs.join(", ")
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let dir = std::env::args()
        .nth(1)
        .context("usage: clawd_pixels <reference dir>")?;
    let dir = Path::new(&dir);
    let sources: Vec<Value> =
        serde_json::from_str(include_str!("../assets/pets/clawd-sources.json"))?;
    let mut pack = Pack::default();

    // The reference walk bobs the whole body. Clawd keeps the standing three-quarter body of the
    // turn and takes only the legs, the bottom four rows, from each frame of the walk cycle.
    let turning = Clip::load(dir, "turning", &sources)?;
    let stand = turning.lane_frame(8)?;
    let front = turning.lane_frame(0)?;
    let blink = turning.lane_frame(4)?;
    let walking = Clip::load(dir, "walking", &sources)?;
    let legs = (HEIGHT - 4) * WIDTH;
    let mut steps = Vec::new();
    for t in 5..=9 {
        let mut frame = walking.lane_frame(t)?;
        frame[..legs].clone_from_slice(&stand[..legs]);
        steps.push(pack.pose("walk", &frame)?);
    }
    pack.clip("walking", &steps)?;
    // The turn faces the viewer and blinks; its second half is flipped to the new heading.
    let stand = pack.pose("stand", &stand)?;
    let front = pack.pose("front", &front)?;
    let blink = pack.pose("blink", &blink)?;
    let turn = [&stand, &stand, &stand, &front, &front, &blink];
    let turn: Vec<String> = turn
        .iter()
        .chain(turn.iter().rev())
        .map(|p| p.to_string())
        .collect();
    pack.clip("turning", &turn)?;
    for name in ACTIONS {
        let clip = Clip::load(dir, name, &sources)?;
        let frames = (0..clip.frames)
            .map(|t| pack.pose(name, &clip.lane_frame(t).with_context(|| name.to_owned())?))
            .collect::<Result<Vec<_>>>()?;
        pack.clip(name, &frames)?;
    }

    println!(
        "# Clawd as {WIDTH}x{HEIGHT} pixel images on the 16x3 cell canvas. See README.md for the format."
    );
    println!(
        "# Generated by `cargo run --example clawd_pixels -- <reference dir>` from the claude.dev"
    );
    println!("# reference animations in clawd-sources.json. Regenerate rather than edit by hand.");
    println!("step_ticks = 5\nmirror = true\nsize = [{WIDTH}, {HEIGHT}]\n\n[palette]");
    for (letter, color) in LETTERS.chars().zip(&pack.colors) {
        println!("{letter} = \"{color}\"");
    }
    print!("{}", pack.poses);
    println!("\n# The body moves one cell every `step_ticks`; the legs change on every tick.");
    print!("{}", pack.clips);
    Ok(())
}
