//! Custom values: anything a car offers beyond the standard OBD PIDs.
//!
//! Makers keep the interesting values behind their own identifiers: service
//! 22 (UDS, read data by identifier) or service 21 (KWP2000, read data by
//! local identifier), often on a control unit other than the engine. The
//! battery management of an electric car is the typical case: state of
//! health, cell voltages and temperatures are nowhere in standard OBD.
//!
//! A custom value says whom to ask (header), what to ask (request) and how to
//! turn the answer into a number (equation). The format is the one of Torque
//! Pro, which the community uses to share such values, so existing lists can
//! be imported directly:
//!
//! ```text
//! Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header
//! State of health,SOH,220105,((Z<8)+AA)/10,0,100,%,7E4
//! ```
//!
//! The equation sees the data bytes of the positive answer as A, B, C, ...
//! Z, AA, AB, ... where A is the first byte after the echoed service and
//! identifier (for 220105 the answer starts 62 01 05, so A is the byte after
//! that).
//!
//! Supported in equations (case does not matter):
//!
//! - numbers: 12, 0.5, .25
//! - bytes: A .. Z, AA .. ZZ
//! - arithmetic: + - * / and parentheses, unary minus
//! - shifts: `x<n` and `x>n` shift the integer part of x by n bits. Not in
//!   the official Torque documentation, but used by the widely shared EV
//!   lists and accepted by Torque. They bind weaker than + and -, as in C:
//!   (A<8)+B, not A<8+B.
//! - bits: {A:3} is bit 3 of A (0 or 1); BIT(x:n) does the same for any
//!   expression
//! - functions: SIGNED(x) or SIGNED8(x): x as 8-bit two's complement;
//!   SIGNED16(x), SIGNED24(x), SIGNED32(x): the same with more bits;
//!   INT16(a:b), INT24(a:b:c), INT32(a:b:c:d): bytes joined, most
//!   significant first, unsigned; MIN(a:b), MAX(a:b), ABS(x), INT(x) (drops
//!   the fraction)
//!
//! Not supported, and reported as an error: val{...} (references to other
//! values), the averaging and delay functions, LOOKUP, FLOAT32.
//!
//! Our own lists may carry a ninth column "Response" with the id the control
//! unit answers on. It matters where it is not header + 8, for example on the
//! Renault-Nissan battery controller (79B answers on 7BB).
//!
//! Values the car broadcasts by itself have the request "CAN132" (CAN plus an
//! 11-bit id) and a DBC signal as equation, "DBC:start|len@order+(factor,offset)"
//! with an optional multiplexer " mN@s|l". They are listened for, not asked.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use caruspex_data::pids::PidCategory;
use caruspex_i18n::l;
use caruspex_link::Elm327;
use caruspex_net::can_signals::{
    CanSignal, decode_can_signal, parse_can_signal, parse_monitor_request,
};
use caruspex_protocols::{LiveValue, Uds, ecu_display_name};

/// Keys of custom values in `LiveValue::key`. OBD keys are pid | (sub << 8)
/// and stay far below these ranges, so both can share one series map.
/// Values of the built-in packs.
pub const PACK_KEY_BASE: i32 = 0x10_0000;
/// Values imported from a file.
pub const USER_KEY_BASE: i32 = 0x20_0000;

pub const fn is_custom_key(key: i32) -> bool {
    key >= PACK_KEY_BASE
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomPid {
    pub name: String,
    pub short_name: String,
    /// Hex, normalized: "220101", "2101", "015C". Or "CAN132" for a value the
    /// car broadcasts by itself.
    pub request: String,
    /// Request id, e.g. "7E4"; empty = functional OBD addressing.
    pub header: String,
    /// Response id; empty = header + 8 (11 bit) or the swapped 29-bit address
    /// (18DA44F1 -> 18DAF144).
    pub response: String,
    pub equation: String,
    pub min_value: f64,
    pub max_value: f64,
    pub unit: String,
    /// Id of the built-in pack, empty for imported values.
    pub pack: String,
    /// Where the definition comes from.
    pub source: String,
    /// Series id, see [`PACK_KEY_BASE`].
    pub key: i32,
    pub category: PidCategory,
}

impl Default for CustomPid {
    fn default() -> Self {
        Self {
            name: String::new(),
            short_name: String::new(),
            request: String::new(),
            header: String::new(),
            response: String::new(),
            equation: String::new(),
            min_value: 0.0,
            max_value: 0.0,
            unit: String::new(),
            pack: String::new(),
            source: String::new(),
            key: 0,
            category: PidCategory::Custom,
        }
    }
}

// ---------------------------------------------------------------------------
// Small text helpers
// ---------------------------------------------------------------------------

fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\r', '\n'])
}

fn is_hex_text(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_hexdigit())
}

fn strip_hex_prefix(s: &str) -> &str {
    if s.len() >= 2 && s.as_bytes()[0] == b'0' && matches!(s.as_bytes()[1], b'x' | b'X') {
        &s[2..]
    } else {
        s
    }
}

/// C `isspace` in the C locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// A decimal number in the C locale: optional sign, digits, optional point
/// and fraction, optional exponent. Locale independent on purpose: the GUI
/// may run under a German one where the point is a comma. The syntax is
/// checked here; the value itself comes from the standard library, which
/// rounds correctly, so "8.354" is exactly the double other programs read
/// (the OBDb import relies on that).
fn parse_number(text: &str) -> Option<f64> {
    let t = trim(text);
    let s = t.as_bytes();
    let mut i = 0;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        i += 1;
    }
    let mut digits = false;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
        digits = true;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            digits = true;
        }
    }
    if !digits {
        return None;
    }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        i += 1;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            i += 1;
        }
        let mut exp_digits = false;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            exp_digits = true;
        }
        if !exp_digits {
            return None;
        }
    }
    if i != s.len() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The opposite, also independent of the locale: `%.10g`.
fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    // Ten significant digits, rounded correctly; the exponent decides between
    // fixed and scientific notation, as %g does.
    let scientific = format!("{value:.9e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let trim_zeros = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..10).contains(&exponent) {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim_zeros(mantissa), exponent.abs())
    } else {
        let decimals = (9 - exponent).max(0) as usize;
        trim_zeros(&format!("{value:.decimals$}"))
    }
}

fn line_text(line: usize) -> String {
    format!("{}{line}: ", l("Zeile ", "Line "))
}

// ---------------------------------------------------------------------------
// Equations
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Number,
    Byte,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    ShiftLeft,
    ShiftRight,
    Signed8,
    Signed16,
    Signed24,
    Signed32,
    Int16,
    Int24,
    Int32,
    Min,
    Max,
    Abs,
    Int,
    Bit,
}

struct FunctionSpec {
    name: &'static str,
    op: Op,
    arguments: usize,
}

const FUNCTIONS: &[FunctionSpec] = &[
    FunctionSpec {
        name: "SIGNED",
        op: Op::Signed8,
        arguments: 1,
    },
    FunctionSpec {
        name: "SIGNED8",
        op: Op::Signed8,
        arguments: 1,
    },
    FunctionSpec {
        name: "SIGNED16",
        op: Op::Signed16,
        arguments: 1,
    },
    FunctionSpec {
        name: "SIGNED24",
        op: Op::Signed24,
        arguments: 1,
    },
    FunctionSpec {
        name: "SIGNED32",
        op: Op::Signed32,
        arguments: 1,
    },
    FunctionSpec {
        name: "INT16",
        op: Op::Int16,
        arguments: 2,
    },
    FunctionSpec {
        name: "INT24",
        op: Op::Int24,
        arguments: 3,
    },
    FunctionSpec {
        name: "INT32",
        op: Op::Int32,
        arguments: 4,
    },
    FunctionSpec {
        name: "MIN",
        op: Op::Min,
        arguments: 2,
    },
    FunctionSpec {
        name: "MAX",
        op: Op::Max,
        arguments: 2,
    },
    FunctionSpec {
        name: "ABS",
        op: Op::Abs,
        arguments: 1,
    },
    FunctionSpec {
        name: "INT",
        op: Op::Int,
        arguments: 1,
    },
    FunctionSpec {
        name: "BIT",
        op: Op::Bit,
        arguments: 2,
    },
];

/// Deeper than any sensible equation, shallow enough for the stack.
const MAX_DEPTH: i32 = 64;

fn byte_name(index: usize) -> String {
    if index < 26 {
        return ((b'A' + index as u8) as char).to_string();
    }
    let index = index - 26;
    format!(
        "{}{}",
        (b'A' + (index / 26) as u8) as char,
        (b'A' + (index % 26) as u8) as char
    )
}

#[derive(Debug)]
struct Node {
    op: Op,
    /// Number
    number: f64,
    /// Byte
    index: usize,
    /// Children, by position in the node list.
    args: Vec<usize>,
}

impl Node {
    fn new(op: Op) -> Self {
        Self {
            op,
            number: 0.0,
            index: 0,
            args: Vec::new(),
        }
    }
}

/// The parsed form of an equation.
#[derive(Debug)]
struct Program {
    nodes: Vec<Node>,
    root: usize,
}

/// Recursive descent over the grammar
///
/// ```text
///   expr    := shift
///   shift   := sum (('<' | '>') sum)*
///   sum     := product (('+' | '-') product)*
///   product := unary (('*' | '/') unary)*
///   unary   := ('-' | '+') unary | primary
///   primary := number | byte | '(' expr ')' | '{' byte ':' integer '}'
///            | function '(' expr (':' expr)* ')'
/// ```
struct Parser<'a> {
    s: &'a [u8],
    nodes: Vec<Node>,
    pos: usize,
    bytes_needed: usize,
    error: String,
}

