//! A simple flow layout on top of [`Document`]: text from top to bottom
//! between margins, wrapped paragraphs, headings that stay with what
//! follows, tables whose rows never break across pages (unless one row is
//! taller than a page), and page numbers once the page count is known.
//!
//! The flow measures from the top of the page (y grows downwards), which is
//! how a report is read; it converts to PDF coordinates when drawing.

use crate::document::{Color, Document, Page};
use crate::fonts::Font;

/// Font, size and colour of a piece of text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub font: Font,
    pub size: f32,
    pub color: Color,
}

impl Style {
    pub const fn new(font: Font, size: f32, color: Color) -> Self {
        Self { font, size, color }
    }

    /// Height of one line.
    pub fn line_height(&self) -> f32 {
        self.size * 1.3
    }

    /// From the top of a line to its baseline: half the extra leading plus
    /// the ascent of Helvetica and Courier (about 0.8 em).
    fn baseline(&self) -> f32 {
        self.size * 0.95
    }

    pub fn width(&self, text: &str) -> f32 {
        self.font.text_width(text, self.size)
    }
}

/// Breaks `text` into lines no wider than `max_width`: at spaces, at "\n"
/// always, and inside a word only when the word alone is too wide (a long
/// part number, a CVN list). Never returns an empty list.
pub fn wrap(text: &str, font: Font, size: f32, max_width: f32) -> Vec<String> {
    let fits = |s: &str| font.text_width(s, size) <= max_width;
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|w| !w.is_empty()) {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if fits(&candidate) {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            if fits(word) {
                line = word.to_string();
                continue;
            }
            for c in word.chars() {
                let mut longer = line.clone();
                longer.push(c);
                if !line.is_empty() && !fits(&longer) {
                    lines.push(std::mem::take(&mut line));
                    line.push(c);
                } else {
                    line = longer;
                }
            }
        }
        lines.push(line);
    }
    lines
}

/// One table cell: one or more pieces of text, each starting on its own
/// line and wrapped to the column (a bold title with a grey explanation
/// below it, for example).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub parts: Vec<(Style, String)>,
}

impl Cell {
    pub fn new(style: Style, text: &str) -> Self {
        Self::default().then(style, text)
    }

    /// Adds a piece below; empty text adds nothing.
    pub fn then(mut self, style: Style, text: &str) -> Self {
        if !text.trim().is_empty() {
            self.parts.push((style, text.to_string()));
        }
        self
    }
}

/// A table across the content width.
#[derive(Clone, Debug)]
pub struct Table {
    /// Share of the width per column; they should add up to 1.
    pub widths: Vec<f32>,
    /// Column titles, repeated at the top of every page the table runs
    /// onto; empty for none.
    pub header: Vec<String>,
    pub header_style: Style,
    pub header_fill: Color,
    /// The thin line under every row.
    pub rule: Color,
    pub rows: Vec<Vec<Cell>>,
}

/// Room around the text of a cell.
const PAD_X: f32 = 6.0;
const PAD_Y: f32 = 4.0;

/// Page margins in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// The lines of one cell, wrapped, with their styles.
type Lines = Vec<(Style, String)>;

pub struct Flow {
    doc: Document,
    width: f32,
    height: f32,
    margins: Margins,
    /// Distance of the cursor from the top edge of the current page.
    y: f32,
}

impl Flow {
    /// Starts the first page.
    pub fn new(mut doc: Document, width: f32, height: f32, margins: Margins) -> Self {
        doc.add_page(width, height);
        Self {
            doc,
            width,
            height,
            margins,
            y: margins.top,
        }
    }

    pub fn left(&self) -> f32 {
        self.margins.left
    }

    pub fn right(&self) -> f32 {
        self.width - self.margins.right
    }

    pub fn content_width(&self) -> f32 {
        self.right() - self.left()
    }

    /// Where the next thing goes, from the top of the page.
    pub fn cursor(&self) -> f32 {
        self.y
    }

    pub fn set_cursor(&mut self, y: f32) {
        self.y = y;
    }

    /// Room left on the current page.
    pub fn remaining(&self) -> f32 {
        self.height - self.margins.bottom - self.y
    }

    /// Room on a fresh page.
    pub fn page_room(&self) -> f32 {
        self.height - self.margins.bottom - self.margins.top
    }

    pub fn page_count(&self) -> usize {
        self.doc.pages.len()
    }

    fn at_page_top(&self) -> bool {
        self.y <= self.margins.top + 0.01
    }

    pub fn new_page(&mut self) {
        self.doc.add_page(self.width, self.height);
        self.y = self.margins.top;
    }

