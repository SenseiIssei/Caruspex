//! Command line of caruspex_cli, parsed by hand: the syntax and the messages
//! are the ones scripts already rely on.

/// The help text, byte for byte the one of the C++ version.
pub const USAGE: &str = include_str!("usage.txt");

/// What `std::stoi` accepts: optional white space, a sign, digits (up to the
/// first character that is none); anything else, or a value outside `i32`,
/// is an error.
pub fn stoi(text: &str) -> Option<i32> {
    let (value, _) = strto(text, 10)?;
    i32::try_from(value).ok()
}

/// `std::stol`-style parse with the number of bytes used. Base 16 accepts a
/// "0x" prefix. `None` if there is no digit at all or the value overflows.
pub(crate) fn strto(text: &str, radix: u32) -> Option<(i64, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }
    if radix == 16
        && i + 2 < bytes.len()
        && bytes[i] == b'0'
        && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X')
        && bytes[i + 2].is_ascii_hexdigit()
    {
        i += 2;
    }
    let first = i;
    let mut value: i64 = 0;
    while i < bytes.len() {
        let Some(d) = (bytes[i] as char).to_digit(radix) else {
            break;
        };
        value = value
            .checked_mul(i64::from(radix))?
            .checked_add(i64::from(d))?;
        i += 1;
    }
    if i == first {
        return None;
    }
    Some((if negative { -value } else { value }, i))
}

#[derive(Debug)]
pub struct Options {
    pub tcp: String,
    pub port: String,
    pub baud: i32,
    /// --baud was given; otherwise the usual rates are tried.
    pub baud_given: bool,
    pub seconds: i32,
    /// --seconds was given; `report` has a shorter default.
    pub seconds_given: bool,
    pub verbose: bool,
    pub demo: bool,
    pub json: bool,
    pub fast_timing: bool,
    pub uds_ids: String,
    pub record_path: String,
    pub replay_path: String,
    pub enet: String,
    pub can_spec: String,
    pub pack: String,
    pub csv_path: String,
    /// j1939: bit rate in kbit/s, 0 tries 250 and 500.
    pub rate: i32,
    /// j1939: clear the faults after reading them.
    pub clear: bool,
    /// report: language code of the report.
    pub lang: String,
    /// report: the PDF file, the one argument without a dash.
    pub output: String,
}

impl Options {
    /// Reads everything after the command. The error is the complete text to
    /// print (with the usage text where the C++ version prints it).
    pub fn parse(argv: &[String]) -> Result<Options, String> {
        let mut o = Options {
            tcp: String::new(),
            port: "COM3".to_string(),
            baud: 38400,
            baud_given: false,
            seconds: 15,
            seconds_given: false,
            verbose: false,
            demo: false,
            json: false,
            fast_timing: false,
            uds_ids: String::new(),
            record_path: String::new(),
            replay_path: String::new(),
            enet: String::new(),
            can_spec: String::new(),
            pack: String::new(),
            csv_path: String::new(),
            rate: 0,
            clear: false,
            lang: String::new(),
            output: String::new(),
        };
        let mut i = 2;
        while i < argv.len() {
            let arg = argv[i].as_str();
            let has_value = i + 1 < argv.len();
            let number = |text: &str| -> Result<i32, String> {
                stoi(text).ok_or_else(|| format!("Not a number after {arg}\n"))
            };
            match arg {
                "--tcp" if has_value => {
                    i += 1;
                    o.tcp = argv[i].clone();
                }
                "--enet" if has_value => {
                    i += 1;
                    o.enet = argv[i].clone();
                }
                "--can" if has_value => {
                    i += 1;
                    o.can_spec = argv[i].clone();
                }
                "--port" if has_value => {
                    i += 1;
                    o.port = argv[i].clone();
                }
                "--baud" if has_value => {
                    i += 1;
                    o.baud = number(&argv[i])?;
                    o.baud_given = true;
                }
                "--seconds" if has_value => {
                    i += 1;
                    o.seconds = number(&argv[i])?;
                    o.seconds_given = true;
                }
                "--ids" if has_value => {
                    i += 1;
                    o.uds_ids = argv[i].clone();
                }
                "--record" if has_value => {
                    i += 1;
                    o.record_path = argv[i].clone();
                }
                "--replay" if has_value => {
                    i += 1;
                    o.replay_path = argv[i].clone();
                }
                "--fast-timing" => o.fast_timing = true,
                "--pack" if has_value => {
                    i += 1;
                    o.pack = argv[i].clone();
                }
                "--csv" if has_value => {
                    i += 1;
                    o.csv_path = argv[i].clone();
                }
                "--rate" if has_value => {
                    i += 1;
                    o.rate = number(&argv[i])?;
                }
                "--lang" if has_value => {
                    i += 1;
                    o.lang = argv[i].clone();
                }
                "--clear" => o.clear = true,
                "--demo" => o.demo = true,
                "--json" => o.json = true,
                "-v" => o.verbose = true,
                // The output file of `report`; main refuses it for the
                // other commands with the usual message.
                _ if o.output.is_empty() && !arg.is_empty() && !arg.starts_with('-') => {
                    o.output = arg.to_string();
                }
                _ => return Err(format!("Unknown option: {arg}\n\n{USAGE}")),
            }
            i += 1;
        }
        if o.seconds < 0 {
            o.seconds = 0;
        }
        Ok(o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn stoi_follows_the_c_library() {
        assert_eq!(stoi("12"), Some(12));
        assert_eq!(stoi("  -7x"), Some(-7));
        assert_eq!(stoi("abc"), None);
        assert_eq!(stoi(""), None);
        assert_eq!(stoi("99999999999"), None);
    }

    #[test]
    fn hex_prefix_is_skipped() {
        assert_eq!(strto("0x1F", 16), Some((31, 4)));
        assert_eq!(strto("1F", 16), Some((31, 2)));
        assert_eq!(strto("0", 16), Some((0, 1)));
    }

    #[test]
    fn options_and_defaults() {
        let o = Options::parse(&args(&[
            "cli",
            "read",
            "--demo",
            "--json",
            "--seconds",
            "-3",
        ]))
        .unwrap();
        assert!(o.demo && o.json);
        assert_eq!(o.seconds, 0);
        assert_eq!(o.port, "COM3");
        assert_eq!(o.baud, 38400);
    }

    #[test]
    fn unknown_option_and_missing_value() {
        let e = Options::parse(&args(&["cli", "read", "--bogus"])).unwrap_err();
        assert!(e.starts_with("Unknown option: --bogus\n\nUsage: caruspex_cli"));
        // An option that needs a value but is last is unknown, as before.
        let e = Options::parse(&args(&["cli", "read", "--tcp"])).unwrap_err();
        assert!(e.starts_with("Unknown option: --tcp\n"));
        let e = Options::parse(&args(&["cli", "read", "--baud", "x"])).unwrap_err();
        assert_eq!(e, "Not a number after --baud\n");
    }

    #[test]
    fn report_takes_a_file_and_a_language() {
        let o = Options::parse(&args(&[
            "cli", "report", "--demo", "out.pdf", "--lang", "de",
        ]))
        .unwrap();
        assert_eq!(o.output, "out.pdf");
        assert_eq!(o.lang, "de");
        assert!(!o.seconds_given);
        // A second file name is an unknown option, as before.
        let e = Options::parse(&args(&["cli", "report", "a.pdf", "b.pdf"])).unwrap_err();
        assert!(e.starts_with("Unknown option: b.pdf\n"));
    }
}
