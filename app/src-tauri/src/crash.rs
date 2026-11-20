// Crash reports, only on request. When Caruspex panics, a short report is
// written to the computer: version, system, where it happened and the call
// chain, but nothing about the car (VINs, hex bytes of the traffic and the
// home folder are cut out of the message). Nothing is sent. The interface
// offers the report at once if it is still alive (a panic in the worker
// thread), otherwise at the next start, with a button that opens a
// prefilled GitHub issue the user can read and change before posting.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

static APP: OnceLock<AppHandle> = OnceLock::new();
static DIR: OnceLock<PathBuf> = OnceLock::new();

/// Lines of the call chain kept in a report.
const BACKTRACE_LINES: usize = 60;

/// Where the reports go: next to the garage, in crash-reports/.
/// CARUSPEX_CRASH_DIR lets tests and screenshots use their own.
pub fn reports_dir(data_dir: &Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("CARUSPEX_CRASH_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    data_dir
        .join("Caruspex")
        .join("Caruspex")
        .join("crash-reports")
}

/// A report the user has not decided about yet.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub path: String,
    pub text: String,
}

/// Installs the panic hook. The default hook still prints to stderr.
pub fn install(dir: PathBuf, app: AppHandle) {
    let _ = DIR.set(dir);
    let _ = APP.set(app);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_default();
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let text = report_text(&message, &location, &thread, &backtrace, &home());
        let Some(dir) = DIR.get() else {
            return;
        };
        if let Ok(path) = write_report(dir, &text, &jiff::ZonedDateTime::now()) {
            // The window may still be there (a panic in the worker thread).
            if let Some(app) = APP.get() {
                let _ = app.emit(
                    "crash_report",
                    CrashReport {
                        path: path.display().to_string(),
                        text,
                    },
                );
            }
        }
    }));
}

fn home() -> String {
    dirs::home_dir()
        .map(|h| h.display().to_string())
        .unwrap_or_default()
}

fn vin_char(c: char) -> bool {
    (c.is_ascii_uppercase() && !matches!(c, 'I' | 'O' | 'Q')) || c.is_ascii_digit()
}

/// Cuts out what could say something about a car or a person: VINs, runs of
/// hex bytes (the traffic with the car) and the home folder.
pub fn scrub(text: &str, home: &str) -> String {
    let mut text = text.to_string();
    if home.len() > 3 {
        text = text.replace(home, "~");
        // Backtraces may show the other kind of slash.
        text = text.replace(&home.replace('\\', "/"), "~");
    }
    // Words of 17 VIN characters with letters and digits.
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        while i < chars.len() && chars[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if i > start {
            let word: String = chars[start..i].iter().collect();
            let vin = word.len() == 17
                && word.chars().all(vin_char)
                && word.chars().any(|c| c.is_ascii_digit())
                && word.chars().any(|c| c.is_ascii_uppercase());
            // A long run of hex digits is traffic, not a word.
            let hex = word.len() >= 8 && word.chars().all(|c| c.is_ascii_hexdigit());
            out.push_str(if vin {
                "<VIN>"
            } else if hex {
                "<hex>"
            } else {
                &word
            });
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    // Hex bytes with spaces between them: "49 02 01 57", three or more.
    let mut result = String::with_capacity(out.len());
    let tokens: Vec<&str> = out.split(' ').collect();
    let is_byte = |t: &str| t.len() == 2 && t.chars().all(|c| c.is_ascii_hexdigit());
    let mut k = 0;
    while k < tokens.len() {
        let mut run = k;
        while run < tokens.len() && is_byte(tokens[run]) {
            run += 1;
        }
        if run - k >= 3 {
            result.push_str("<hex>");
            k = run;
        } else {
            result.push_str(tokens[k]);
            k += 1;
        }
        if k < tokens.len() {
            result.push(' ');
        }
    }
    result
}

/// The text of a report.
pub fn report_text(
    message: &str,
    location: &str,
    thread: &str,
    backtrace: &str,
    home: &str,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("Caruspex {}\n", env!("CARGO_PKG_VERSION")));
    text.push_str(&format!(
        "System: {} {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    text.push_str(&format!("Thread: {}\n", scrub(thread, home)));
    text.push_str(&format!("Where: {}\n", scrub(location, home)));
    text.push_str(&format!("Message: {}\n", scrub(message, home)));
    text.push_str("\nCall chain:\n");
    for line in backtrace.lines().take(BACKTRACE_LINES) {
        text.push_str(&scrub(line, home));
        text.push('\n');
    }
    text
}

/// Writes the report into `dir` as crash-<time>.txt.
pub fn write_report(
    dir: &Path,
    text: &str,
    when: &jiff::ZonedDateTime,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let mut path = dir.join(format!("crash-{}.txt", when.strftime("%Y%m%d-%H%M%S")));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("crash-{}-{n}.txt", when.strftime("%Y%m%d-%H%M%S")));
        n += 1;
    }
    std::fs::write(&path, text)?;
    Ok(path)
}

/// The newest report nobody has decided about.
pub fn pending(dir: &Path) -> Option<CrashReport> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("crash-") && name.ends_with(".txt") && !name.ends_with(".seen.txt")
        })
        .collect();
    // By the name without ".txt", so crash-<time>-2 comes after crash-<time>.
    files.sort_by_key(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .trim_end_matches(".txt")
            .to_string()
    });
    let path = files.pop()?;
    let text = std::fs::read_to_string(&path).ok()?;
    Some(CrashReport {
        path: path.display().to_string(),
        text,
    })
}