    /// Starts a new page unless `height` still fits; true if it did.
    pub fn ensure(&mut self, height: f32) -> bool {
        if height > self.remaining() && !self.at_page_top() {
            self.new_page();
            return true;
        }
        false
    }

    /// Vertical space; at the bottom of a page it is simply used up.
    pub fn space(&mut self, height: f32) {
        self.y = (self.y + height).min(self.height - self.margins.bottom);
    }

    fn page(&mut self) -> &mut Page {
        self.doc
            .pages
            .last_mut()
            .expect("the flow always has a page")
    }

    /// One line of text whose line box starts `top` below the page top.
    pub fn text_at(&mut self, x: f32, top: f32, style: Style, text: &str) {
        let y = self.height - top - style.baseline();
        self.page()
            .text(x, y, style.font, style.size, style.color, text);
    }

    /// The same, ending at `right`.
    pub fn text_right(&mut self, right: f32, top: f32, style: Style, text: &str) {
        let x = right - style.width(text);
        self.text_at(x, top, style, text);
    }

    /// Spaced-out text (Tc), for the brand line.
    pub fn text_spaced(&mut self, x: f32, top: f32, style: Style, spacing: f32, text: &str) {
        let y = self.height - top - style.baseline();
        self.page()
            .text_spaced(x, y, style.font, style.size, style.color, spacing, text);
    }

    /// A horizontal line across the content width at the cursor.
    pub fn rule(&mut self, width: f32, color: Color) {
        let y = self.height - self.y;
        let (left, right) = (self.left(), self.right());
        self.page().line(left, y, right, y, width, color);
    }

    /// Wrapped text across the content width; may continue on the next page
    /// between lines.
    pub fn paragraph(&mut self, text: &str, style: Style) {
        let left = self.left();
        for line in wrap(text, style.font, style.size, self.content_width()) {
            self.ensure(style.line_height());
            let top = self.y;
            self.text_at(left, top, style, &line);
            self.y += style.line_height();
        }
    }

    /// A heading, moved to the next page together with `keep` points of
    /// what follows when they do not fit together.
    pub fn heading(&mut self, text: &str, style: Style, keep: f32) {
        let lines = wrap(text, style.font, style.size, self.content_width());
        let height = lines.len() as f32 * style.line_height();
        self.ensure(height + keep.min(self.page_room() / 2.0));
        let left = self.left();
        for line in lines {
            let top = self.y;
            self.text_at(left, top, style, &line);
            self.y += style.line_height();
        }
    }

    fn column_x(&self, table: &Table) -> Vec<(f32, f32)> {
        let mut x = self.left();
        table
            .widths
            .iter()
            .map(|share| {
                let w = share * self.content_width();
                let column = (x, w);
                x += w;
                column
            })
            .collect()
    }

    fn cell_lines(cell: &Cell, width: f32) -> Lines {
        let mut lines = Lines::new();
        for (style, text) in &cell.parts {
            for line in wrap(text, style.font, style.size, (width - 2.0 * PAD_X).max(1.0)) {
                lines.push((*style, line));
            }
        }
        lines
    }

    fn header_height(table: &Table) -> f32 {
        if table.header.is_empty() {
            0.0
        } else {
            table.header_style.line_height() + 2.0 * PAD_Y
        }
    }

    fn draw_header(&mut self, table: &Table, columns: &[(f32, f32)]) {
        if table.header.is_empty() {
            return;
        }
        let h = Self::header_height(table);
        let y = self.height - self.y - h;
        let (left, width) = (self.left(), self.content_width());
        self.page().fill_rect(left, y, width, h, table.header_fill);
        let top = self.y + PAD_Y;
        for ((x, _), title) in columns.iter().zip(&table.header) {
            self.text_at(x + PAD_X, top, table.header_style, title);
        }
        self.y += h;
    }

    /// Height of the header and the first row, for [`Flow::heading`].
    pub fn table_start_height(&self, table: &Table) -> f32 {
        let columns = self.column_x(table);
        let first = table.rows.first().map_or(0.0, |row| {
            row.iter()
                .zip(&columns)
                .map(|(cell, (_, w))| lines_height(&Self::cell_lines(cell, *w)))
                .fold(0.0, f32::max)
                + 2.0 * PAD_Y
        });
        Self::header_height(table) + first
    }

