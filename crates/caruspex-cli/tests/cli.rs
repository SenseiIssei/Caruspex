//! Runs the built `caruspex_cli` binary. The files in `tests/golden/` are what
//! the C++ version printed for the same command line (line endings
//! normalised), so a difference here is a difference to the old tool.

use std::path::PathBuf;
use std::process::Command;

struct Run {
    out: String,
    err: String,
    code: i32,
}

fn run(args: &[&str]) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_caruspex_cli"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the binary starts");
    Run {
        out: String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        err: String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n"),
        code: output.status.code().unwrap_or(-1),
    }
}

fn golden(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "golden", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

fn check(args: &[&str], golden_name: &str) {
    let r = run(args);
    assert_eq!(r.out, golden(golden_name), "{args:?}");
}

const DEMO_REC: &str = "tests/data/demo.jsonl";
const UDS_REC: &str = "tests/data/demo-uds.jsonl";

// ---------------------------------------------------------------------------
// Text of the tool itself
// ---------------------------------------------------------------------------

#[test]
fn help_is_the_old_help() {
    check(&["help"], "help.txt");
    check(&["--help"], "help.txt");
    check(&["-h"], "help.txt");
    assert_eq!(run(&["help"]).code, 0);
}

#[test]
fn version_names_the_tool() {
    let r = run(&["version"]);
    assert_eq!(
        r.out,
        format!("caruspex_cli {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(run(&["--version"]).out, r.out);
}

#[test]
fn unknown_command_and_option_print_the_usage_and_exit_1() {
    check(&["bogus"], "unknown_command.txt");
    assert_eq!(run(&["bogus"]).code, 1);
    check(&["read", "--bogus"], "unknown_option.txt");
    assert_eq!(run(&["read", "--bogus"]).code, 1);
}

#[test]
fn option_without_value_is_an_unknown_option() {
    let r = run(&["read", "--tcp"]);
    assert!(
        r.out
            .starts_with("Unknown option: --tcp\n\nUsage: caruspex_cli")
    );
    assert_eq!(r.code, 1);
}

#[test]
fn bad_number_is_reported() {
    check(&["read", "--baud", "x"], "not_a_number.txt");
    assert_eq!(run(&["read", "--baud", "x"]).code, 1);
    assert_eq!(
        run(&["health", "--seconds", "1x2", "--demo"])
            .out
            .lines()
            .next(),
        Some("Adapter    : ELM327 v1.5"),
        "like std::stoi, trailing text after the digits is ignored"
    );
}

// ---------------------------------------------------------------------------
// The demo car, against what the C++ tool printed
// ---------------------------------------------------------------------------

#[test]
fn demo_read() {
    check(&["read", "--demo"], "demo_read.txt");
    check(&["read", "--demo", "--json"], "demo_read_json.txt");
}

#[test]
fn demo_vin() {
    check(&["vin", "--demo"], "demo_vin.txt");
    check(&["vin", "--demo", "--json"], "demo_vin_json.txt");
}

#[test]
fn demo_freeze() {
    check(&["freeze", "--demo"], "demo_freeze.txt");
    check(&["freeze", "--demo", "--json"], "demo_freeze_json.txt");
}

#[test]
fn demo_info_and_clear() {
    check(&["info", "--demo"], "demo_info.txt");
    check(&["clear", "--demo"], "demo_clear.txt");
}

#[test]
fn demo_live_prints_the_usual_values() {
    let r = run(&["live", "--demo"]);
    assert_eq!(r.code, 0);
    let rows: Vec<&str> = r.out.lines().skip_while(|l| !l.starts_with("  ")).collect();
    assert_eq!(rows.len(), 12);
    assert!(rows.iter().any(|l| l.contains("Engine speed")));
    // "  %-40s %10.2f %s"
    let rpm = rows.iter().find(|l| l.contains("Engine speed")).unwrap();
    assert!(rpm.ends_with(" 1/min"));
    assert!(rpm.contains('.'));
}

#[test]
fn demo_ecus_lists_every_unit() {
    let r = run(&["ecus", "--demo"]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains(" MIL "));
    assert!(r.out.contains("CALID"));
    assert!(r.out.contains("spark ignition"));
}

#[test]
fn demo_probe() {
    let r = run(&["probe", "--demo"]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains("\nCommands:\n"));
    assert!(r.out.contains("\nVerdict: "));
    let json = run(&["probe", "--demo", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json.out).unwrap();
    assert_eq!(v["reachable"], true);
    assert!(v["checks"].as_array().unwrap().len() > 5);
}

#[test]
fn demo_health_has_the_report_and_valid_json() {
    let r = run(&["health", "--demo", "--seconds", "1"]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains("Reading readiness and warning lamp ...\n"));
    assert!(r.out.contains("Collecting 18 live values for 1 s ...\n"));
    assert!(r.out.contains("/100 ("));
    let j = run(&["health", "--demo", "--seconds", "1", "--json"]);
    // Progress lines stay out of the JSON.
    let v: serde_json::Value = serde_json::from_str(&j.out).unwrap();
    assert_eq!(v["liveSeries"]["seconds"], 1);
    assert!(v["liveSeries"]["snapshots"].as_i64().unwrap() >= 1);
    assert!(v["brand"].as_str().is_some());
}

#[test]
fn json_keys_keep_the_old_order() {
    let j = run(&["read", "--demo", "--json"]);
    let keys: Vec<&str> = j
        .out
        .lines()
        .filter(|l| l.starts_with("  \""))
        .map(|l| l.trim_start().split('"').nth(1).unwrap())
        .collect();
    assert_eq!(keys, ["adapter", "vin", "brand", "status", "faults"]);
}

// ---------------------------------------------------------------------------
// UDS
// ---------------------------------------------------------------------------

/// The time of the run is the only thing that differs between two runs.
fn without_timing(text: &str) -> String {
    text.lines()
        .filter(|l| !l.contains(" s.") && !l.trim_start().starts_with("\"seconds\""))
        .map(|l| format!("{l}\n"))
        .collect()
}

#[test]
fn uds_from_a_recording_matches_the_old_output() {
    let r = run(&["uds", "--replay", UDS_REC]);
    assert_eq!(
        without_timing(&r.out),
        without_timing(&golden("replay_uds.txt"))
    );
    let r = run(&["uds", "--replay", UDS_REC, "--json"]);
    assert_eq!(
        without_timing(&r.out),
        without_timing(&golden("replay_uds_json.txt"))
    );
}

#[test]
fn uds_demo_finds_its_six_units() {
    let r = run(&["uds", "--demo", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(v["asked"], 6);
    assert_eq!(v["present"], 6);
    // Progress goes to stderr, only without --json.
    assert!(r.err.is_empty());
    let r = run(&["uds", "--demo"]);
    assert!(r.err.contains("[1/6] Engine ..."));
    assert!(r.out.contains("6 of 6 control units answered, "));
}

#[test]
fn uds_ids_are_parsed_like_before() {
    let r = run(&["uds", "--demo", "--ids", "760,b12,18DA28F1,0x737"]);
    assert!(r.out.contains("ECU 760 (760/768): present"));
    assert!(
        r.out
            .contains("ECU 18DA28F1 (18DA28F1/18DAF128): no answer")
    );
    assert!(r.out.contains("Engine control (DME) (6F1/612): no answer"));
    assert!(r.out.contains("ECU 737 (737/73F)"));
    assert!(r.out.contains("of 4 control units answered"));
    check(
        &["uds", "--demo", "--ids", "zz", "--json"],
        "uds_bad_id.txt",
    );
    assert_eq!(
        run(&["uds", "--demo", "--ids", "zz"]).out,
        "Not a hex id: zz\n"
    );
    assert_eq!(
        run(&["uds", "--demo", "--ids", "bzz"]).out,
        "Not a BMW address: bzz\n"
    );
    assert_eq!(run(&["uds", "--demo", "--ids", "zz"]).code, 1);
}

// ---------------------------------------------------------------------------
// Custom values
// ---------------------------------------------------------------------------

#[test]
fn custom_without_a_pack_lists_the_packs() {
    check(&["custom"], "custom_nothing.txt");
    check(
        &["custom", "--demo", "--pack", "nope"],
        "custom_unknown_pack.txt",
    );
    assert_eq!(run(&["custom"]).code, 1);
}

#[test]
fn custom_demo_reads_the_demo_pack() {
    let r = run(&["custom", "--demo"]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains("Caruspex demo\n"));
    assert!(r.out.contains("9 of 9 values read, "), "{}", r.out);
    let j = run(&["custom", "--demo", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&j.out).unwrap();
    assert_eq!(v["title"], "Caruspex demo");
    assert_eq!(v["read"], 9);
    assert_eq!(v["failed"], 0);
    assert_eq!(v["values"].as_array().unwrap().len(), 9);
}

#[test]
fn custom_csv_import_reports_what_it_skipped() {
    let dir = std::env::temp_dir().join(format!("caruspex_cli_csv_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("values.csv");
    std::fs::write(
        &file,
        "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n\
         Engine speed,RPM,010C,((A*256)+B)/4,0,8000,rpm,\n\
         Broken,BRK,zz,A,0,1,,\n",
    )
    .unwrap();
    let r = run(&["custom", "--demo", "--csv", file.to_str().unwrap()]);
    assert!(r.out.contains("Skipped: "));
    assert!(r.out.contains("Engine speed"));
    assert!(r.out.contains("1 of 1 values read, "));
    let missing = run(&[
        "custom",
        "--demo",
        "--csv",
        dir.join("none.csv").to_str().unwrap(),
    ]);
    assert!(missing.out.starts_with("Cannot open "));
    assert_eq!(missing.code, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Recording and replay
// ---------------------------------------------------------------------------

#[test]
fn replay_matches_the_old_output() {
    for (args, name) in [
        (vec!["read"], "replay_read.txt"),
        (vec!["read", "--json"], "replay_read_json.txt"),
        (vec!["vin"], "replay_vin.txt"),
        (vec!["freeze", "--json"], "replay_freeze_json.txt"),
        (vec!["health", "--seconds", "0"], "replay_health.txt"),
        (
            vec!["health", "--seconds", "0", "--json"],
            "replay_health_json.txt",
        ),
        (vec!["probe"], "replay_probe.txt"),
        (vec!["ecus"], "replay_ecus.txt"),
    ] {
        let mut full = args.clone();
        full.extend(["--replay", DEMO_REC]);
        check(&full, name);
    }
}

#[test]
fn replay_of_a_missing_file_fails_with_code_2() {
    let r = run(&["read", "--replay", "tests/data/none.jsonl"]);
    assert!(r.out.starts_with("Cannot open the recording: "));
    assert_eq!(r.code, 2);
    let j = run(&["read", "--replay", "tests/data/none.jsonl", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&j.out).unwrap();
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .starts_with("Cannot open the recording")
    );
}

#[test]
fn a_recorded_session_plays_back_the_same() {
    let dir = std::env::temp_dir().join(format!("caruspex_cli_rec_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("session.jsonl");
    let path = file.to_str().unwrap();

    let live = run(&["read", "--demo", "--record", path]);
    assert_eq!(live.code, 0);
    assert!(live.out.contains(&format!("Recording  : {path}\n")));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.starts_with("{\"format\":\"caruspex-recording\""));

    let played = run(&["read", "--replay", path]);
    assert_eq!(played.code, 0);
    // Same fault list and status, apart from the header of the replay.
    let tail = |s: &str| s.split("\nVIN: ").nth(1).map(str::to_string);
    assert_eq!(tail(&played.out), tail(&live.out));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn verbose_log_goes_to_stderr_with_json() {
    let r = run(&["read", "--demo", "--json", "-v"]);
    assert!(r.err.contains("ATZ"));
    serde_json::from_str::<serde_json::Value>(&r.out).unwrap();
    let r = run(&["read", "--demo", "-v"]);
    assert!(r.out.contains("ATZ"));
}

#[test]
fn fast_timing_is_shown() {
    let r = run(&["read", "--demo", "--fast-timing"]);
    assert!(r.out.contains("Timing     : ATAT"));
}

// ---------------------------------------------------------------------------
// Connections that cannot work
// ---------------------------------------------------------------------------

#[test]
fn missing_connections_fail_with_the_old_messages() {
    let r = run(&["read", "--tcp", "127.0.0.1:1"]);
    assert!(
        r.out
            .starts_with("Connection failed: No connection to 127.0.0.1:1")
    );
    assert_eq!(r.code, 2);
    let r = run(&["read", "--tcp", "127.0.0.1:1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .starts_with("Connection failed: ")
    );
    let r = run(&["read", "--tcp", "127.0.0.1:x"]);
    assert_eq!(r.code, 2);
}

#[test]
fn kklcheck_without_the_port_stops_at_once() {
    let r = run(&["kklcheck", "--port", "NO_SUCH_PORT"]);
    assert!(r.out.starts_with(
        "KKL self-test on NO_SUCH_PORT\nThe cable does NOT need to be plugged into the car for this.\n\n[fail]  Cannot open the port: "
    ));
    assert_eq!(r.code, 2);
}

#[test]
fn interfaces_prints_something() {
    let r = run(&["interfaces"]);
    assert_eq!(r.code, 0);
    assert!(!r.out.is_empty());
}