type Parsed = Option<usize>;

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            s: text.as_bytes(),
            nodes: Vec::new(),
            pos: 0,
            bytes_needed: 0,
            error: String::new(),
        }
    }

    fn parse(mut self) -> Result<(Program, usize), String> {
        let root = self.expression(0);
        if root.is_some() {
            self.skip_space();
            if self.pos < self.s.len() {
                self.fail(l("Unerwartetes Zeichen", "Unexpected character"));
            }
        }
        match root {
            Some(root) if self.error.is_empty() => Ok((
                Program {
                    nodes: self.nodes,
                    root,
                },
                self.bytes_needed,
            )),
            _ => Err(self.error),
        }
    }

    fn skip_space(&mut self) {
        while self.pos < self.s.len() && is_space(self.s[self.pos]) {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_space();
        self.s.get(self.pos).copied().unwrap_or(0)
    }

    fn fail(&mut self, what: &str) -> Parsed {
        if self.error.is_empty() {
            self.error = format!(
                "{what}{}{}",
                l(" an Stelle ", " at position "),
                self.pos + 1
            );
        }
        None
    }

    fn add(&mut self, node: Node) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn binary(&mut self, op: Op, a: usize, b: usize) -> usize {
        let mut n = Node::new(op);
        n.args = vec![a, b];
        self.add(n)
    }

    fn expression(&mut self, depth: i32) -> Parsed {
        if depth > MAX_DEPTH {
            return self.fail(l(
                "Gleichung zu tief verschachtelt",
                "Equation nested too deeply",
            ));
        }
        let mut left = self.sum(depth)?;
        loop {
            let c = self.peek();
            if c != b'<' && c != b'>' {
                return Some(left);
            }
            self.pos += 1;
            let right = self.sum(depth)?;
            left = self.binary(
                if c == b'<' {
                    Op::ShiftLeft
                } else {
                    Op::ShiftRight
                },
                left,
                right,
            );
        }
    }

    fn sum(&mut self, depth: i32) -> Parsed {
        let mut left = self.product(depth)?;
        loop {
            let c = self.peek();
            if c != b'+' && c != b'-' {
                return Some(left);
            }
            self.pos += 1;
            let right = self.product(depth)?;
            left = self.binary(if c == b'+' { Op::Add } else { Op::Subtract }, left, right);
        }
    }

    fn product(&mut self, depth: i32) -> Parsed {
        let mut left = self.unary(depth)?;
        loop {
            let c = self.peek();
            if c != b'*' && c != b'/' {
                return Some(left);
            }
            self.pos += 1;
            let right = self.unary(depth)?;
            left = self.binary(
                if c == b'*' { Op::Multiply } else { Op::Divide },
                left,
                right,
            );
        }
    }

    fn unary(&mut self, depth: i32) -> Parsed {
        if depth > MAX_DEPTH {
            return self.fail(l(
                "Gleichung zu tief verschachtelt",
                "Equation nested too deeply",
            ));
        }
        let c = self.peek();
        if c == b'+' {
            self.pos += 1;
            return self.unary(depth + 1);
        }
        if c == b'-' {
            self.pos += 1;
            let inner = self.unary(depth + 1)?;
            let mut n = Node::new(Op::Negate);
            n.args = vec![inner];
            return Some(self.add(n));
        }
        self.primary(depth)
    }

    fn byte_node(&mut self, index: usize) -> usize {
        self.bytes_needed = self.bytes_needed.max(index + 1);
        let mut n = Node::new(Op::Byte);
        n.index = index;
        self.add(n)
    }

    fn identifier(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.s.len() && self.s[self.pos].is_ascii_alphanumeric() {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.pos]).into_owned()
    }

    fn primary(&mut self, depth: i32) -> Parsed {
        let c = self.peek();
        if c == 0 {
            return self.fail(l("Gleichung endet zu früh", "Equation ends too early"));
        }
        if c == b'(' {
            self.pos += 1;
            let inner = self.expression(depth + 1)?;
            if self.peek() != b')' {
                return self.fail(l(
                    "Schließende Klammer fehlt",
                    "Missing closing parenthesis",
                ));
            }
            self.pos += 1;
            return Some(inner);
        }
        if c.is_ascii_digit() || c == b'.' {
            let start = self.pos;
            while self.pos < self.s.len()
                && (self.s[self.pos].is_ascii_digit() || self.s[self.pos] == b'.')
            {
                self.pos += 1;
            }
            let text = String::from_utf8_lossy(&self.s[start..self.pos]).into_owned();
            let Some(value) = parse_number(&text) else {
                self.pos = start;
                return self.fail(l("Ungültige Zahl", "Invalid number"));
            };
            let mut n = Node::new(Op::Number);
            n.number = value;
            return Some(self.add(n));
        }
        if c == b'{' {
            // {A:3}: bit 3 of byte A.
            self.pos += 1;
            self.skip_space();
            let name = self.identifier();
            let Some(index) = TorqueEquation::byte_index(&name) else {
                return self.fail(l(
                    "Byte wie A oder AB erwartet",
                    "Byte like A or AB expected",
                ));
            };
            if self.peek() != b':' {
                return self.fail(l("Doppelpunkt erwartet", "Colon expected"));
            }
            self.pos += 1;
            self.skip_space();
            let start = self.pos;
            while self.pos < self.s.len() && self.s[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if start == self.pos || self.pos - start > 2 {
                return self.fail(l("Bitnummer erwartet", "Bit number expected"));
            }
            let bit: i32 = String::from_utf8_lossy(&self.s[start..self.pos])
                .parse()
                .unwrap_or(0);
            if bit > 31 {
                return self.fail(l("Bitnummer zu groß", "Bit number too large"));
            }
            if self.peek() != b'}' {
                return self.fail(l("Schließende Klammer fehlt", "Missing closing brace"));
            }
            self.pos += 1;
            let mut number = Node::new(Op::Number);
            number.number = f64::from(bit);
            let byte = self.byte_node(index);
            let number = self.add(number);
            return Some(self.binary(Op::Bit, byte, number));
        }
        if c.is_ascii_alphabetic() {
            let start = self.pos;
            let name = self.identifier().to_ascii_uppercase();
            let next = self.peek();
            if next == b'{' {
                self.pos = start;
                return self.fail(l(
                    "Verweise auf andere Werte (val{...}) werden nicht unterstützt",
                    "References to other values (val{...}) are not supported",
                ));
            }
            if next != b'(' {
                let Some(index) = TorqueEquation::byte_index(&name) else {
                    self.pos = start;
                    return self.fail(l("Unbekannter Name", "Unknown name"));
                };
                return Some(self.byte_node(index));
            }
            let Some(spec) = FUNCTIONS.iter().rev().find(|f| f.name == name) else {
                self.pos = start;
                return self.fail(l("Unbekannte Funktion", "Unknown function"));
            };
            self.pos += 1; // '('
            let mut call = Node::new(spec.op);
            loop {
                let arg = self.expression(depth + 1)?;
                call.args.push(arg);
                let sep = self.peek();
                // Torque separates arguments with ':'; a comma is taken too.
                if sep == b':' || sep == b',' {
                    self.pos += 1;
                    continue;
                }
                if sep == b')' {
                    self.pos += 1;
                    break;
                }
                return self.fail(l(
                    "Schließende Klammer fehlt",
                    "Missing closing parenthesis",
                ));
            }
            if call.args.len() != spec.arguments {
                self.pos = start;
                return self.fail(l(
                    "Falsche Anzahl von Argumenten",
                    "Wrong number of arguments",
                ));
            }
            return Some(self.add(call));
        }
        self.fail(l("Unerwartetes Zeichen", "Unexpected character"))
    }
}

/// Whole number part as a 64-bit integer, if it fits comfortably.
fn to_integer(v: f64) -> Option<i64> {
    if !v.is_finite() || v.abs() >= 9.0e18 {
        return None;
    }
    Some(v.trunc() as i64)
}

struct Evaluator<'a> {
    program: &'a Program,
    data: &'a [u8],
}

type Evaluated = Result<f64, String>;

fn fail(de: &'static str, en: &'static str) -> String {
    l(de, en).to_string()
}

fn integer(v: f64) -> Result<i64, String> {
    to_integer(v).ok_or_else(|| {
        fail(
            "Zahl zu groß für eine Bitoperation.",
            "Number too large for a bit operation.",
        )
    })
}

