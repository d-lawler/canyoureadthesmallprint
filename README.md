# smallpint

Checks the legal copy on ad artwork for WCAG contrast and writes a rationale legal can sign off.

Drop in a PNG, JPG or WebP. Smallpint:

1. **Works out what the artwork is** from the file alone: standard ad sizes (incl. @2x/@3x exports), embedded DPI for print and OOH (incl. artwork supplied at 10% or 25%), and aspect-ratio families. Only asks a question when the size matches nothing.
2. **Finds the legal copy** without OCR: the smallest consistent block of text on the artwork. The user can redraw the box.
3. **Measures it**: text colour, worst-case background (for gradients and photos), and type size converted to what a person actually sees (CSS pt on screens; reading-distance equivalent for OOH).
4. **Walks an internal rule tree** to AAA / AA / Fail and assembles a memo from pre-approved wording in [`crates/core/templates.toml`](crates/core/templates.toml).

Everything runs in the browser as WebAssembly. Artwork never leaves the device; there is no server and no LLM.

## Layout

```
crates/
  core/    the engine: context detection, text finding, measurement, rules, memo
  wasm/    wasm-bindgen API for the browser
  cli/     `smallpint <file>` for testing and batch checks
web/       Vite + Vue UI (engine runs in a Web Worker)
fixtures/  generated test artwork with known answers (python3 fixtures/make.py)
```

## Run it

Needs Rust (via rustup) and Node 22. wasm-pack comes from npm.

```sh
rustup target add wasm32-unknown-unknown

cargo test --workspace                 # unit + fixture regression tests
cargo run --release -p smallpint-cli -- fixtures/story_grey_fail.png

cd web && npm install && npm run dev   # builds the wasm, then starts Vite
```

**Deploy:** link the repo in Netlify and leave the build settings empty; `netlify.toml` builds the Rust engine to WebAssembly and publishes `web/dist`. GitHub Actions runs the tests on every push.

## How context detection works

`crates/core/src/context.rs` holds the format catalogue. In order of strength:

| Signal | Example | Result |
| --- | --- | --- |
| Exact ad size (×1, ×2, ×3) | 600×500 at 144dpi | MPU exported @2x, 300 CSS px wide |
| DPI → physical size | 7200×3600 at 300dpi = 610×305mm | 48-sheet supplied at 10%, seen from 15m |
| Aspect ratio | 720×1280 | Story (9:16), 390 CSS px wide |
| Nothing | 1234×777 | Ask: Phone / Web page / Billboard / Print |

72/96dpi and their @2x/@3x multiples are ignored as DPI evidence, since screen tools stamp them on everything. When two readings fit equally (a 48-sheet at 10% is the same file as a 12-sheet at 20%), the stricter one is the default and the other is offered under **Change**.

Type size: x-height is measured from the glyph row profile and divided by 0.56, the high end of real fonts, so the estimate errs small and the stricter normal-text threshold applies when in doubt.

## Changing the memo wording

Edit `crates/core/templates.toml` and bump `ruleset`. Every memo's footer records the ruleset, engine version and a hash of the file it assessed.

## Known limits (v0.1)

- PDF artwork isn't read yet (export a PNG/JPG at the intended DPI).
- Weight (bold) is only measured when glyphs are large enough; otherwise text is treated as regular.
- Text detection is heuristic. Very stylised or touching letters may need the box drawing by hand.
