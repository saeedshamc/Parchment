# Writing a template

A template is a folder with two files:

```
my-theme/
├── template.html   # page skeleton with {{placeholders}}
└── style.css       # your stylesheet (optional)
```

Start from a built-in one: `parch --eject github -o my-theme`, then use it with
`parch file.md -t my-theme` (also works with folder mode and `--watch`; the
template folder is reloaded on every rebuild).

## Placeholders

| Placeholder | Replaced with |
|-------------|---------------|
| `{{title}}` | Page title (HTML-escaped): front matter `title`, else the first `# H1` |
| `{{lang}}` | Language tag (`fa`, `ar`, `he`, `en`, or the `lang` you passed) |
| `{{dir}}` | `rtl` or `ltr` |
| `{{styles}}` | A `<style>` block, or a `<link rel="stylesheet">` in `--split` / folder mode |
| `{{toc}}` | The `<nav class="toc">` block, or nothing when there is no table of contents |
| `{{content}}` | The rendered Markdown |

Rules: placeholders may contain spaces (`{{ title }}`); unknown placeholders
become empty; substitution is single-pass, so Markdown that contains the text
`{{title}}` is never expanded. Always keep `{{styles}}` inside `<head>` and use
`lang="{{lang}}" dir="{{dir}}"` on `<html>`.

## What the HTML looks like

- Headings: `<h2 id="slug" dir="auto">`. Slugs keep Persian/Arabic letters.
- Paragraphs, list items and block quotes carry `dir="auto"` (mixed-language
  documents align per block). With `--dir ltr|rtl` they are left plain.
- Code: `<pre class="code" data-lang="rust"><code class="language-rust">…</code></pre>`.
  Highlighted tokens use classes prefixed with `hl-`.
- Tables, task lists (`<input type="checkbox" disabled>`), footnotes
  (`.footnote-definition`, `sup.footnote-reference`), `<del>` for strikethrough.

## Styling tips

- Use logical properties (`margin-inline-start`, `padding-inline-end`,
  `border-inline-start`, `text-align: start`) so one stylesheet serves LTR and RTL.
- Keep `pre { direction: ltr; text-align: left; }`, code is always left-to-right.
- Persian fonts: list `"Vazirmatn"` first, then a system fallback. Fonts are not
  downloaded; to embed one, add an `@font-face` with a `data:` URL or ship the
  file next to the page.

## Syntax colours

The shared highlight stylesheet is prepended to your CSS and uses these
variables, so you can recolour code by overriding them in `:root`:

```css
:root {
  --hl-comment: #6a737d;
  --hl-string:  #0a7f5a;
  --hl-keyword: #d73a49;
  --hl-number:  #005cc5;
  --hl-func:    #6f42c1;
  --hl-var:     #e36209;
}
```

## Adding a built-in template

Create `crates/mdr-core/templates/<name>/{template.html,style.css}` and add it to
the `BUILTIN` array in `crates/mdr-core/src/template.rs` (the existing test checks
that every built-in has all placeholders).