/// The user decided (opened the issue or let it be): the report stays on
/// the computer as crash-<time>.seen.txt and is not offered again. Only
/// files in the report folder are touched.
pub fn mark_seen(dir: &Path, path: &str) -> Result<(), String> {
    let path = PathBuf::from(path);
    if path.parent() != Some(dir) {
        return Err(format!("not a crash report: {}", path.display()));
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err("no file name".into());
    };
    let Some(stem) = name.strip_suffix(".txt") else {
        return Err(format!("not a crash report: {name}"));
    };
    std::fs::rename(&path, dir.join(format!("{stem}.seen.txt"))).map_err(|e| e.to_string())
}

/// The reports folder of this run, for the commands.
pub fn dir() -> Option<&'static PathBuf> {
    DIR.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_data_and_the_home_folder_are_cut_out() {
        let home = r"C:\Users\someone";
        let text = scrub(
            r"VIN WVWZZZ1KZ6W123456 not decoded, reply 49 02 01 57 56 57 at C:\Users\someone\.cargo\x.rs and 410C1AF8BB00",
            home,
        );
        assert!(!text.contains("WVWZZZ"), "{text}");
        assert!(!text.contains("49 02 01"), "{text}");
        assert!(!text.contains("410C1AF8"), "{text}");
        assert!(!text.contains("someone"), "{text}");
        assert!(text.contains("<VIN>") && text.contains("<hex>"), "{text}");
        assert!(text.contains(r"~\.cargo"), "{text}");
        // Ordinary words and small numbers stay.
        assert_eq!(
            scrub("index out of bounds: the len is 3 but the index is 5", home),
            "index out of bounds: the len is 3 but the index is 5"
        );
        assert_eq!(scrub("at 0x1f in core", home), "at 0x1f in core");
    }

    #[test]
    fn a_report_names_version_place_and_message() {
        let text = report_text(
            "boom WVWZZZ1KZ6W123456",
            "app/src-tauri/src/worker.rs:12:5",
            "caruspex-worker",
            "   0: std::panicking\n   1: caruspex::worker::Worker::run",
            "",
        );
        assert!(text.starts_with(&format!("Caruspex {}", env!("CARGO_PKG_VERSION"))));
        assert!(text.contains("Where: app/src-tauri/src/worker.rs:12:5"));
        assert!(text.contains("Message: boom <VIN>"));
        assert!(text.contains("caruspex::worker::Worker::run"));
    }

    #[test]
    fn reports_are_offered_once() {
        let dir = std::env::temp_dir().join(format!("caruspex-crash-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(pending(&dir).is_none());
        let when = jiff::ZonedDateTime::now();
        let first = write_report(&dir, "one", &when).unwrap();
        let second = write_report(&dir, "two", &when).unwrap();
        assert_ne!(first, second);
        let offered = pending(&dir).unwrap();
        assert_eq!(offered.text, "two");
        mark_seen(&dir, &offered.path).unwrap();
        assert_eq!(pending(&dir).unwrap().text, "one");
        mark_seen(&dir, &pending(&dir).unwrap().path).unwrap();
        assert!(pending(&dir).is_none());
        // The reports stay on the computer.
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        // Nothing outside the folder is renamed.
        assert!(mark_seen(&dir, r"C:\Windows\win.ini").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
