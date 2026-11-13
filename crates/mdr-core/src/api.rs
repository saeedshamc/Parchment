//! JSON-friendly request/response layer shared by the desktop app and WASM.

use serde::{Deserialize, Serialize};

use crate::render::{DirMode, Options, TocMode};
use crate::template::{convert, CssMode, Template};

fn d_template() -> String {
    "github".into()
}
fn d_auto() -> String {
    "auto".into()
}
fn d_true() -> bool {
    true
}
fn d_inline() -> String {
    "inline".into()
}
fn d_href() -> String {
    "style.css".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RenderRequest {
    pub markdown: String,
    #[serde(default = "d_template")]
    pub template: String,
    /// `auto` | `on` | `off`
    #[serde(default = "d_auto")]
    pub toc: String,
    /// `auto` | `ltr` | `rtl`
    #[serde(default = "d_auto")]
    pub dir: String,
    pub lang: Option<String>,
    pub title: Option<String>,
    pub persian_digits: bool,
    #[serde(default = "d_true")]
    pub raw_html: bool,
    #[serde(default = "d_true")]
    pub highlight: bool,
    pub smart_punctuation: bool,
    /// `inline` | `split`
    #[serde(default = "d_inline")]
    pub css_mode: String,
    /// Stylesheet URL used when `css_mode` is `split`.
    #[serde(default = "d_href")]
    pub css_href: String,
}

impl Default for RenderRequest {
    fn default() -> Self {
        Self {
            markdown: String::new(),
            template: d_template(),
            toc: d_auto(),
            dir: d_auto(),
            lang: None,
            title: None,
            persian_digits: false,
            raw_html: true,
            highlight: true,
            smart_punctuation: false,
            css_mode: d_inline(),
            css_href: d_href(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderResponse {
    pub html: String,
    pub css: String,
    pub title: String,
}

pub fn template_names() -> Vec<String> {
    Template::builtin_names()
        .into_iter()
        .map(String::from)
        .collect()
}

pub fn render_request(req: &RenderRequest) -> Result<RenderResponse, String> {
    let tpl = Template::builtin(&req.template).ok_or_else(|| {
        format!(
            "unknown template '{}' (available: {})",
            req.template,
            Template::builtin_names().join(", ")
        )
    })?;
    let opts = Options {
        title: req.title.clone().filter(|t| !t.is_empty()),
        lang: req.lang.clone().filter(|l| !l.is_empty()),
        dir: DirMode::parse(&req.dir).ok_or_else(|| format!("invalid dir '{}'", req.dir))?,
        toc: TocMode::parse(&req.toc).ok_or_else(|| format!("invalid toc '{}'", req.toc))?,
        persian_digits: req.persian_digits,
        raw_html: req.raw_html,
        highlight: req.highlight,
        smart_punctuation: req.smart_punctuation,
        ..Options::default()
    };
    let mode = if req.css_mode == "split" {
        CssMode::Link(req.css_href.clone())
    } else {
        CssMode::Inline
    };
    let page = convert(&req.markdown, &tpl, &opts, &mode);
    Ok(RenderResponse {
        html: page.html,
        css: page.css,
        title: page.title,
    })
}

/// JSON in, JSON out. Used by the WASM build.
pub fn render_json(request_json: &str) -> Result<String, String> {
    let req: RenderRequest = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let resp = render_request(&req)?;
    serde_json::to_string(&resp).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_json_uses_defaults() {
        let out = render_json(r##"{"markdown":"# Hi"}"##).unwrap();
        assert!(out.contains("\"title\":\"Hi\""));
    }

    #[test]
    fn bad_inputs_report_errors() {
        assert!(render_json(r#"{"template":"zzz"}"#)
            .unwrap_err()
            .contains("unknown template"));
        assert!(render_json(r#"{"dir":"up"}"#)
            .unwrap_err()
            .contains("invalid dir"));
        assert!(render_json("not json").is_err());
    }

    #[test]
    fn split_mode_links_css() {
        let r = render_request(&RenderRequest {
            markdown: "x".into(),
            css_mode: "split".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(r.html.contains("href=\"style.css\""));
    }
}
