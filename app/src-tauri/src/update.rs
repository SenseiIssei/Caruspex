// The update check of MainWindow::checkForUpdates: one request to GitHub
// for the newest release, without anything about the car or the user.
use serde_json::{Value, json};

const RELEASES: &str = "https://api.github.com/repos/SenseiIssei/Caruspex/releases/latest";

/// The numeric segments at the start of a version text, like
/// QVersionNumber::fromString ("v1.2.3-beta" -> [1, 2, 3]).
pub fn version_segments(text: &str) -> Vec<u64> {
    let text = text.strip_prefix('v').unwrap_or(text);
    let mut out = Vec::new();
    for part in text.split('.') {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            break;
        }
        out.push(digits.parse().unwrap_or(0));
        if digits.len() != part.len() {
            break;
        }
    }
    out
}

/// QVersionNumber ordering: segment by segment, and with an equal common
/// part the longer number is the newer one.
pub fn is_newer(candidate: &[u64], mine: &[u64]) -> bool {
    for (a, b) in candidate.iter().zip(mine) {
        if a != b {
            return a > b;
        }
    }
    candidate.len() > mine.len()
}

fn segments_text(v: &[u64]) -> String {
    v.iter().map(u64::to_string).collect::<Vec<_>>().join(".")
}

/// Null when up to date, offline or anything else goes wrong (offline: say
/// nothing).
pub fn check(current: &str) -> Value {
    let agent = format!("Caruspex/{current}");
    // timeout_global: the 8 seconds cover the whole request, from resolving
    // the name to the last byte of the body.
    let response = ureq::get(RELEASES)
        .header("User-Agent", &agent)
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(8)))
        .build()
        .call();
    let Ok(mut response) = response else {
        return Value::Null;
    };
    let Ok(release) = response.body_mut().read_json::<Value>() else {
        return Value::Null;
    };
    let tag = release["tag_name"].as_str().unwrap_or("");
    let url = release["html_url"].as_str().unwrap_or("");
    if tag.is_empty() || url.is_empty() {
        return Value::Null;
    }
    let newest = version_segments(tag);
    if newest.is_empty() || !is_newer(&newest, &version_segments(current)) {
        return Value::Null;
    }
    json!({ "version": segments_text(&newest), "url": url })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_like_qversionnumber() {
        assert_eq!(version_segments("v1.2.3-beta"), vec![1, 2, 3]);
        assert!(is_newer(
            &version_segments("v0.10.0"),
            &version_segments("0.9.0")
        ));
        assert!(!is_newer(
            &version_segments("0.9.0"),
            &version_segments("0.9.0")
        ));
        assert!(is_newer(
            &version_segments("0.9.0.1"),
            &version_segments("0.9.0")
        ));
        assert!(!is_newer(
            &version_segments("0.8.9"),
            &version_segments("0.9")
        ));
    }
}
