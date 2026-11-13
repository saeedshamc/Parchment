// Parchment UI. Runs unchanged inside Tauri (desktop/Android) and in a plain
// browser (WebAssembly build). No bundler, no framework.

const $ = (id) => document.getElementById(id);
const tauri = window.__TAURI__;
const isTauri = Boolean(tauri && tauri.core && tauri.core.invoke);
const hasFs = Boolean(isTauri && tauri.dialog && tauri.fs);
const isAndroid = /android/i.test(navigator.userAgent);

const editor = $("editor");
const preview = $("preview");
const panes = document.querySelector(".panes");
const statusEl = $("status");
const optTemplate = $("opt-template");
const optToc = $("opt-toc");
const optDir = $("opt-dir");
const optDigits = $("opt-digits");
const optMode = $("opt-mode");
const fileInput = $("file-input");

const I18N = {
  en: {
    open: "Open", export: "Export", template: "Template", toc: "Contents", dir: "Direction",
    auto: "Auto", on: "On", off: "Off", digits: "Persian digits", output: "Output",
    single: "Single file", split: "HTML + CSS", edit: "Edit", preview: "Preview",
    saved: "Saved.", wasmMissing: "WebAssembly module not found. Run scripts/build-web.sh first.",
  },
  fa: {
    open: "باز کردن", export: "خروجی", template: "قالب", toc: "فهرست مطالب", dir: "جهت",
    auto: "خودکار", on: "روشن", off: "خاموش", digits: "اعداد فارسی", output: "نوع خروجی",
    single: "یک فایل", split: "HTML + CSS", edit: "ویرایش", preview: "پیش‌نمایش",
    saved: "ذخیره شد.", wasmMissing: "ماژول WebAssembly پیدا نشد. اول scripts/build-web.sh را اجرا کنید.",
  },
};

const SAMPLE = `# سلام Parchment

این یک **پیش‌نمایش زنده** است. متن را ویرایش کنید و خروجی همین‌جا دیده می‌شود.

## Features

- RTL + LTR in the same document
- Tables, task lists, footnotes
- Themes: github, minimal, dark, magazine

| Name | Value |
|------|-------|
| Core | Rust |

\`\`\`js
console.log("hello");
\`\`\`
`;

function load(key) {
  try { return localStorage.getItem(key); } catch { return null; }
}
function save(key, value) {
  try { localStorage.setItem(key, value); } catch { /* storage may be unavailable */ }
}

let lang = load("lang") || ((navigator.language || "en").toLowerCase().startsWith("fa") ? "fa" : "en");
const t = (key) => I18N[lang][key] ?? key;

function applyLang() {
  document.documentElement.lang = lang;
  document.documentElement.dir = lang === "fa" ? "rtl" : "ltr";
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
}

function setStatus(message, isError = false) {
  statusEl.textContent = message;
  statusEl.className = isError ? "error" : "";
}

// ---------------------------------------------------------------- backend

async function initBackend() {
  if (isTauri) {
    const { invoke } = tauri.core;
    return {
      templates: () => invoke("templates"),
      render: (req) => invoke("render", { req }),
    };
  }
  const mod = await import("./pkg/mdr_wasm.js");
  await mod.default();
  return {
    templates: async () => Array.from(mod.templates()),
    render: async (req) => JSON.parse(mod.render(JSON.stringify(req))),
  };
}

let backend = null;
let baseName = "document";
let timer = 0;
let seq = 0;
let pendingScroll = 0;

function request(extra = {}) {
  return {
    markdown: editor.value,
    template: optTemplate.value || "github",
    toc: optToc.value,
    dir: optDir.value,
    persian_digits: optDigits.checked,
    css_mode: "inline",
    css_href: "style.css",
    ...extra,
  };
}

function readScroll() {
  try { return preview.contentWindow.scrollY || 0; } catch { return 0; }
}

preview.addEventListener("load", () => {
  try { preview.contentWindow.scrollTo(0, pendingScroll); } catch { /* cross-origin */ }
});

async function renderPreview() {
  if (!backend) return;
  const mine = ++seq;
  try {
    const res = await backend.render(request());
    if (mine !== seq) return;
    pendingScroll = readScroll();
    preview.srcdoc = res.html;
    setStatus(res.title || "");
  } catch (e) {
    setStatus(String(e), true);
  }
}

function schedule() {
  clearTimeout(timer);
  timer = setTimeout(renderPreview, 150);
  save("draft", editor.value);
}

// ---------------------------------------------------------------- files

const stem = (name) => (name.split(/[\\/]/).pop() || "").replace(/\.[^.]*$/, "") || "document";