    /// Draws the table at the cursor.
    pub fn table(&mut self, table: &Table) {
        let columns = self.column_x(table);
        let header_h = Self::header_height(table);
        self.ensure(self.table_start_height(table));
        self.draw_header(table, &columns);
        for row in &table.rows {
            let cells: Vec<Lines> = row
                .iter()
                .zip(&columns)
                .map(|(cell, (_, w))| Self::cell_lines(cell, *w))
                .collect();
            self.draw_row(table, &columns, &cells, header_h);
        }
    }

    /// One row: on this page if it fits, else on the next one; split by
    /// lines only when it is taller than a whole page.
    fn draw_row(&mut self, table: &Table, columns: &[(f32, f32)], cells: &[Lines], header_h: f32) {
        let mut next = vec![0usize; cells.len()];
        loop {
            let rest = cells
                .iter()
                .zip(&next)
                .map(|(lines, &from)| lines_height(&lines[from..]))
                .fold(0.0, f32::max)
                + 2.0 * PAD_Y;
            let fresh_room = self.page_room() - header_h;
            let top_of_table = self.y <= self.margins.top + header_h + 0.01;
            if rest > self.remaining() && (rest <= fresh_room || !top_of_table) {
                self.new_page();
                self.draw_header(table, columns);
                continue;
            }
            // Everything that fits below the cursor.
            let room = self.remaining() - 2.0 * PAD_Y;
            let mut taken = vec![0usize; cells.len()];
            let mut chunk = 0.0f32;
            for (i, lines) in cells.iter().enumerate() {
                let mut h = 0.0;
                for (style, _) in &lines[next[i]..] {
                    if h + style.line_height() > room + 0.01 {
                        break;
                    }
                    h += style.line_height();
                    taken[i] += 1;
                }
                chunk = chunk.max(h);
            }
            let top = self.y + PAD_Y;
            for (i, lines) in cells.iter().enumerate() {
                let x = columns[i].0 + PAD_X;
                let mut line_top = top;
                for (style, text) in &lines[next[i]..next[i] + taken[i]] {
                    self.text_at(x, line_top, *style, text);
                    line_top += style.line_height();
                }
                next[i] += taken[i];
            }
            self.y += chunk + 2.0 * PAD_Y;
            self.rule(0.6, table.rule);
            if cells.iter().zip(&next).all(|(lines, &n)| n >= lines.len()) {
                return;
            }
            self.new_page();
            self.draw_header(table, columns);
        }
    }

    /// Ends the flow: `footer(page, pages)` gives the left and the right
    /// footer text of every page (numbered from 1), drawn with `style`
    /// under a thin `rule` line.
    pub fn finish(
        mut self,
        style: Style,
        rule: Color,
        footer: impl Fn(usize, usize) -> (String, String),
    ) -> Document {
        let pages = self.doc.pages.len();
        let (left, right) = (self.left(), self.right());
        let baseline = self.margins.bottom * 0.5;
        for (i, page) in self.doc.pages.iter_mut().enumerate() {
            let (l, r) = footer(i + 1, pages);
            page.line(
                left,
                baseline + style.size + 4.0,
                right,
                baseline + style.size + 4.0,
                0.5,
                rule,
            );
            page.text(left, baseline, style.font, style.size, style.color, &l);
            let x = right - style.width(&r);
            page.text(x, baseline, style.font, style.size, style.color, &r);
        }
        self.doc
    }
}

