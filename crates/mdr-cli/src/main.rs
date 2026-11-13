//! `parch` — render Markdown files or folders to themed HTML/CSS.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use clap::{CommandFactory, Parser, ValueEnum};
use mdr_core::{build_page, render, CssMode, DirMode, Options, Template, TocMode};

type Res<T> = Result<T, String>;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DirArg {
    Auto,
    Ltr,
    Rtl,
}

/// Parchment: turn Markdown into themed, self-contained HTML/CSS.
#[derive(Parser, Debug)]
#[command(name = "parch", version, about, long_about = None)]
struct Cli {
    /// Markdown file, a folder of Markdown files, or `-` for stdin
    input: Option<String>,

    /// Output file or folder (file input: `<name>.html` next to the input; folder input: `./dist`)
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Built-in template name (see --list-templates) or a template folder
    #[arg(short, long, default_value = "github", value_name = "NAME|DIR")]
    template: String,

    /// Print the built-in templates and exit
    #[arg(long)]
    list_templates: bool,

    /// Copy a built-in template into a folder (-o) so you can customise it
    #[arg(long, value_name = "NAME")]
    eject: Option<String>,

    /// Write the CSS to a separate `style.css` instead of embedding it
    #[arg(long, conflicts_with = "inline")]
    split: bool,

    /// Embed the CSS in every page (default for single files)
    #[arg(long)]
    inline: bool,

    /// Always add a table of contents
    #[arg(long, conflicts_with = "no_toc")]
    toc: bool,

    /// Never add a table of contents (default: automatic for longer documents)
    #[arg(long)]
    no_toc: bool,

    /// Text direction
    #[arg(long, value_enum, default_value_t = DirArg::Auto)]
    dir: DirArg,

    /// Language tag for <html lang> (default: guessed from the text)
    #[arg(long)]
    lang: Option<String>,

    /// Page title (default: first H1)
    #[arg(long)]
    title: Option<String>,

    /// Convert 0-9 to Persian digits in prose
    #[arg(long)]
    fa_digits: bool,

    /// Show raw HTML from the Markdown as text instead of passing it through
    #[arg(long)]
    no_raw_html: bool,

    /// Disable syntax highlighting
    #[arg(long)]
    no_highlight: bool,

    /// Smart quotes, dashes and ellipses
    #[arg(long)]
    smart: bool,

    /// Rebuild whenever the input (or a template folder) changes
    #[arg(short, long)]
    watch: bool,

