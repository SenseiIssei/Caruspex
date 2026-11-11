// The command line of main.cpp, as far as it makes sense for this app:
//
//   caruspex --demo
//   caruspex --tcp 192.168.0.10:35000
//   caruspex --language en --theme light --page faults
//   caruspex --demo --report bericht.pdf
use crate::texts::main_text;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cli {
    pub tcp: Option<String>,
    pub port: Option<String>,
    pub bluetooth: Option<String>,
    pub enet: Option<String>,
    pub can: Option<String>,
    pub report: Option<String>,
    pub report_wait_ms: u64,
    pub language: Option<String>,
    pub demo: bool,
    pub replay: Option<String>,
    pub page: Option<String>,
    pub shot: Option<String>,
    pub shot_delay_ms: u64,
    pub size: Option<(f64, f64)>,
    /// Developer aid: where the window opens (logical pixels), without
    /// taking the focus, so test runs stay off the screen in use.
    pub window_pos: Option<(f64, f64)>,
    pub theme: Option<String>,
    /// "beginner" or "pro" for this run, without changing the setting.
    pub mode: Option<String>,
    pub help: bool,
    pub version: bool,
}

impl Cli {
    /// Any option that connects at start.
    pub fn connects(&self) -> bool {
        self.tcp.is_some()
            || self.demo
            || self.replay.is_some()
            || self.port.is_some()
            || self.bluetooth.is_some()
            || self.enet.is_some()
            || self.can.is_some()
    }

    /// No network request for screenshots, reports and other unattended
    /// runs (CARUSPEX_CHECK_UPDATES overrides that).
    pub fn allows_update_check(&self) -> bool {
        (self.shot.is_none() && self.report.is_none())
            || std::env::var_os("CARUSPEX_CHECK_UPDATES").is_some()
    }
}

const WITH_VALUE: [&str; 16] = [
    "mode",
    "tcp",
    "port",
    "bluetooth",
    "enet",
    "can",
    "report",
    "report-wait",
    "language",
    "replay",
    "page",
    "shot",
    "shot-delay",
    "size",
    "theme",
    "window-pos",
];

/// Parses the arguments after the program name. Unknown arguments are
/// passed over, so the developer tools can add their own.
pub fn parse<I: IntoIterator<Item = String>>(arguments: I) -> Result<Cli, String> {
    let mut cli = Cli {
        report_wait_ms: 8000,
        shot_delay_ms: 5200,
        ..Cli::default()
    };
    let mut items = arguments.into_iter().peekable();
    while let Some(arg) = items.next() {
        let Some(option) = arg.strip_prefix("--") else {
            continue;
        };
        let (name, inline) = match option.split_once('=') {
            Some((n, v)) => (n.to_string(), Some(v.to_string())),
            None => (option.to_string(), None),
        };
        let value = if WITH_VALUE.contains(&name.as_str()) {
            match inline.or_else(|| items.next()) {
                Some(v) => Some(v),
                None => return Err(format!("--{name}: ?")),
            }
        } else {
            None
        };
        match name.as_str() {
            "tcp" => cli.tcp = value,
            "port" => cli.port = value,
            "bluetooth" => cli.bluetooth = value,
            "enet" => cli.enet = value,
            "can" => cli.can = value,
            "report" => cli.report = value,
            "report-wait" => {
                cli.report_wait_ms = value.and_then(|v| v.parse().ok()).unwrap_or(8000)
            }
            "language" => cli.language = value,
            "demo" => cli.demo = true,
            "replay" => cli.replay = value,
            "page" => cli.page = value,
            "shot" => cli.shot = value,
            "shot-delay" => cli.shot_delay_ms = value.and_then(|v| v.parse().ok()).unwrap_or(5200),
            "size" => {
                cli.size = value.and_then(|v| {
                    let (w, h) = v.split_once('x')?;
                    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
                })
            }
            "window-pos" => {
                cli.window_pos = value.and_then(|v| {
                    let (x, y) = v.split_once(',')?;
                    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
                })
            }
            "theme" => cli.theme = value,
            "mode" => cli.mode = value,
            "help" => cli.help = true,
            "version" => cli.version = true,
            _ => {}
        }
    }
    Ok(cli)
}

