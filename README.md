# Parchment

> Turn plain Markdown into beautiful pages. One Rust core, every platform.

Parchment renders `.md` files into a polished graphical view and exports them
as clean **HTML + CSS** using swappable templates. It has first-class
**RTL / Persian** support: the document direction and language are detected
automatically, every block gets `dir="auto"`, and code always stays left-to-right.

| Where | What | How |
|-------|------|-----|
| Terminal / **Termux** | `parch` CLI (single ~3 MB binary) | `cargo install --path crates/mdr-cli` |
| Desktop | Live-preview app (Tauri 2) | `cd app && cargo tauri build` |
| Android | Same app as desktop | `cd app && cargo tauri android build` |
| Web | Static site, runs fully in the browser (WASM) | `scripts/build-web.sh` |

Everything is powered by the same crate, [`mdr-core`](crates/mdr-core), so the
output is identical on every platform.

## Features

- Tables, task lists, footnotes, strikethrough, heading attributes (`## Title {#id .class}`)
- Syntax highlighting for fenced code (theme-aware, works offline)
- Automatic table of contents (shown for longer documents, or force it with `--toc`)
- Unicode heading anchors (Persian headings link correctly)
- Optional Persian digits (`--fa-digits`), never applied inside code
- 4 built-in templates: `github`, `minimal`, `dark`, `magazine` (light/dark aware)
- Your own templates: a folder with `template.html` + `style.css`
- **Folder mode**: converts a whole directory into a static site, rewrites
  `*.md` links to `*.html`, copies images and other assets, builds an index page
- Front matter (`title`, `lang`, `dir`, `toc`)
- Output as one self-contained file or `index.html` + `style.css`
- `--watch` to rebuild on save; `--no-raw-html` to neutralise HTML in untrusted Markdown

## CLI

```bash
parch README.md                      # -> README.html next to the input (CSS embedded)
parch README.md -t magazine -o out/  # -> out/README.html
parch notes.md --split               # -> notes.html + style.css
parch docs/ -o dist/                 # whole folder -> static site (shared style.css)
parch docs/ -o dist/ --watch         # rebuild on change
cat a.md | parch - > a.html          # stdin / stdout

parch --list-templates
parch --eject github -o my-theme     # copy a built-in template to edit it
parch a.md -t ./my-theme             # use your template
```

Useful flags: `--toc` / `--no-toc`, `--dir auto|ltr|rtl`, `--lang fa`, `--title`,
`--fa-digits`, `--no-raw-html`, `--no-highlight`, `--smart`, `--inline` / `--split`, `-q`.

Front matter example:

```markdown
---
title: عنوان صفحه
lang: fa
toc: on
---
# سلام
```

### Termux (Android)

```bash
pkg install rust git
git clone <your repo url> parchment && cd parchment
cargo install --path crates/mdr-cli      # first build takes a few minutes
termux-setup-storage                     # once, to read /sdcard
parch ~/storage/shared/notes.md -o ~/storage/shared/notes.html
```

## Desktop & Android app

The app is a thin [Tauri 2](https://tauri.app) shell around the Rust core with a
framework-free UI (`app/ui`): editor and live preview side by side (tabs on
phones), template / contents / direction / digits options, Open and Export.
The preview runs in a sandboxed iframe with scripts disabled.

Prerequisites: Rust, the [Tauri system dependencies](https://tauri.app/start/prerequisites/)
and the CLI: `cargo install tauri-cli --version "^2"`.

```bash
cd app
cargo tauri dev            # run
cargo tauri build          # installers/bundles in app/src-tauri/target/release/bundle
```

Android (needs Android Studio SDK + NDK and `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`):

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
cd app
cargo tauri android init
cargo tauri icon src-tauri/icons/icon.png     # generates the Android launcher icons
cargo tauri android build --apk               # or: cargo tauri android dev
```

On Android, export is always a single self-contained HTML file.

## Web version

```bash
cargo install wasm-pack
scripts/build-web.sh
python3 -m http.server -d dist-web 8080
```

`dist-web/` is plain static files; host it anywhere. Nothing is uploaded: the
conversion happens in the browser.

## Use as a library

```rust
use mdr_core::{convert, CssMode, Options, Template};

let page = convert(
    "# Hello\n\nworld",
    &Template::builtin("magazine").unwrap(),
    &Options::default(),
    &CssMode::Inline,
);
std::fs::write("hello.html", page.html)?;
```

## Project layout

```
crates/mdr-core/   Markdown -> HTML, templates, RTL, highlighting (no I/O)
crates/mdr-cli/    `parch` command
crates/mdr-wasm/   WebAssembly bindings
app/ui/            Shared UI (Tauri + browser), no build step
app/src-tauri/     Tauri 2 shell (desktop + Android)
examples/          demo.md, a small multi-page site, pre-rendered output
docs/TEMPLATES.md  How to write a template
scripts/           build-web.sh
```

## Development

```bash
cargo test --workspace            # core + CLI (the Tauri app is built separately)
cargo run -p mdr-cli -- examples/demo.md -o /tmp/demo.html
```

Status: the core and CLI are covered by unit tests and were exercised end to
end. The Tauri shell and the browser build are small and use only the
documented APIs, but have to be compiled on your machine (they need the
platform toolchains), so expect to fix small things on first build.

## License

MIT
