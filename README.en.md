# Sine qua non (Rust)

An **independent, from-scratch Rust rewrite** of the core of the curve plotter
**Sine qua non** — the historical French (Delphi) software created by
**Patrice Rabiller** in 2004 and co-written by **Patrick Pradeau** since 2010,
built for mathematics teachers and their students.

> This repository is not the original project: it is a brand-new program
> written from zero (no Delphi code reused). The original Sine qua non is
> still downloadable from
> [Patrice Rabiller's website](http://www.patrice-rabiller.fr/SineQuaNon/menusqn.htm).

**Readme en français : [README.md](README.md)**

![Example](docs/exemple.png)

## v0.3 status — sign and variation tables

| Included | Details |
|---|---|
| Curves | functions `y = f(x)`, parametric curves, polar curves |
| Axes | arbitrary origin, unit in mm, linear **or logarithmic** scales |
| Grid | **5 × 5 mm** minor squares like the original, page up to A0 |
| Points | placed on any curve (function, parametric, polar) with displayed coordinates |
| Tangents | at a point of a function — numeric derivative (central differences), displayed `y = mx + p` equation |
| Integrals | **hatched** areas between the curve and the x-axis, displayed `∫ ≈` value (trapezoid rule) |
| **Tables** | **text code faithful to the original**: cells `[a:b:c]`, separators `|` `\|\|` `\|0` `0`, single-bar fallback, arrows `/:>` `\:>` `_>`, hatching `//`, macros `\infty` `\sqrt{}` `\frac{a}{b}` `\array{}` `\text{}` `\left\|` — rendered with bars **per row**, zeros **on** the bars, values **centred on** the boundaries, stretched arrows with clearance; the original's 10 examples included (`Var1…Var8`, `Sgn`, `Abs`) |
| **Generator** | **“⚡ From a curve”**: zeros of f′ by scanning + bisection, signs, f values at critical points and endpoints, arrows — generated code remains **editable** |
| Interface | graphical (egui), **bilingual English / French** (selector, persisted), wheel = zoom, drag = pan, auto-saved session (curves + language + annotations **+ tables**) |
| Export | **SVG** (mm, printable 1:1, vector labels and hatching) and **PNG** (hatching, markers, bitmap text) — tables included |
| Analysis | calculator syntax, implicit multiplication (`2x`, `3sin(x)`), asymptote clipping |

**Not yet included** (roadmap, one module per release): statistics, probability
distributions (binomial, normal), weighted probability trees,
sequences, extrema/zeros, LaTeX expressions, BMP/GIF/EMF/EPS export,
Bézier curves, printing, global preferences.

## Installation

### Linux — any distribution (one line)

```bash
curl -fsSL https://raw.githubusercontent.com/devfrp/sinequanon/main/install.sh | bash
```

- The **gnu** build (dynamic glibc linking) is installed first: the graphical
  interface works (wayland/GL loaded dynamically); the **static musl** binary
  is a fallback, with an opt-in `--static` flag for servers/containers
  (CLI only — a 100% static binary cannot load the graphics libraries).
- SHA-256 checksum verified automatically; installs into `/usr/local/bin`
  (or `~/.local/bin` without root).
- **Public** repository: no token needed. On a private mirror, prefix the
  `curl` with `GITHUB_TOKEN=…` (or run `gh auth login`).
- Alternatives: `curl … | bash -s -- --bin-dir ~/bin`, `--version v0.2.0`
  or `--static`.

> If the `raw.githubusercontent.com` CDN serves you a stale script (rare but
> seen), bypass it:
> `curl -fsSL -H "Accept: application/vnd.github.raw" https://api.github.com/repos/devfrp/sinequanon/contents/install.sh | bash`

### Windows — `.exe`

Download `sinequanon-windows-x86_64.exe` from the
[Releases](https://github.com/devfrp/sinequanon/releases) page (MSVC build
automated by GitHub Actions).

### macOS — universal binary

Download `sinequanon-macos-universal.tar.gz` (arm64 + x86_64 merged via
`lipo`), then:

```bash
tar -xzf sinequanon-macos-universal.tar.gz && xattr -d com.apple.quarantine sinequanon 2>/dev/null; ./sinequanon
```

### From source

```bash
cargo build --release     # ~8 min; then target/release/sinequanon
cargo test                # 44 tests
```

## Usage

```bash
sinequanon                          # graphical interface
sinequanon --lang en                # graphical interface in English
sinequanon export -f "sin(x)" -f "x^2/4-2" --svg curve.svg
sinequanon export -p "2+cos(theta)" --png curve.png --px-par-mm 6
sinequanon export -c "cos(t);sin(t)/2" --t-min -6.28 --t-max 6.28
sinequanon export -f "log(x)" --log-x --png log.png
sinequanon export -f "x^2" --aire 0:0:2 --tangente 0:2 --point 0:1 --svg tp.svg
sinequanon --aide
```

- `--point C:x`: point on curve #`C` at abscissa `x` (or at parameter
  `t`/`θ` for parametric/polar curves);