/// The help text, in the active language.
pub fn help_text() -> String {
    let lines: [(&str, String); 15] = [
        (
            "--tcp <host:port>",
            main_text(
                "WLAN-Adapter direkt verbinden, z.B. 192.168.0.10:35000",
                "Connect a Wi-Fi adapter directly, e.g. 192.168.0.10:35000",
            ),
        ),
        (
            "--port <port|auto>",
            main_text(
                "Adapter am seriellen Anschluss verbinden, z.B. COM3 oder /dev/ttyUSB0, auto sucht an allen",
                "Connect an adapter on a serial port, e.g. COM3 or /dev/ttyUSB0, auto searches all of them",
            ),
        ),
        (
            "--bluetooth <address|auto>",
            main_text(
                "Bluetooth-Adapter verbinden, über die Adresse oder auto für die Suche",
                "Connect a Bluetooth adapter by its address, or auto to search",
            ),
        ),
        (
            "--enet <address|auto>",
            main_text(
                "BMW oder Mini über ein ENET-Kabel verbinden, über die Adresse oder auto für die Suche",
                "Connect a BMW or Mini over an ENET cable, by its address or auto to search",
            ),
        ),
        (
            "--can <spec>",
            main_text(
                "CAN-Interface verbinden: j2534:<treiber.dll>, slcan:COM5 oder socketcan:can0, mit @250000 für die Bitrate",
                "Connect a CAN interface: j2534:<driver.dll>, slcan:COM5 or socketcan:can0, with @250000 for the bit rate",
            ),
        ),
        (
            "--report <file>",
            main_text(
                "Nach dem Verbinden einen Prüfbericht schreiben (PDF, oder HTML bei .html) und beenden",
                "After connecting, write a diagnostic report (PDF, or HTML for .html) and quit",
            ),
        ),
        (
            "--language <de|en|fr|...>",
            main_text(
                "Sprache für diesen Start: de oder en",
                "Language for this run: de or en",
            ),
        ),
        (
            "--demo",
            main_text(
                "Gleich mit dem eingebauten Demo-Auto verbinden",
                "Connect straight to the built-in demo car",
            ),
        ),
        (
            "--replay <file.jsonl>",
            main_text(
                "Eine aufgezeichnete Sitzung abspielen",
                "Play back a recorded session",
            ),
        ),
        (
            "--page <name>",
            main_text(
                "Mit dieser Seite starten, z.B. faults oder live",
                "Start on this page, e.g. faults or live",
            ),
        ),
        (
            "--shot <file.png>",
            main_text(
                "Nach ein paar Sekunden ein Bild des Fensters speichern und beenden (für Dokumentation)",
                "Save a picture of the window after a few seconds and quit (for documentation)",
            ),
        ),
        (
            "--size <WxH>",
            main_text("Fenstergröße, z.B. 1440x920", "Window size, e.g. 1440x920"),
        ),
        (
            "--theme <dark|light>",
            main_text(
                "Erscheinungsbild für diesen Start: dark oder light",
                "Appearance for this run: dark or light",
            ),
        ),
        (
            "--mode <beginner|pro>",
            // New since the Qt version: a core text, translated with the
            // other texts of "Was soll ich jetzt tun?".
            caruspex_i18n::l(
                "Einsteiger- oder Profimodus für diesen Start",
                "Beginner or pro mode for this run",
            )
            .to_string(),
        ),
        ("--help, --version", String::new()),
    ];
    let mut out = main_text(
        "Fahrzeugdiagnose über EOBD mit ELM327-kompatiblen Adaptern",
        "Vehicle diagnostics over EOBD with ELM327-compatible adapters",
    );
    out.push_str("\n\n");
    for (option, text) in lines {
        out.push_str(&format!("  {option:<28} {text}\n"));
    }
    out
}

/// What the page reads from its address at start (main.tsx): ?lang, ?theme,
/// ?page, ?demo. The page only goes there when no connection comes, because
/// a connection jumps to the start page; otherwise it follows later.
pub fn start_query(cli: &Cli) -> String {
    fn enc(v: &str) -> String {
        v.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect()
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(l) = &cli.language {
        parts.push(format!("lang={}", enc(l)));
    }
    if let Some(t) = &cli.theme {
        parts.push(format!("theme={}", enc(t)));
    }
    if let Some(p) = &cli.page
        && !cli.connects()
    {
        parts.push(format!("page={}", enc(p)));
    }
    if let Some(m) = &cli.mode {
        parts.push(format!("mode={}", enc(m)));
    }
    if cli.demo {
        parts.push("demo".into());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_like_main_cpp() {
        let cli = parse(args(&[
            "--demo",
            "--page=faults",
            "--language",
            "en",
            "--size",
            "1280x800",
        ]))
        .unwrap();
        assert!(cli.demo);
        assert_eq!(cli.page.as_deref(), Some("faults"));
        assert_eq!(cli.language.as_deref(), Some("en"));
        assert_eq!(cli.size, Some((1280.0, 800.0)));
        let placed = parse(args(&["--window-pos", "-1880,40"])).unwrap();
        assert_eq!(placed.window_pos, Some((-1880.0, 40.0)));
        assert_eq!(cli.report_wait_ms, 8000);
        assert!(cli.connects());
        assert_eq!(start_query(&cli), "?lang=en&demo");
        let plain = parse(args(&["--theme", "light", "--page", "live"])).unwrap();
        assert_eq!(start_query(&plain), "?theme=light&page=live");
        let beginner = parse(args(&["--mode", "beginner", "--demo"])).unwrap();
        assert_eq!(start_query(&beginner), "?mode=beginner&demo");
        assert!(parse(args(&["--tcp"])).is_err());
        assert!(help_text().contains("--report"));
    }
}