fn lines_height(lines: &[(Style, String)]) -> f32 {
    lines.iter().map(|(s, _)| s.line_height()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{A4_HEIGHT, A4_WIDTH};

    const BODY: Style = Style::new(Font::Helvetica, 10.0, Color::BLACK);
    const MARGINS: Margins = Margins {
        top: 50.0,
        right: 50.0,
        bottom: 60.0,
        left: 50.0,
    };

    #[test]
    fn wrap_breaks_at_spaces_and_keeps_every_word() {
        let text = "The control unit adds on average more fuel than it calculated.";
        let lines = wrap(text, Font::Helvetica, 10.0, 120.0);
        assert!(lines.len() > 1);
        for l in &lines {
            assert!(Font::Helvetica.text_width(l, 10.0) <= 120.0, "{l}");
        }
        assert_eq!(lines.join(" "), text);
    }

    #[test]
    fn wrap_cuts_a_word_that_is_too_long_and_honours_line_breaks() {
        let lines = wrap("AAAAAAAAAAAAAAAAAAAA", Font::Courier, 10.0, 60.0);
        assert_eq!(lines, ["AAAAAAAAAA", "AAAAAAAAAA"]);
        assert_eq!(wrap("a\nb", Font::Helvetica, 10.0, 100.0), ["a", "b"]);
        assert_eq!(wrap("", Font::Helvetica, 10.0, 100.0), [""]);
    }

    fn table(rows: usize, lines_per_row: usize) -> Table {
        let text = vec!["word"; lines_per_row].join("\n");
        Table {
            widths: vec![0.3, 0.7],
            header: vec!["A".into(), "B".into()],
            header_style: BODY,
            header_fill: Color::hex(0xEEEEEE),
            rule: Color::hex(0xDDDDDD),
            rows: (0..rows)
                .map(|i| {
                    vec![
                        Cell::new(BODY, &format!("row{i:03}")),
                        Cell::new(BODY, &text),
                    ]
                })
                .collect(),
        }
    }

    /// The top-down positions of every "rowNNN" text, per page.
    fn row_tops(doc: &Document) -> Vec<Vec<(usize, f32)>> {
        doc.pages
            .iter()
            .map(|p| {
                let mut out = Vec::new();
                let mut last_y = 0.0;
                for line in p.content().lines() {
                    if let Some(td) = line.strip_suffix(" Td") {
                        last_y = td.split(' ').nth(1).unwrap().parse::<f32>().unwrap();
                    }
                    if let Some(rest) = line.strip_prefix("(row") {
                        let n: usize = rest[..3].parse().unwrap();
                        out.push((n, A4_HEIGHT - last_y));
                    }
                }
                out
            })
            .collect()
    }

    #[test]
    fn rows_are_never_split_and_the_header_repeats() {
        let mut flow = Flow::new(Document::new("t"), A4_WIDTH, A4_HEIGHT, MARGINS);
        flow.table(&table(40, 3));
        let doc = flow.finish(BODY, Color::BLACK, |p, n| {
            (String::new(), format!("{p}/{n}"))
        });
        // 40 rows of 3 lines of 13 points plus padding: about 47 points each,
        // 15 to a page.
        assert_eq!(doc.pages.len(), 3);
        let tops = row_tops(&doc);
        let mut seen = Vec::new();
        for page in &tops {
            for &(n, top) in page {
                assert!(top >= MARGINS.top, "row {n} starts above the margin");
                // The last of its three lines still ends above the bottom margin.
                assert!(top + 3.0 * BODY.line_height() <= A4_HEIGHT - MARGINS.bottom + 0.1);
                seen.push(n);
            }
        }
        assert_eq!(
            seen,
            (0..40).collect::<Vec<_>>(),
            "every row exactly once, in order"
        );
        for p in &doc.pages {
            assert_eq!(
                p.content().matches("(A) Tj").count(),
                1,
                "header on every page"
            );
            assert!(p.content().contains(" re\nf\n"), "header background");
        }
        assert!(doc.pages[2].content().contains("(3/3) Tj"));
    }

    #[test]
    fn a_row_taller_than_a_page_continues_on_the_next() {
        let mut flow = Flow::new(Document::new("t"), A4_WIDTH, A4_HEIGHT, MARGINS);
        flow.table(&table(1, 80));
        let doc = flow.finish(BODY, Color::BLACK, |_, _| (String::new(), String::new()));
        assert_eq!(doc.pages.len(), 2);
        let words: usize = doc
            .pages
            .iter()
            .map(|p| p.content().matches("(word) Tj").count())
            .sum();
        assert_eq!(words, 80, "no line lost at the break");
    }

    #[test]
    fn a_heading_moves_with_its_table() {
        let mut flow = Flow::new(Document::new("t"), A4_WIDTH, A4_HEIGHT, MARGINS);
        flow.set_cursor(A4_HEIGHT - MARGINS.bottom - 20.0);
        let t = table(2, 1);
        let keep = flow.table_start_height(&t);
        flow.heading("Heading", BODY, keep);
        flow.table(&t);
        let doc = flow.finish(BODY, Color::BLACK, |_, _| (String::new(), String::new()));
        assert_eq!(doc.pages.len(), 2);
        assert!(!doc.pages[0].content().contains("(Heading)"));
        assert!(doc.pages[1].content().contains("(Heading) Tj"));
    }

    #[test]
    fn paragraphs_flow_onto_new_pages() {
        let mut flow = Flow::new(Document::new("t"), A4_WIDTH, A4_HEIGHT, MARGINS);
        let text = vec!["Lorem ipsum dolor sit amet."; 400].join(" ");
        flow.paragraph(&text, BODY);
        assert!(flow.page_count() >= 2);
        let doc = flow.finish(BODY, Color::BLACK, |_, _| (String::new(), String::new()));
        let words: usize = doc
            .pages
            .iter()
            .map(|p| p.content().matches("Lorem").count())
            .sum();
        assert_eq!(words, 400);
    }
}
