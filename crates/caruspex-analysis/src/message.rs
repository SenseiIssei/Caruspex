//! Texts of the vehicle checks (used car, truck health, used truck): the
//! German and English pattern with its arguments, rendered in the active
//! language when shown. The patterns are looked up in the catalog like every
//! other core text, so a report is translated into all languages, numbers
//! included.

use caruspex_i18n::{Language, Text, l, language};

use crate::common::format_number;

/// One argument of a [`Message`].
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// A number with this many decimals, written the way the language does.
    Number(f64, i32),
    /// A text that is the same in every language (a VIN, a fault code) or
    /// already in the active language.
    Plain(String),
    /// A fixed name in both languages (a readiness monitor).
    Name(Text),
    /// Several messages, joined with a comma.
    List(Vec<Message>),
    /// Several longer messages, one per line (the page keeps line breaks).
    Lines(Vec<Message>),
}

/// A text with the placeholders %1 to %9, rendered in the active language.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Message {
    pub de: &'static str,
    pub en: &'static str,
    pub args: Vec<Arg>,
}

/// A message from its German and English pattern.
pub fn text(de: &'static str, en: &'static str) -> Message {
    Message {
        de,
        en,
        args: Vec::new(),
    }
}

/// A message that only carries a ready text (the health check summary).
pub fn raw(value: impl Into<String>) -> Message {
    Message {
        de: "",
        en: "",
        args: vec![Arg::Plain(value.into())],
    }
}

impl Message {
    pub fn num(mut self, value: f64, decimals: i32) -> Self {
        self.args.push(Arg::Number(value, decimals));
        self
    }

    pub fn plain(mut self, value: impl Into<String>) -> Self {
        self.args.push(Arg::Plain(value.into()));
        self
    }

    pub fn name(mut self, value: Text) -> Self {
        self.args.push(Arg::Name(value));
        self
    }

    pub fn list(mut self, items: Vec<Message>) -> Self {
        self.args.push(Arg::List(items));
        self
    }

    /// Like `list`, but one item per line, for entries too long to read
    /// run together.
    pub fn lines(mut self, items: Vec<Message>) -> Self {
        self.args.push(Arg::Lines(items));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.en.is_empty() && self.args.is_empty()
    }

    /// The text in the active language.
    pub fn text(&self) -> String {
        let rendered: Vec<String> = self.args.iter().map(render_arg).collect();
        if self.en.is_empty() {
            return rendered.join(" ");
        }
        fill(l(self.de, self.en), &rendered)
    }
}

fn render_arg(arg: &Arg) -> String {
    match arg {
        Arg::Number(value, decimals) => {
            format_number(*value, *decimals, language() == Language::German)
        }
        Arg::Plain(s) => s.clone(),
        Arg::Name(t) => t.get().to_string(),
        Arg::List(items) => items
            .iter()
            .map(Message::text)
            .collect::<Vec<_>>()
            .join(", "),
        Arg::Lines(items) => items.iter().map(Message::text).collect::<Vec<_>>().join(
            ";
",
        ),
    }
}

/// Fills %1 to %9 in one pass, like QString::arg.
fn fill(pattern: &str, values: &[String]) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%'
            && let Some(&d) = chars.peek()
            && let Some(n) = d.to_digit(10)
            && n >= 1
            && (n as usize) <= values.len()
        {
            chars.next();
            out.push_str(&values[n as usize - 1]);
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_fill_their_arguments() {
        assert_eq!(fill("%1 of %2", &["3".into(), "8".into()]), "3 of 8");
        // Arguments that contain a placeholder themselves stay as they are.
        assert_eq!(fill("%1 / %2", &["%2".into(), "x".into()]), "%2 / x");
        assert_eq!(raw("Fertig").args.len(), 1);
        assert!(Message::default().is_empty());
        assert!(!raw("x").is_empty());
    }
}
