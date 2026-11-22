//! Torque equations: the parser into a small tree and its evaluation on the
//! bytes of an answer.

use std::sync::Arc;

use caruspex_i18n::l;

use super::{is_space, parse_number, trim};

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
