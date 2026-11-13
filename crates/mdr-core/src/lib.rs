//! # mdr-core
//!
//! Markdown -> themed HTML/CSS with first-class RTL (Persian/Arabic/Hebrew)
//! support. No I/O: the same code runs in the CLI, the desktop/Android app
//! and in the browser via WASM.
//!
//! ```
//! use mdr_core::{convert, CssMode, Options, Template};
//!
//! let page = convert("# Hello\n\nworld", &Template::default_template(), &Options::default(), &CssMode::Inline);
//! assert!(page.html.contains("<h1"));
//! ```

#[cfg(feature = "api")]
pub mod api;
pub mod digits;
pub mod frontmatter;
pub mod highlight;
pub mod render;
pub mod rtl;
pub mod slug;
pub mod template;
pub mod toc;

pub use render::{render, rewrite_md_link, DirMode, Document, Options, TocMode};
pub use template::{build_page, convert, CssMode, Page, Template};
pub use toc::Heading;