async function openFile() {
  try {
    if (hasFs) {
      const picked = await tauri.dialog.open({
        multiple: false,
        filters: [{ name: "Markdown", extensions: ["md", "markdown", "txt"] }],
      });
      if (!picked) return;
      const path = typeof picked === "string" ? picked : picked.path;
      editor.value = await tauri.fs.readTextFile(path);
      baseName = stem(path);
      schedule();
    } else {
      fileInput.click();
    }
  } catch (e) {
    setStatus(String(e), true);
  }
}

async function loadBrowserFile(file) {
  if (!file) return;
  editor.value = await file.text();
  baseName = stem(file.name);
  schedule();
}

fileInput.addEventListener("change", async () => {
  await loadBrowserFile(fileInput.files[0]);
  fileInput.value = "";
});
editor.addEventListener("dragover", (e) => e.preventDefault());
editor.addEventListener("drop", async (e) => {
  e.preventDefault();
  await loadBrowserFile(e.dataTransfer && e.dataTransfer.files[0]);
});

function download(name, text, type) {
  const url = URL.createObjectURL(new Blob([text], { type: `${type};charset=utf-8` }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
}

async function exportHtml() {
  const mode = isAndroid ? "inline" : optMode.value;
  try {
    const res = await backend.render(request({ css_mode: mode }));
    const htmlName = `${baseName}.html`;
    if (hasFs) {
      if (mode === "split") {
        const picked = await tauri.dialog.open({ directory: true });
        if (!picked) return;
        const dir = typeof picked === "string" ? picked : picked.path;
        const sep = dir.includes("\\") ? "\\" : "/";
        await tauri.fs.writeTextFile(dir + sep + htmlName, res.html);
        await tauri.fs.writeTextFile(dir + sep + "style.css", res.css);
      } else {
        const path = await tauri.dialog.save({
          defaultPath: htmlName,
          filters: [{ name: "HTML", extensions: ["html"] }],
        });
        if (!path) return;
        await tauri.fs.writeTextFile(path, res.html);
      }
    } else {
      download(htmlName, res.html, "text/html");
      if (mode === "split") setTimeout(() => download("style.css", res.css, "text/css"), 400);
    }
    setStatus(t("saved"));
  } catch (e) {
    setStatus(String(e), true);
  }
}

// ---------------------------------------------------------------- settings

function saveOptions() {
  save("opts", JSON.stringify({
    template: optTemplate.value, toc: optToc.value, dir: optDir.value,
    digits: optDigits.checked, mode: optMode.value,
  }));
}

function restoreOptions() {
  try {
    const o = JSON.parse(load("opts") || "{}");
    if (o.template && [...optTemplate.options].some((x) => x.value === o.template)) optTemplate.value = o.template;
    if (o.toc) optToc.value = o.toc;
    if (o.dir) optDir.value = o.dir;
    if (typeof o.digits === "boolean") optDigits.checked = o.digits;
    if (o.mode) optMode.value = o.mode;
  } catch { /* ignore corrupt settings */ }
}

function setView(view) {
  panes.dataset.view = view;
  $("tab-edit").classList.toggle("active", view === "edit");
  $("tab-view").classList.toggle("active", view === "view");
  if (view === "view") renderPreview();
}

// ---------------------------------------------------------------- start

async function main() {
  applyLang();
  if (isAndroid) $("mode-wrap").hidden = true;
  editor.value = load("draft") || SAMPLE;

  try {
    backend = await initBackend();
    const names = await backend.templates();
    optTemplate.replaceChildren(...names.map((n) => new Option(n, n)));
  } catch (e) {
    setStatus(isTauri ? String(e) : t("wasmMissing"), true);
    return;
  }
  restoreOptions();

  editor.addEventListener("input", schedule);
  [optTemplate, optToc, optDir, optDigits, optMode].forEach((el) =>
    el.addEventListener("change", () => { saveOptions(); schedule(); }));
  $("btn-open").addEventListener("click", openFile);
  $("btn-export").addEventListener("click", exportHtml);
  $("btn-lang").addEventListener("click", () => {
    lang = lang === "fa" ? "en" : "fa";
    save("lang", lang);
    applyLang();
  });
  $("tab-edit").addEventListener("click", () => setView("edit"));
  $("tab-view").addEventListener("click", () => setView("view"));
  document.addEventListener("keydown", (e) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === "o") { e.preventDefault(); openFile(); }
    if (k === "s") { e.preventDefault(); exportHtml(); }
  });

  setView("edit");
  renderPreview();
}

main();
