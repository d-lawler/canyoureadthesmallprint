# can you read the small print?

a small tool that checks whether the legal copy on an ad is legible enough against the WCAG contrast rules, and writes a short note you can hand to legal.

[canyoureadthesmallprint.netlify.app](https://canyoureadthesmallprint.netlify.app)

drop in an image, click the small print (or drag a box around it), and it tells you **AAA**, **AA** or **below AA**, with the reasoning. it all runs in your browser; the image never leaves your machine.

a caveat up front: this was vibe coded with claude over an evening. the maths is the standard WCAG maths and there are tests, but it's a sketch rather than an audit tool. if you spot something wrong, issues and PRs are very welcome.

## the maths

all of this comes from the WCAG 2.x spec. nothing here is new.

**1. colour to luminance.** each sRGB channel (0 to 1) is linearised, then weighted by how sensitive our eyes are to it (green most, blue least):

```
c ≤ 0.04045  →  c / 12.92
otherwise    →  ((c + 0.055) / 1.055) ^ 2.4

L = 0.2126 R + 0.7152 G + 0.0722 B
```

**2. contrast ratio.** lighter over darker, with a small offset for ambient light. it runs from 1:1 (same colour) to 21:1 (black on white):

```
ratio = (L_lighter + 0.05) / (L_darker + 0.05)
```

**3. thresholds.**

|              | AA    | AAA   |
| ------------ | ----- | ----- |
| normal text  | 4.5:1 | 7:1   |
| large text (18pt+, or 14pt+ bold) | 3:1 | 4.5:1 |

that's the easy part. the fiddly bits are around the maths:

- **which pixels are text?** inside the box you select, pixels are split into two groups by brightness ([Otsu's method](https://en.wikipedia.org/wiki/Otsu%27s_method)). the text colour is the median of the most solid 30% of text pixels, so anti-aliased edges don't drag it towards the background.
- **which background?** the box is cut into tiles and every tile behind text is measured. the worst one is reported, so white text over a photo is judged on its weakest spot, not an average.
- **how big is the text?** the x-height is measured from the rows of text pixels and divided by 0.56, the high end for real fonts. that way the estimate errs small, which means stricter thresholds when in doubt. ratios are truncated, never rounded up across a threshold.
- **how big will it look?** this comes from the file. standard ad sizes (including @2x exports), embedded DPI for print (including billboard artwork supplied at 10%), or aspect ratio. on screens, `pt = image px × (screen css width ÷ image width) × 0.75`. for billboards the text is scaled to the same visual angle at a 40cm reading distance (`size × 0.4m ÷ viewing distance`). it's a rough benchmark, since WCAG doesn't formally cover print.

## what it doesn't do

- it's not legal advice. it checks contrast and size; it doesn't judge whether the wording is enough, or apply ASA / CAP / FTC prominence rules (beyond flagging very small text).
- it doesn't read the text (no OCR). it knows *where* the copy is, not *what* it says.
- PDFs aren't supported yet. export a PNG or JPG at the intended DPI.
- bold detection is rough, so text is treated as regular weight unless it's clearly bold.
- the text finder is heuristic. photo texture can produce stray outlines, and very stylised lettering may need a hand-drawn box.

## how it's built

the analysis is rust, compiled to WebAssembly and run in a web worker. the interface is a small vue app. there's no server; netlify just hosts the files.

```
crates/
  core/    the engine: format detection, text finding, measurement, rules, memo
  wasm/    the browser bindings
  cli/     same engine on the command line, handy for testing
web/       vue + vite interface
fixtures/  generated test artwork with known answers
```

the memo is assembled from fixed sentences in [`crates/core/templates.toml`](crates/core/templates.toml), picked by the rules. there's no AI in the app itself. the same image always gives the same note, and the wording can be reviewed once and versioned (`ruleset` in that file). every note records the ruleset, version and a hash of the image.

## run it

needs rust (via rustup) and node 22.

```sh
rustup target add wasm32-unknown-unknown

cargo test --workspace                   # unit + fixture tests
cargo run --release -p smallpint-cli -- fixtures/story_grey_fail.png

cd web && npm install && npm run dev     # builds the wasm, starts vite
```

netlify picks up `netlify.toml`, so linking the repo is all the setup it needs.