- `--tangente C:x`: tangent to function #`C` at `x`;
- `--aire C:a:b`: hatched integral of function #`C` between `a` and `b`.

(Options are French for historical parity with the original software;
`aire` = *area*, `tangente` = *tangent*, `point` = *point*.)

Expression syntax: `+ - * / ^`, functions `sin cos tan asin acos atan
atan2 sinh cosh tanh exp ln log log2 sqrt abs floor ceil round sign min max
mod pow`, constants `pi` (or `π`) and `e`, variables `x` (function), `t`
(parametric), `theta` (or `θ`, polar). Implicit multiplication accepted.

## Continuous integration (GitHub Actions)

The workflow [`.github/workflows/build.yml`](.github/workflows/build.yml):

1. **tests** — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` on every push to `main`;
2. **builds** — on `v*` tags or manual dispatch (`workflow_dispatch`):
   - Windows `x86_64-pc-windows-msvc` → **`sinequanon-windows-x86_64.exe`**,
   - macOS arm64 + x86_64 → **`sinequanon-macos-universal.tar.gz`** (via `lipo`),
   - Linux `x86_64` **gnu** (GUI) and **static musl** (CLI), `aarch64` gnu and musl (via `cross`, non-blocking);
3. **release** — on `v*` tags: GitHub Release publication with all files
   + `SHA256SUMS`.

```bash
gh workflow run build                 # manual build
git tag v0.2.0 && git push --tags     # automatic build + release
```

## Architecture

- **100% Rust** (`#![forbid(unsafe_code)]`): [egui/eframe](https://github.com/emilk/egui) interface,
  expression parser/evaluator, adaptive sampling, Liang-Barsky clipping,
  PNG encoder (stored-deflate blocks) and SVG generator all written from
  scratch — zero compiled C dependencies.
- `src/expr.rs` parser/evaluator · `src/grid.rs` mm axes ·
  `src/curve.rs` curves + line styles · `src/annot.rs` points/tangents/
  integrals (central derivative, trapezoids, hatching) · `src/tableau.rs`
  sign/variation tables (original grammar) · `src/tabgen.rs` generator
  from a curve · `src/render.rs` SVG/PNG ·
  `src/png.rs` raster + encoder + 5×7 bitmap font · `src/i18n.rs` bilingual
  strings · `src/app.rs` interface · `src/session.rs` JSON session.

## Roadmap (option-by-option parity with the original 2.9.4)

- ✅ v0.1 — plotting core: curves, axes, 5 × 5 mm grid, SVG/PNG, session
- ✅ v0.2 — graphical module: points, tangents, hatched integrals + bilingual interface
- ✅ v0.3 — sign and variation tables (original's text code) + generation from a curve
- ⬜ statistics (data series, histogram, frequency polygon, box plot)
- ⬜ probability (weighted trees, binomial distribution, normal distribution)
- ⬜ numeric sequences (staircase mode)
- ⬜ automatic extrema and zeros
- ⬜ BMP/GIF/EMF/EPS/LaTeX exports, Bézier curves, preferences, printing

## Credits and license

- Rust code: independent project under the **MIT** license ([LICENSE](LICENSE)).
- Credit for the concept: **Patrice Rabiller** and **Patrick Pradeau** for
  [Sine qua non](http://www.patrice-rabiller.fr/SineQuaNon/menusqn.htm)
  (freeware, Delphi sources published in 2010). Thanks to them for the idea,
  the implicit specification and the 5 × 5 mm grid.