fn signed_bits(v: f64, bits: u32) -> Evaluated {
    let n = integer(v)?;
    let mask = (1u64 << bits) - 1;
    let raw = (n as u64) & mask;
    let sign = 1u64 << (bits - 1);
    Ok(if raw >= sign {
        raw as f64 - 2.0_f64.powi(bits as i32)
    } else {
        raw as f64
    })
}

fn shift_count(v: f64) -> Result<i32, String> {
    let n = integer(v)?;
    if !(0..=62).contains(&n) {
        return Err(fail(
            "Verschiebung außerhalb von 0 bis 62 Bit.",
            "Shift outside 0 to 62 bits.",
        ));
    }
    Ok(n as i32)
}

impl Evaluator<'_> {
    fn join_bytes(&self, node: &Node) -> Evaluated {
        let mut total = 0.0;
        for &arg in &node.args {
            let b = self.eval(arg)?;
            let n = integer(b)?;
            total = total * 256.0 + ((n as u64) & 0xFF) as f64;
        }
        Ok(total)
    }

    fn eval(&self, index: usize) -> Evaluated {
        let node = &self.program.nodes[index];
        match node.op {
            Op::Number => return Ok(node.number),
            Op::Byte => {
                return match self.data.get(node.index) {
                    Some(&b) => Ok(f64::from(b)),
                    None => Err(format!(
                        "{}{}{}{}{}",
                        l("Antwort zu kurz: Byte ", "Answer too short: byte "),
                        byte_name(node.index),
                        l(" fehlt (", " is missing ("),
                        self.data.len(),
                        l(" Datenbytes erhalten).", " data bytes received).")
                    )),
                };
            }
            Op::Negate => return Ok(-self.eval(node.args[0])?),
            Op::Signed8 | Op::Signed16 | Op::Signed24 | Op::Signed32 => {
                let a = self.eval(node.args[0])?;
                let bits = match node.op {
                    Op::Signed8 => 8,
                    Op::Signed16 => 16,
                    Op::Signed24 => 24,
                    _ => 32,
                };
                return signed_bits(a, bits);
            }
            Op::Int16 | Op::Int24 | Op::Int32 => return self.join_bytes(node),
            Op::Abs => return Ok(self.eval(node.args[0])?.abs()),
            Op::Int => return Ok(self.eval(node.args[0])?.trunc()),
            _ => {}
        }
        // Everything else takes two arguments.
        let a = self.eval(node.args[0])?;
        let b = self.eval(node.args[1])?;
        match node.op {
            Op::Add => Ok(a + b),
            Op::Subtract => Ok(a - b),
            Op::Multiply => Ok(a * b),
            Op::Divide => {
                if b == 0.0 {
                    return Err(fail("Division durch null.", "Division by zero."));
                }
                Ok(a / b)
            }
            Op::ShiftLeft | Op::ShiftRight => {
                let n = integer(a)?;
                let count = shift_count(b)?;
                // Scaling by a power of two is exact for these magnitudes and
                // avoids the undefined behaviour of shifting negative numbers;
                // floor makes the right shift arithmetic, as on two's
                // complement integers.
                Ok(if node.op == Op::ShiftLeft {
                    n as f64 * 2.0_f64.powi(count)
                } else {
                    (n as f64 * 2.0_f64.powi(-count)).floor()
                })
            }
            Op::Min => Ok(a.min(b)),
            Op::Max => Ok(a.max(b)),
            Op::Bit => {
                let n = integer(a)?;
                let count = shift_count(b)?;
                Ok((((n as u64) >> count) & 1) as f64)
            }
            _ => Err(fail("Unbekannte Operation.", "Unknown operation.")),
        }
    }
}

/// A parsed equation. Parse once, evaluate for every answer.
#[derive(Clone, Debug, Default)]
pub struct TorqueEquation {
    program: Option<Arc<Program>>,
    bytes_needed: usize,
    text: String,
    error: String,
}

impl TorqueEquation {
    pub fn new(text: &str) -> Self {
        let mut eq = Self {
            text: text.to_string(),
            ..Self::default()
        };
        let trimmed = trim(text);
        if trimmed.is_empty() {
            eq.error = l("Leere Gleichung", "Empty equation").to_string();
            return eq;
        }
        match Parser::new(trimmed).parse() {
            Ok((program, bytes_needed)) => {
                eq.program = Some(Arc::new(program));
                eq.bytes_needed = bytes_needed;
            }
            Err(error) => eq.error = error,
        }
        eq
    }

    pub fn ok(&self) -> bool {
        self.program.is_some()
    }

