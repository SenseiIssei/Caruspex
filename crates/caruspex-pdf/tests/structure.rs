//! Reads the written file back the way a viewer does: trailer, startxref,
//! the cross-reference table, every object at its offset, every stream with
//! its /Length. If `qpdf` is on the path it checks the file as well.

use caruspex_pdf::{A4_HEIGHT, A4_WIDTH, Cell, Color, Document, Flow, Font, Margins, Style, Table};

const BODY: Style = Style::new(Font::Helvetica, 10.0, Color::BLACK);

fn sample(rows: usize) -> Vec<u8> {
    let margins = Margins {
        top: 50.0,
        right: 50.0,
        bottom: 60.0,
        left: 50.0,
    };
    let mut doc = Document::new("Pr\u{00FC}fbericht (Test) \\ \u{0436}");
    doc.producer = "caruspex-pdf test".into();
    doc.creation_date = "D:20261120120000".into();
    let mut flow = Flow::new(doc, A4_WIDTH, A4_HEIGHT, margins);
    flow.paragraph(
        "Klammern (wie hier) und \\ Backslash, Gr\u{00FC}\u{00DF}e",
        BODY,
    );
    let table = Table {
        widths: vec![0.2, 0.8],
        header: vec!["Code".into(), "Bedeutung".into()],
        header_style: BODY,
        header_fill: Color::hex(0xEEF0F6),
        rule: Color::hex(0xE2E5EE),
        rows: (0..rows)
            .map(|i| {
                vec![
                    Cell::new(
                        Style::new(Font::CourierBold, 10.0, Color::BLACK),
                        &format!("P{i:04}"),
                    ),
                    Cell::new(BODY, "System too lean (bank 1)"),
                ]
            })
            .collect(),
    };
    flow.table(&table);
    flow.finish(BODY, Color::BLACK, |p, n| {
        ("Caruspex".into(), format!("Page {p} of {n}"))
    })
    .to_bytes()
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

fn number_after(hay: &[u8], key: &[u8]) -> usize {
    let at =
        find(hay, key, 0).unwrap_or_else(|| panic!("{}", String::from_utf8_lossy(key))) + key.len();
    let digits: String = hay[at..]
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .map(|&b| b as char)
        .collect();
    digits.parse().unwrap()
}

/// Checks the file and returns the number of pages.
fn check(pdf: &[u8]) -> usize {
    assert!(pdf.starts_with(b"%PDF-1.4\n"));
    assert!(pdf.ends_with(b"%%EOF\n"));

    // startxref points at the table.
    let sx = rfind(pdf, b"startxref\n").unwrap();
    let xref: usize = number_after(&pdf[sx..], b"startxref");
    assert_eq!(&pdf[xref..xref + 5], b"xref\n");

    let trailer = &pdf[rfind(pdf, b"trailer").unwrap()..];
    let size = number_after(trailer, b"/Size");

    // "0 N" and N entries of exactly 20 bytes.
    let header_end = find(pdf, b"\n", xref + 5).unwrap();
    assert_eq!(
        std::str::from_utf8(&pdf[xref + 5..header_end]).unwrap(),
        format!("0 {size}")
    );
    let entries = &pdf[header_end + 1..];
    assert_eq!(&entries[..20], b"0000000000 65535 f \n");
    let mut objects = 0;
    for n in 1..size {
        let e = &entries[20 * n..20 * n + 20];
        assert_eq!(&e[10..], b" 00000 n \n", "entry {n}");
        let offset: usize = std::str::from_utf8(&e[..10]).unwrap().parse().unwrap();
        let head = format!("{n} 0 obj\n");
        assert_eq!(
            &pdf[offset..offset + head.len()],
            head.as_bytes(),
            "object {n} at {offset}"
        );
        objects += 1;
    }
    assert_eq!(&entries[20 * size..20 * size + 8], b"trailer\n");
    // As many objects in the file as the table lists.
    assert_eq!(pdf.windows(7).filter(|w| w == b" 0 obj\n").count(), objects);
    assert_eq!(pdf.windows(7).filter(|w| w == b"endobj\n").count(), objects);

    // Every stream is as long as it says.
    let mut at = 0;
    let mut streams = 0;
    while let Some(s) = find(pdf, b"stream\n", at) {
        if pdf[..s].ends_with(b"end") {
            at = s + 7;
            continue;
        }
        let dict_start = rfind(&pdf[..s], b"<< /Length ").unwrap();
        let length = number_after(&pdf[dict_start..s], b"/Length");
        let data = s + 7;
        assert_eq!(
            &pdf[data + length..data + length + 10],
            b"\nendstream",
            "stream at {s}"
        );
        at = data + length;
        streams += 1;
    }

    let pages = number_after(pdf, b"/Count");
    assert_eq!(streams, pages, "one content stream per page");
    assert_eq!(size - 1, 8 + 2 * pages, "fixed objects plus two per page");
    assert!(pdf.is_ascii() || pdf[9..14] == *b"%\xE2\xE3\xCF\xD3");
    assert!(
        pdf[15..].is_ascii(),
        "everything after the binary marker is ASCII"
    );
    pages
}

#[test]
fn one_page_document_is_well_formed() {
    let pdf = sample(3);
    assert_eq!(check(&pdf), 1);
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/BaseFont /Helvetica /Encoding /WinAnsiEncoding"));
    assert!(text.contains("(P0002) Tj"));
    assert!(text.contains("(Page 1 of 1) Tj"));
    // ( ) and \ escaped, ü and ß as octal WinAnsi codes.
    assert!(text.contains("(Klammern \\(wie hier\\) und \\\\ Backslash, Gr\\374\\337e) Tj"));
    // The title is UTF-16, so the Cyrillic letter survives there.
    assert!(text.contains("/Title <FEFF0050007200FC"));
    assert!(text.contains("0436>"));
}

#[test]
fn long_tables_make_more_pages_and_the_file_stays_consistent() {
    let pdf = sample(150);
    let pages = check(&pdf);
    assert!(pages >= 3, "{pages}");
    let text = String::from_utf8_lossy(&pdf);
    for i in 0..150 {
        assert_eq!(text.matches(&format!("(P{i:04}) Tj")).count(), 1, "row {i}");
    }
    assert!(text.contains(&format!("(Page {pages} of {pages}) Tj")));
}

#[test]
fn empty_document_still_has_a_page() {
    let pdf = Document::new("").to_bytes();
    assert_eq!(check(&pdf), 1);
}

/// qpdf is not needed; when it is installed it gets the last word.
#[test]
fn qpdf_accepts_the_file_if_installed() {
    let Ok(probe) = std::process::Command::new("qpdf").arg("--version").output() else {
        return;
    };
    if !probe.status.success() {
        return;
    }
    let path = std::env::temp_dir().join(format!("caruspex-pdf-{}.pdf", std::process::id()));
    std::fs::write(&path, sample(150)).unwrap();
    let out = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}
