# can you read the small print?

a small tool that checks whether the legal copy on an ad is legible enough, and writes a short note you can hand to legal.

[canyoureadthesmallprint.netlify.app](https://canyoureadthesmallprint.netlify.app)

drop in an image, click the small print (or drag a box around it), and it tells you **AAA**, **AA** or **below AA**, with the reasoning. it all runs in your browser, so the image never leaves your machine.

this was vibe coded with claude over an evening, so treat it as a handy sketch rather than an audit tool. if something looks off, issues and PRs are very welcome.

## how it works, if you're curious

it's mostly one bit of maths from the WCAG spec. every colour gets turned into a "relative luminance" (how bright it looks to your eye, green counts for a lot, blue barely at all). then the text and background are compared as a ratio:

```
(lighter + 0.05) / (darker + 0.05)
```

that gives you anything from 1:1 (same colour) to 21:1 (black on white). small text needs 4.5:1 for AA and 7:1 for AAA. big text gets a bit more leeway.

the maths is the easy bit. the fun problems are around it:

- **figuring out which pixels are text.** inside your box it splits the pixels into light and dark, and takes the text colour from the solid middle of the letters rather than the soft anti-aliased edges.
- **text over photos.** it checks the background in little tiles and reports the worst one. white text that's fine over a dark sleeve but disappears over a bright window gets judged on the window.
- **how big the text really is.** a 1080px instagram story ends up about 390px wide on a phone, so text that looks fine in figma can be tiny in real life. it works this out from the file: standard ad sizes, @2x exports, and DPI for print (it can tell a billboard supplied at 10% scale). for billboards it scales by viewing distance, so text on a 48-sheet seen from 15m is treated like much smaller text held in your hand.
- **erring on the strict side.** when it has to guess (font size, weight, rounding) it guesses in the direction that makes passing harder.

the note for legal is stitched together from fixed sentences in [`crates/core/templates.toml`](crates/core/templates.toml), with no AI in the app. the same image always gives the same note, and the wording can be checked once and versioned.

## what it doesn't do

- it's not legal advice. it checks contrast and size, not whether the wording is enough.
- it doesn't read the text. it knows *where* the small print is, not *what* it says.
- no PDFs yet. export a PNG or JPG.
- the text finder sometimes outlines bits of photo too. just ignore those.

## running it

the engine is rust compiled to WebAssembly, the interface is a small vue app, and netlify hosts it. there's no server.

```sh
rustup target add wasm32-unknown-unknown
cargo test --workspace                 # tests
cd web && npm install && npm run dev   # local site
```

there's also a command-line version for checking files in bulk: `cargo run --release -p smallpint-cli -- some-ad.png`

## licence

MIT, do what you like with it. see [LICENSE](LICENSE).
