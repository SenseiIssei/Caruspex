//! A small PDF 1.4 writer, so the command line tool can write its report
//! without a web view: text in the standard fonts (Helvetica and Courier,
//! regular and bold, WinAnsiEncoding, nothing embedded), lines, filled
//! rectangles, any number of pages, and a flow layout with wrapped text,
//! tables and page numbers.
//!
//!     use caruspex_pdf::{Document, Flow, Margins, Style, Font, Color, A4_WIDTH, A4_HEIGHT};
//!     let margins = Margins { top: 50.0, right: 50.0, bottom: 60.0, left: 50.0 };
//!     let mut flow = Flow::new(Document::new("Report"), A4_WIDTH, A4_HEIGHT, margins);
//!     flow.paragraph("Hello", Style::new(Font::Helvetica, 10.0, Color::BLACK));
//!     let style = Style::new(Font::Helvetica, 8.0, Color::BLACK);
//!     let bytes = flow
//!         .finish(style, Color::BLACK, |p, n| (String::new(), format!("{p}/{n}")))
//!         .to_bytes();
//!     assert!(bytes.starts_with(b"%PDF-1.4"));
#![forbid(unsafe_code)]

pub mod document;
pub mod encoding;
pub mod fonts;
pub mod layout;

pub use document::{A4_HEIGHT, A4_WIDTH, Color, Document, Page};
pub use encoding::{Encoded, covers, encode, fallback, winansi_byte};
pub use fonts::Font;
pub use layout::{Cell, Flow, Margins, Style, Table, wrap};