    /// Only print errors
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Res<()> {
    if cli.list_templates {
        for name in Template::builtin_names() {
            println!("{name}");
        }
        return Ok(());
    }
    if let Some(name) = &cli.eject {
        return eject(name, cli.output.clone(), cli.quiet);
    }
    let input = match &cli.input {
        Some(i) => i.clone(),
        None => {
            let _ = Cli::command().print_help();
            return Err("missing INPUT".into());
        }
    };

    if input == "-" {
        if cli.watch {
            return Err("--watch cannot be used with stdin".into());
        }
        return build_stdin(cli);
    }

    let path = PathBuf::from(&input);
    if !path.exists() {
        return Err(format!("{}: no such file or folder", path.display()));
    }

    build(cli, &path)?;
    if !cli.watch {
        return Ok(());
    }

    if !cli.quiet {
        eprintln!("watching {} (Ctrl+C to stop)", path.display());
    }
    let tpl_dir = Path::new(&cli.template)
        .is_dir()
        .then(|| PathBuf::from(&cli.template));
    let out_skip = if path.is_dir() {
        Some(dir_output(cli))
    } else {
        None
    };
    let mut last = stamp(&path, tpl_dir.as_deref(), out_skip.as_deref());
    loop {
        sleep(Duration::from_millis(500));
        let now = stamp(&path, tpl_dir.as_deref(), out_skip.as_deref());
        if now != last {
            last = now;
            if let Err(e) = build(cli, &path) {
                eprintln!("error: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------- options

fn options(cli: &Cli) -> Options {
    Options {
        title: cli.title.clone(),
        lang: cli.lang.clone(),
        dir: match cli.dir {
            DirArg::Auto => DirMode::Auto,
            DirArg::Ltr => DirMode::Ltr,
            DirArg::Rtl => DirMode::Rtl,
        },
        toc: if cli.toc {
            TocMode::On
        } else if cli.no_toc {
            TocMode::Off
        } else {
            TocMode::Auto
        },
        persian_digits: cli.fa_digits,
        raw_html: !cli.no_raw_html,
        highlight: !cli.no_highlight,
        smart_punctuation: cli.smart,
        ..Options::default()
    }
}

fn load_template(spec: &str) -> Res<Template> {
    let p = Path::new(spec);
    if p.is_dir() {
        let html = fs::read_to_string(p.join("template.html"))
            .map_err(|e| format!("{}: cannot read template.html: {e}", p.display()))?;
        let css = fs::read_to_string(p.join("style.css")).unwrap_or_default();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "custom".into());
        return Ok(Template::new(name, html, css));
    }
    Template::builtin(spec).ok_or_else(|| {
        format!(
            "unknown template '{spec}'. Built-in: {}. You can also pass a folder containing template.html (+ style.css).",
            Template::builtin_names().join(", ")
        )
    })
}

fn eject(name: &str, out: Option<PathBuf>, quiet: bool) -> Res<()> {
    let t = Template::builtin(name).ok_or_else(|| {
        format!(
            "unknown template '{name}'. Built-in: {}",
            Template::builtin_names().join(", ")
        )
    })?;
    let dir = out.unwrap_or_else(|| PathBuf::from(format!("{}-template", t.name)));
    if dir.join("template.html").exists() {
        return Err(format!("{} already contains a template", dir.display()));
    }
    write_file(&dir.join("template.html"), &t.html)?;
    write_file(&dir.join("style.css"), &t.css)?;
    if !quiet {
        eprintln!("template '{}' copied to {}", t.name, dir.display());
        eprintln!("use it with: parch FILE.md -t {}", dir.display());
    }
    Ok(())
}

// ---------------------------------------------------------------- building

fn build(cli: &Cli, path: &Path) -> Res<()> {
    let tpl = load_template(&cli.template)?;
    let opts = options(cli);
    if path.is_dir() {
        build_dir(cli, path, &tpl, &opts)
    } else {
        build_file(cli, path, &tpl, &opts)
    }
}

fn build_stdin(cli: &Cli) -> Res<()> {
    let tpl = load_template(&cli.template)?;
    let mut src = String::new();
    io::stdin()
        .read_to_string(&mut src)
        .map_err(|e| format!("stdin: {e}"))?;
    let doc = render(&src, &options(cli));
    match &cli.output {
        None => {
            let page = build_page(&doc, &tpl, &CssMode::Inline);
            io::stdout()
                .write_all(page.html.as_bytes())
                .map_err(|e| e.to_string())
        }
        Some(out) => {
            let out = if out.is_dir() {
                out.join("index.html")
            } else {
                out.clone()
            };
            write_page(cli, &doc, &tpl, &out)
        }
    }
}

fn build_file(cli: &Cli, input: &Path, tpl: &Template, opts: &Options) -> Res<()> {
    let src = read_text(input)?;
    let doc = render(&src, opts);
    let out = resolve_out_file(input, cli.output.as_deref());
    write_page(cli, &doc, tpl, &out)
}

/// Writes one page (and `style.css` beside it in split mode).
fn write_page(cli: &Cli, doc: &mdr_core::Document, tpl: &Template, out: &Path) -> Res<()> {
    let mode = if cli.split {
        CssMode::Link("style.css".into())
    } else {
        CssMode::Inline
    };
    let page = build_page(doc, tpl, &mode);
    write_file(out, &page.html)?;
    if cli.split {
        let css_path = out
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("style.css");
        write_file(&css_path, &page.css)?;
    }
    if !cli.quiet {
        eprintln!("wrote {}", out.display());
    }
    Ok(())
}

fn resolve_out_file(input: &Path, out: Option<&Path>) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".into());
    match out {
        None => input.with_file_name(format!("{stem}.html")),
        Some(o) => {
            let s = o.to_string_lossy();
            if o.is_dir() || s.ends_with('/') || s.ends_with('\\') {
                o.join(format!("{stem}.html"))
            } else {
                o.to_path_buf()
            }
        }
    }
}

fn dir_output(cli: &Cli) -> PathBuf {
    cli.output.clone().unwrap_or_else(|| PathBuf::from("dist"))
}

fn is_markdown(p: &Path) -> bool {
    matches!(
        p.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .as_deref(),
        Some("md") | Some("markdown")
    )
}

fn skip_name(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules" || name == "target"
}

fn collect_files(dir: &Path, out_canon: &Path, acc: &mut Vec<PathBuf>) -> Res<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if skip_name(&name) {
            continue;
        }
        if p.is_dir() {
            if p.canonicalize().map(|c| c == out_canon).unwrap_or(false) {
                continue;
            }
            collect_files(&p, out_canon, acc)?;
        } else if p.is_file() {
            acc.push(p);
        }
    }
    Ok(())
}

fn rel_url(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn build_dir(cli: &Cli, root: &Path, tpl: &Template, base_opts: &Options) -> Res<()> {
    let out_root = dir_output(cli);
    fs::create_dir_all(&out_root).map_err(|e| format!("{}: {e}", out_root.display()))?;
    let out_canon = out_root.canonicalize().map_err(|e| e.to_string())?;
    if root.canonicalize().map(|r| r == out_canon).unwrap_or(false) {
        return Err("the output folder must differ from the input folder".into());
    }

    let mut files = Vec::new();
    collect_files(root, &out_canon, &mut files)?;

    let mut opts = base_opts.clone();
    opts.rewrite_md_links = true;
    let inline = cli.inline;

    let mut pages: Vec<(String, String)> = Vec::new(); // (relative html url, title)
    let mut copied = 0usize;

    for file in &files {
        let rel = file.strip_prefix(root).map_err(|e| e.to_string())?;
        if is_markdown(file) {
            let src = read_text(file)?;
            let doc = render(&src, &opts);
            let html_rel = rel.with_extension("html");
            let depth = html_rel.components().count().saturating_sub(1);
            let mode = if inline {
                CssMode::Inline
            } else {
                CssMode::Link(format!("{}style.css", "../".repeat(depth)))
            };
            let page = build_page(&doc, tpl, &mode);
            write_file(&out_root.join(&html_rel), &page.html)?;
            let title = if doc.title == "Untitled" {
                file.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or(doc.title.clone())
            } else {
                doc.title.clone()
            };
            pages.push((rel_url(&html_rel), title));
        } else {
            let dest = out_root.join(rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::copy(file, &dest).map_err(|e| format!("{}: {e}", file.display()))?;
            copied += 1;
        }
    }

    // Root index page when the folder has none.
    if !pages.is_empty() && !pages.iter().any(|(u, _)| u == "index.html") {
        let dir_name = root
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Index".into());
        let mut sorted = pages.clone();
        sorted.sort_by(|a, b| {
            let ra = a.0.eq_ignore_ascii_case("readme.html");
            let rb = b.0.eq_ignore_ascii_case("readme.html");
            rb.cmp(&ra).then_with(|| a.0.cmp(&b.0))
        });
        let mut md = format!("# {dir_name}\n\n");
        for (url, title) in &sorted {
            let t = title.replace('[', "\\[").replace(']', "\\]");
            md.push_str(&format!("- [{t}](<{url}>)\n"));
        }
        let doc = render(
            &md,
            &Options {
                rewrite_md_links: false,
                toc: TocMode::Off,
                ..opts.clone()
            },
        );
        let mode = if inline {
            CssMode::Inline
        } else {
            CssMode::Link("style.css".into())
        };
        let page = build_page(&doc, tpl, &mode);
        write_file(&out_root.join("index.html"), &page.html)?;
    }

    if !inline {
        write_file(&out_root.join("style.css"), &tpl.full_css())?;
    }

    if !cli.quiet {
        eprintln!(
            "wrote {} page(s){} to {}",
            pages.len(),
            if copied > 0 {
                format!(" + {copied} asset(s)")
            } else {
                String::new()
            },
            out_root.display()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------- helpers

fn read_text(p: &Path) -> Res<String> {
    fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn write_file(p: &Path, content: &str) -> Res<()> {
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    fs::write(p, content).map_err(|e| format!("{}: {e}", p.display()))
}

/// Cheap change detector for --watch: newest mtime + number of files.
fn stamp(input: &Path, tpl_dir: Option<&Path>, skip: Option<&Path>) -> (SystemTime, usize) {
    let skip_canon = skip.and_then(|s| s.canonicalize().ok());
    let mut newest = SystemTime::UNIX_EPOCH;
    let mut count = 0usize;
    let mut visit = |p: &Path| {
        if let Ok(m) = fs::metadata(p).and_then(|m| m.modified()) {
            if m > newest {
                newest = m;
            }
        }
        count += 1;
    };
    fn walk(dir: &Path, skip: Option<&Path>, visit: &mut dyn FnMut(&Path)) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if skip_name(&e.file_name().to_string_lossy()) {
                    continue;
                }
                if p.is_dir() {
                    if skip
                        .map(|s| p.canonicalize().map(|c| c == s).unwrap_or(false))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    walk(&p, skip, visit);
                } else {
                    visit(&p);
                }
            }
        }
    }
    if input.is_dir() {
        walk(input, skip_canon.as_deref(), &mut visit);
    } else {
        visit(input);
    }
    if let Some(t) = tpl_dir {
        walk(t, None, &mut visit);
    }
    (newest, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_file_resolution() {
        assert_eq!(
            resolve_out_file(Path::new("a/b/readme.md"), None),
            PathBuf::from("a/b/readme.html")
        );
        assert_eq!(
            resolve_out_file(Path::new("x.md"), Some(Path::new("out/"))),
            PathBuf::from("out/x.html")
        );
        assert_eq!(
            resolve_out_file(Path::new("x.md"), Some(Path::new("y.htm"))),
            PathBuf::from("y.htm")
        );
    }

    #[test]
    fn markdown_detection() {
        assert!(is_markdown(Path::new("A.MD")));
        assert!(is_markdown(Path::new("a.markdown")));
        assert!(!is_markdown(Path::new("a.txt")));
    }

    #[test]
    fn url_building() {
        assert_eq!(rel_url(Path::new("a/b/c.html")), "a/b/c.html");
    }
}