    /// Why parsing failed, active language. Empty if `ok()`.
    pub fn error(&self) -> &str {
        &self.error
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Number of data bytes the equation reads at most (highest letter + 1).
    pub fn bytes_needed(&self) -> usize {
        self.bytes_needed
    }

    /// Evaluates against the data bytes (A = data[0]). `Err` with a message if
    /// a byte is missing, a division by zero happens or the result is not a
    /// finite number.
    pub fn evaluate(&self, data: &[u8]) -> Result<f64, String> {
        let Some(program) = &self.program else {
            return Err(self.error.clone());
        };
        let ev = Evaluator { program, data };
        let result = ev.eval(program.root)?;
        if !result.is_finite() {
            return Err(fail(
                "Ergebnis ist keine endliche Zahl.",
                "Result is not a finite number.",
            ));
        }
        Ok(result)
    }

    /// Index of a byte name: "A" -> 0, "Z" -> 25, "AA" -> 26. `None` if it is
    /// not one or two letters.
    pub fn byte_index(name: &str) -> Option<usize> {
        let n = name.to_ascii_uppercase();
        let b = n.as_bytes();
        let letter = |c: u8| c.is_ascii_uppercase();
        match b {
            [a] if letter(*a) => Some(usize::from(a - b'A')),
            [a, c] if letter(*a) && letter(*c) => {
                Some(26 + usize::from(a - b'A') * 26 + usize::from(c - b'A'))
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Torque CSV
// ---------------------------------------------------------------------------

/// The request as bytes: "0x220101" -> 22 01 01. `None` if it is not 1 to 7
/// whole hex bytes.
pub fn parse_request_hex(text: &str) -> Option<Vec<u8>> {
    let t = trim(text);
    let mut s = strip_hex_prefix(t).to_string();
    s.retain(|c| c != ' ');
    if !is_hex_text(&s) || !s.len().is_multiple_of(2) || s.len() < 2 || s.len() > 14 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// One CSV record with the line it started on.
struct Record {
    line: usize,
    cells: Vec<String>,
}

fn cell_text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Splits the text into records. Quoted fields may hold commas, doubled
/// quotes and line breaks. A line whose first non-blank character is '#'
/// outside a quoted field is a comment.
fn split_csv(text: &str) -> Vec<Record> {
    let t = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    // A UTF-8 byte order mark, as spreadsheet programs like to write.
    if t.starts_with(&[0xEF, 0xBB, 0xBF]) {
        i = 3;
    }
    while i < t.len() {
        // Start of a line: comment or blank?
        let mut j = i;
        while j < t.len() && (t[j] == b' ' || t[j] == b'\t') {
            j += 1;
        }
        if j < t.len() && t[j] == b'#' {
            while j < t.len() && t[j] != b'\n' {
                j += 1;
            }
            i = j + 1;
            line += 1;
            continue;
        }
        let mut record = Record {
            line,
            cells: Vec::new(),
        };
        let mut cell: Vec<u8> = Vec::new();
        let mut quoted = false;
        let mut done = false;
        while i < t.len() && !done {
            let c = t[i];
            if quoted {
                if c == b'"' {
                    if i + 1 < t.len() && t[i + 1] == b'"' {
                        cell.push(b'"');
                        i += 2;
                        continue;
                    }
                    quoted = false;
                } else {
                    if c == b'\n' {
                        line += 1;
                    }
                    cell.push(c);
                }
                i += 1;
                continue;
            }
            if c == b'"' {
                quoted = true;
            } else if c == b',' {
                record.cells.push(cell_text(std::mem::take(&mut cell)));
            } else if c == b'\n' {
                line += 1;
                done = true;
            } else if c != b'\r' {
                cell.push(c);
            }
            i += 1;
        }
        record.cells.push(cell_text(cell));
        if !record.cells.iter().all(|s| trim(s).is_empty()) {
            out.push(record);
        }
    }
    out
}

const COL_NAME: usize = 0;
const COL_SHORT: usize = 1;
const COL_REQUEST: usize = 2;
const COL_EQUATION: usize = 3;
const COL_MIN: usize = 4;
const COL_MAX: usize = 5;
const COL_UNIT: usize = 6;
const COL_HEADER: usize = 7;
const COL_RESPONSE: usize = 8;
const COLUMN_COUNT: usize = 9;

fn column_for(title: &str) -> Option<usize> {
    let t: String = trim(title)
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !matches!(c, ' ' | '_' | '-'))
        .collect();
    match t.as_str() {
        "name" => Some(COL_NAME),
        "shortname" => Some(COL_SHORT),
        "modeandpid" | "pid" => Some(COL_REQUEST),
        "equation" => Some(COL_EQUATION),
        "minvalue" | "min" => Some(COL_MIN),
        "maxvalue" | "max" => Some(COL_MAX),
        "units" | "unit" => Some(COL_UNIT),
        "header" => Some(COL_HEADER),
        "response" => Some(COL_RESPONSE),
        _ => None,
    }
}

/// "7E4" -> "7E4", "0x7e4" -> "7E4", "DA44F1" -> "18DA44F1". `None` if it is no
/// CAN id; the flag tells whether it is a 29-bit id.
fn normalize_id(text: &str) -> Option<(String, bool)> {
    let mut s = strip_hex_prefix(trim(text)).to_ascii_uppercase();
    if !is_hex_text(&s) {
        return None;
    }
    let bits29 = if s.len() <= 3 {
        if u64::from_str_radix(&s, 16).ok()? > 0x7FF {
            return None;
        }
        while s.len() < 3 {
            s.insert(0, '0');
        }
        false
    } else if s.len() == 6 || s.len() == 8 {
        if s.len() == 6 {
            s = format!("18{s}"); // the usual priority byte
        }
        if u64::from_str_radix(&s, 16).ok()? > 0x1FFF_FFFF {
            return None;
        }
        true
    } else {
        return None;
    };
    Some((s, bits29))
}

/// The id a control unit answers on when nothing else is said.
fn default_response(header: &str) -> String {
    let id = u64::from_str_radix(header, 16).unwrap_or(0);
    if header.len() > 3 {
        // 18DA<target><source>: the answer swaps target and source.
        let r = (id & 0xFFFF_0000) | ((id & 0xFF) << 8) | ((id >> 8) & 0xFF);
        format!("{r:08X}")
    } else {
        format!("{:03X}", (id + 8) & 0x7FF)
    }
}

fn csv_field(s: &str) -> String {
    let needs_quotes = s.contains([',', '"', '\r', '\n']) || s.starts_with('#');
    if !needs_quotes {
        return s.to_string();
    }
    let mut out = String::from("\"");
    for c in s.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Reads a Torque CSV. A header line is optional; if present, the columns are
/// found by name (Name, ShortName, ModeAndPID, Equation, Min Value, Max Value,
/// Units, Header, Response; anything else is ignored), otherwise they are
/// taken in that order. Lines starting with '#' are comments. Fields may be
/// quoted ("a, b" and "say ""hi"""). A line that does not make a valid value
/// is skipped and described in the returned errors ("Zeile 7: ..."), the rest
/// is still read. Keys are `key_base` + index of the value in the result
/// ([`USER_KEY_BASE`] for imports).
pub fn parse_torque_csv(text: &str, key_base: i32) -> (Vec<CustomPid>, Vec<String>) {
    let mut out: Vec<CustomPid> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let records = split_csv(text);

    // Column layout: by the header line if there is one, Torque's order otherwise.
    let mut map: [Option<usize>; COLUMN_COUNT] = std::array::from_fn(Some);
    let mut first = 0;
    if let Some(head_record) = records.first() {
        let head = &head_record.cells;
        let is_header = head
            .iter()
            .any(|title| matches!(column_for(title), Some(COL_REQUEST) | Some(COL_EQUATION)));
        if is_header {
            map = [None; COLUMN_COUNT];
            for (i, title) in head.iter().enumerate() {
                if let Some(col) = column_for(title)
                    && map[col].is_none()
                {
                    map[col] = Some(i);
                }
            }
            first = 1;
            if map[COL_NAME].is_none() || map[COL_REQUEST].is_none() || map[COL_EQUATION].is_none()
            {
                errors.push(format!(
                    "{}{}",
                    line_text(head_record.line),
                    l(
                        "Kopfzeile ohne die Spalten Name, ModeAndPID und Equation.",
                        "Header line without the columns Name, ModeAndPID and Equation."
                    )
                ));
                return (out, errors);
            }
        }
    }

    for record in &records[first..] {
        let cell = |col: usize| -> String {
            match map[col] {
                Some(i) if i < record.cells.len() => trim(&record.cells[i]).to_string(),
                _ => String::new(),
            }
        };
        let mut reject = |why: String| errors.push(format!("{}{why}", line_text(record.line)));

        let mut pid = CustomPid {
            name: cell(COL_NAME),
            short_name: cell(COL_SHORT),
            equation: cell(COL_EQUATION),
            unit: cell(COL_UNIT),
            ..CustomPid::default()
        };
        if pid.name.is_empty() {
            reject(l("Name fehlt.", "Name is missing.").to_string());
            continue;
        }
        let request = cell(COL_REQUEST);
        if let Some(can_id) = parse_monitor_request(&request) {
            // A value the car broadcasts by itself: "CAN132" with a DBC signal.
            if let Err(why) = parse_can_signal(&pid.equation) {
                reject(format!("{}{why}", l("Gleichung: ", "Equation: ")));
                continue;
            }
            pid.request = format!("CAN{can_id:03X}");
            let min_text = cell(COL_MIN);
            let max_text = cell(COL_MAX);
            if !read_range(&mut pid, &min_text, &max_text) {
                reject(
                    l(
                        "Min oder Max ist keine Zahl.",
                        "Min or max is not a number.",
                    )
                    .to_string(),
                );
                continue;
            }
            if pid.short_name.is_empty() {
                pid.short_name = pid.name.clone();
            }
            pid.key = key_base + out.len() as i32;
            out.push(pid);
            continue;
        }
        let Some(bytes) = parse_request_hex(&request) else {
            reject(if request.is_empty() {
                l("ModeAndPID fehlt.", "ModeAndPID is missing.").to_string()
            } else {
                format!(
                    "{}{request}",
                    l(
                        "ModeAndPID ist keine Anfrage in Hex: ",
                        "ModeAndPID is no hex request: "
                    )
                )
            });
            continue;
        };
        if bytes.len() < 2 {
            reject(
                l(
                    "ModeAndPID braucht Dienst und Kennung, z. B. 220101.",
                    "ModeAndPID needs service and identifier, e.g. 220101.",
                )
                .to_string(),
            );
            continue;
        }
        pid.request = bytes.iter().map(|b| format!("{b:02X}")).collect();

        let equation = TorqueEquation::new(&pid.equation);
        if !equation.ok() {
            reject(format!(
                "{}{}",
                l("Gleichung: ", "Equation: "),
                equation.error()
            ));
            continue;
        }

        let min_text = cell(COL_MIN);
        let max_text = cell(COL_MAX);
        if !read_range(&mut pid, &min_text, &max_text) {
            reject(
                l(
                    "Min oder Max ist keine Zahl.",
                    "Min or max is not a number.",
                )
                .to_string(),
            );
            continue;
        }

        // Header, optionally with the response id: "79B", "79B/7BB", "79B:7BB".
        let mut header = cell(COL_HEADER);
        let mut response = cell(COL_RESPONSE);
        if let Some(split) = header.find(['/', ':']) {
            if response.is_empty() {
                response = header[split + 1..].to_string();
            }
            header.truncate(split);
        }
        let mut header_wide = false;
        if !trim(&header).is_empty() {
            match normalize_id(&header) {
                Some((id, wide)) => {
                    pid.header = id;
                    header_wide = wide;
                }
                None => {
                    reject(format!(
                        "{}{header}",
                        l("Header ist keine CAN-Adresse: ", "Header is no CAN id: ")
                    ));
                    continue;
                }
            }
        }
        if !trim(&response).is_empty() {
            if pid.header.is_empty() {
                reject(l("Response ohne Header.", "Response without a header.").to_string());
                continue;
            }
            match normalize_id(&response) {
                Some((id, wide)) if wide == header_wide => pid.response = id,
                _ => {
                    reject(format!(
                        "{}{response}",
                        l(
                            "Response ist keine passende CAN-Adresse: ",
                            "Response is no matching CAN id: "
                        )
                    ));
                    continue;
                }
            }
        }
        if pid.short_name.is_empty() {
            pid.short_name = pid.name.clone();
        }
        pid.key = key_base + out.len() as i32;
        out.push(pid);
    }
    (out, errors)
}

/// Min and max columns; empty cells keep the default. False if one is not a
/// number.
fn read_range(pid: &mut CustomPid, min_text: &str, max_text: &str) -> bool {
    if !min_text.is_empty() {
        match parse_number(min_text) {
            Some(v) => pid.min_value = v,
            None => return false,
        }
    }
    if !max_text.is_empty() {
        match parse_number(max_text) {
            Some(v) => pid.max_value = v,
            None => return false,
        }
    }
    true
}

/// Writes values in the same format, with a header line. The Response column
/// is added only if a value has one.
pub fn write_torque_csv(pids: &[CustomPid]) -> String {
    let with_response = pids.iter().any(|p| !p.response.is_empty());
    let mut out =
        String::from("Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header");
    if with_response {
        out.push_str(",Response");
    }
    out.push('\n');
    for p in pids {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}",
            csv_field(&p.name),
            csv_field(&p.short_name),
            csv_field(&p.request),
            csv_field(&p.equation),
            format_number(p.min_value),
            format_number(p.max_value),
            csv_field(&p.unit),
            csv_field(&p.header)
        ));
        if with_response {
            out.push(',');
            out.push_str(&csv_field(&p.response));
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

const REQUEST_TIMEOUT_MS: u64 = 2000;

/// Broadcast values are listened for, not asked ("CAN132").
fn is_monitored(pid: &CustomPid) -> bool {
    parse_monitor_request(&pid.request).is_some()
}

/// Where to ask, as the adapter sees it. Empty for the default addressing.
fn address_key(pid: &CustomPid) -> String {
    if is_monitored(pid) {
        return "listen".to_string();
    }
    if pid.header.is_empty() {
        return String::new();
    }
    let response = if pid.response.is_empty() {
        default_response(&pid.header)
    } else {
        pid.response.clone()
    };
    format!("{}/{response}", pid.header)
}

/// One answer: the data bytes after the echo, or why there are none.
type Answer = Result<Vec<u8>, String>;

/// Result of one value in [`CustomPidReader::read_all`]. The value is filled
/// as far as the definition goes, also when reading failed.
#[derive(Clone, Debug, Default)]
pub struct CustomReading {
    pub ok: bool,
    pub value: LiveValue,
    pub error: String,
}

/// Reads custom values over and over, as the live view does. It remembers
/// which control unit the adapter is pointed at, so consecutive values of the
/// same header cost no extra AT commands, and it parses every equation only
/// once. Dropping it puts the adapter back on the default OBD addressing.
pub struct CustomPidReader<'a> {
    uds: Uds<'a>,
    addressed: bool,
    /// Address key the adapter points at.
    current: String,
    equations: HashMap<String, TorqueEquation>,
    signals: HashMap<String, CanSignal>,
}

impl<'a> CustomPidReader<'a> {
    pub fn new(elm: &'a mut Elm327) -> Self {
        Self {
            uds: Uds::new(elm),
            addressed: false,
            current: String::new(),
            equations: HashMap::new(),
            signals: HashMap::new(),
        }
    }

    /// Back to the default OBD addressing now (also done when dropped).
    pub fn restore_default(&mut self) {
        if self.addressed {
            self.uds.restore_default();
        }
        self.addressed = false;
        self.current.clear();
    }

    fn address(&mut self, pid: &CustomPid) -> Result<(), String> {
        let key = address_key(pid);
        if key.is_empty() {
            self.restore_default();
            return Ok(());
        }
        if self.addressed && key == self.current {
            return Ok(());
        }
        let invalid = || l("Ungültige Adresse.", "Invalid address.").to_string();
        let response = if pid.response.is_empty() {
            default_response(&pid.header)
        } else {
            pid.response.clone()
        };
        let def = caruspex_data::ecu_addresses::EcuDefinition {
            request: u32::from_str_radix(&pid.header, 16).map_err(|_| invalid())?,
            response: u32::from_str_radix(&response, 16).map_err(|_| invalid())?,
            extended29: pid.header.len() > 3,
            name: format!("ECU {}", pid.header),
            ..Default::default()
        };
        self.addressed = false;
        self.current.clear();
        self.uds.select(&def)?; // select puts the default back itself
        self.addressed = true;
        self.current = key;
        Ok(())
    }

    fn ask(&mut self, pid: &CustomPid) -> Answer {
        let request = match parse_request_hex(&pid.request) {
            Some(r) if r.len() >= 2 => r,
            _ => return Err(l("Ungültige Anfrage.", "Invalid request.").to_string()),
        };
        self.address(pid)?;
        let response = self.uds.request(&request, REQUEST_TIMEOUT_MS)?;
        // The positive answer repeats everything after the service byte
        // (the identifier), then come the data bytes.
        let echo = request.len() - 1;
        if response.len() < echo || response[..echo] != request[1..] {
            return Err(l(
                "Die Antwort passt nicht zur Anfrage.",
                "The answer does not match the request.",
            )
            .to_string());
        }
        Ok(response[echo..].to_vec())
    }

    /// The frames of one id, for all values that need it. Multiplexed values
    /// need frames with every multiplexer value, so it listens a little
    /// longer for those.
    fn listen(&mut self, can_id: u32, multiplexed: bool) -> Vec<Vec<u8>> {
        self.restore_default();
        self.uds.elm().capture_frames(
            can_id,
            if multiplexed { 900 } else { 400 },
            if multiplexed { 40 } else { 6 },
        )
    }

    fn base_value(pid: &CustomPid) -> LiveValue {
        LiveValue {
            key: pid.key,
            name: pid.name.clone(),
            unit: pid.unit.clone(),
            category: pid.category,
            display_min: pid.min_value,
            display_max: pid.max_value,
            ..LiveValue::default()
        }
    }

    fn decode_monitored(
        &mut self,
        pid: &CustomPid,
        frames: &[Vec<u8>],
        out: &mut LiveValue,
    ) -> Result<(), String> {
        *out = Self::base_value(pid);
        let signal = match self.signals.get(&pid.equation) {
            Some(s) => *s,
            None => {
                let s = parse_can_signal(&pid.equation)?;
                self.signals.insert(pid.equation.clone(), s);
                s
            }
        };
        if frames.is_empty() {
            return Err(format!(
                "{}{}{}",
                l(
                    "Diese Nachricht ist auf dem Bus nicht zu hören (",
                    "This message is not heard on the bus ("
                ),
                pid.request,
                l(
                    "). Steckt das Kabel am richtigen Bus, und ist das Auto wach?",
                    "). Is the cable on the right bus, and is the car awake?"
                )
            ));
        }
        // The newest frame that carries the value.
        for frame in frames.iter().rev() {
            if let Some(v) = decode_can_signal(&signal, frame) {
                out.value = v;
                return Ok(());
            }
        }
        Err(l(
            "Der Wert kam in den gehörten Nachrichten nicht vor.",
            "The value was not in the frames heard.",
        )
        .to_string())
    }

    fn equation(&mut self, text: &str) -> TorqueEquation {
        self.equations
            .entry(text.to_string())
            .or_insert_with(|| TorqueEquation::new(text))
            .clone()
    }

    fn decode(
        &mut self,
        pid: &CustomPid,
        answer: &Answer,
        out: &mut LiveValue,
    ) -> Result<(), String> {
        *out = Self::base_value(pid);
        if !pid.header.is_empty() {
            out.ecu_source = if pid.response.is_empty() {
                default_response(&pid.header)
            } else {
                pid.response.clone()
            };
            out.ecu = ecu_display_name(&out.ecu_source);
        }
        let data = answer.as_ref().map_err(Clone::clone)?;
        out.value = self.equation(&pid.equation).evaluate(data)?;
        Ok(())
    }

    /// One value, the simple way.
    pub fn read(&mut self, pid: &CustomPid) -> Result<LiveValue, String> {
        let mut out = LiveValue::default();
        if let Some(id) = parse_monitor_request(&pid.request) {
            let multiplexed = pid.equation.contains(" m") || pid.equation.contains(")m");
            let frames = self.listen(id, multiplexed);
            self.decode_monitored(pid, &frames, &mut out)?;
            return Ok(out);
        }
        let eq = self.equation(&pid.equation);
        if !eq.ok() {
            return Err(eq.error().to_string());
        }
        let answer = self.ask(pid);
        self.decode(pid, &answer, &mut out)?;
        Ok(out)
    }

    /// All values, in the order given. Values sharing a header and a request
    /// are served from one answer (the battery management of an EV puts some
    /// thirty values into one), and values of the same control unit are
    /// asked one after another to save re-addressing.
    pub fn read_all(&mut self, pids: &[CustomPid]) -> Vec<CustomReading> {
        let mut out: Vec<CustomReading> = vec![CustomReading::default(); pids.len()];

        // Group by control unit, in the order the units first appear; the
        // control unit the adapter already points at goes first.
        let mut order: Vec<String> = Vec::new();
        for pid in pids {
            let key = address_key(pid);
            if !order.contains(&key) {
                order.push(key);
            }
        }
        let current = if self.addressed {
            self.current.clone()
        } else {
            String::new()
        };
        if let Some(here) = order.iter().position(|k| *k == current) {
            order[..=here].rotate_right(1);
        }

        // Broadcast values: listen once per id for all values of that id.
        let mut heard: BTreeMap<u32, Vec<Vec<u8>>> = BTreeMap::new();
        for pid in pids {
            let Some(id) = parse_monitor_request(&pid.request) else {
                continue;
            };
            if heard.contains_key(&id) {
                continue;
            }
            let multiplexed = pids.iter().any(|other| {
                parse_monitor_request(&other.request) == Some(id)
                    && other
                        .equation
                        .as_bytes()
                        .get(4..)
                        .is_some_and(|rest| rest.contains(&b'm'))
            });
            let frames = self.listen(id, multiplexed);
            heard.insert(id, frames);
        }
        for (i, pid) in pids.iter().enumerate() {
            let Some(id) = parse_monitor_request(&pid.request) else {
                continue;
            };
            let frames = heard.get(&id).cloned().unwrap_or_default();
            let reading = &mut out[i];
            match self.decode_monitored(pid, &frames, &mut reading.value) {
                Ok(()) => reading.ok = true,
                Err(e) => reading.error = e,
            }
        }

        for unit in &order {
            if unit == "listen" {
                continue;
            }
            let mut answers: BTreeMap<String, Answer> = BTreeMap::new(); // by request
            for (i, pid) in pids.iter().enumerate() {
                if address_key(pid) != *unit {
                    continue;
                }
                let eq = self.equation(&pid.equation);
                if !eq.ok() {
                    let reading = &mut out[i];
                    reading.value.key = pid.key;
                    reading.value.name = pid.name.clone();
                    reading.error = eq.error().to_string();
                    continue;
                }
                if !answers.contains_key(&pid.request) {
                    let answer = self.ask(pid);
                    answers.insert(pid.request.clone(), answer);
                }
                let answer = &answers[&pid.request];
                let reading = &mut out[i];
                match self.decode(pid, answer, &mut reading.value) {
                    Ok(()) => reading.ok = true,
                    Err(e) => reading.error = e,
                }
            }
        }
        out
    }
}

impl Drop for CustomPidReader<'_> {
    fn drop(&mut self) {
        self.restore_default();
    }
}

/// One value read once, the simple way. Addresses the header (if any),
/// restores the default addressing afterwards. For more than one value use
/// [`CustomPidReader`], which is much faster.
pub fn read_custom(elm: &mut Elm327, pid: &CustomPid) -> Result<LiveValue, String> {
    let mut reader = CustomPidReader::new(elm);
    reader.read(pid)
}

#[cfg(test)]
mod tests {
    use super::{format_number, parse_number};

    #[test]
    fn numbers_are_written_like_printf_g_with_ten_digits() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(100.0), "100");
        assert_eq!(format_number(-0.25), "-0.25");
        assert_eq!(format_number(1234.5), "1234.5");
        assert_eq!(format_number(16777215.0), "16777215");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
        assert_eq!(format_number(1.0e10), "1e+10");
        assert_eq!(format_number(0.0001), "0.0001");
        assert_eq!(format_number(0.00001), "1e-05");
        assert_eq!(format_number(123456789012.0), "1.23456789e+11");
    }

    #[test]
    fn numbers_are_read_independent_of_the_locale() {
        assert_eq!(parse_number(" 1.5 "), Some(1.5));
        assert_eq!(parse_number("-230"), Some(-230.0));
        assert_eq!(parse_number("1e3"), Some(1000.0));
        assert_eq!(parse_number("1,5"), None);
        assert_eq!(parse_number("e3"), None);
        assert_eq!(parse_number("low"), None);
    }
}
