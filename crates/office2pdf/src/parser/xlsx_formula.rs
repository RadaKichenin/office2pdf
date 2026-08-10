//! A small spreadsheet-formula evaluator, enough for `cfRule type="expression"`.
//!
//! Conditional-format expressions are the one place a workbook's *formulas*
//! decide what is drawn: a Gantt template paints its whole bar area with them
//! and ships no chart part at all (issue #852). Evaluating them needs three
//! things beyond a calculator — relative references that rebase per cell,
//! defined names that expand to more formula text, and Excel's coercion of
//! booleans to 1 and 0.
//!
//! This is deliberately not a general engine. It covers the operators and the
//! handful of functions those rules use, and answers `None` for anything else
//! so an unknown formula draws nothing rather than something wrong.

use std::collections::HashMap;

/// How many name expansions deep to go before giving up, so a name that
/// references itself cannot loop.
const MAX_NAME_DEPTH: usize = 8;

/// A value a formula can carry. Text exists only to be compared; every
/// arithmetic path coerces to a number, and a boolean coerces to 1 or 0 the
/// way Excel's own `*`/`+` on comparisons does.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    Number(f64),
    Bool(bool),
    Text(String),
    /// An empty cell. Excel reads it as 0 in arithmetic and as `""` in a text
    /// comparison, which is why it is not simply `Number(0.0)`.
    Blank,
}

impl Value {
    fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
            Self::Blank => Some(0.0),
            // Excel does coerce a numeric string in arithmetic.
            Self::Text(text) => text.trim().parse::<f64>().ok(),
        }
    }

    /// Excel's truthiness: a non-zero number, or `TRUE`.
    pub(super) fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(flag) => *flag,
            Self::Number(number) => *number != 0.0,
            Self::Blank => false,
            Self::Text(text) => text.eq_ignore_ascii_case("true"),
        }
    }
}

/// A cell reference with its `$` anchoring preserved, so the relative parts
/// can be rebased onto another cell.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Reference {
    column: u32,
    row: u32,
    column_absolute: bool,
    row_absolute: bool,
}

impl Reference {
    /// This reference as seen from `(column, row)`, given the base cell the
    /// formula's relative parts were written against.
    ///
    /// A defined name's relative references are stored against the cell that
    /// was active when the name was defined; for the templates this serves
    /// that is `A1`, so a relative `A` means "the using cell's own column"
    /// and a relative `1` means "its own row".
    fn rebase(self, column: u32, row: u32, base: (u32, u32)) -> Option<(u32, u32)> {
        let resolved_column: i64 = if self.column_absolute {
            i64::from(self.column)
        } else {
            i64::from(self.column) + i64::from(column) - i64::from(base.0)
        };
        let resolved_row: i64 = if self.row_absolute {
            i64::from(self.row)
        } else {
            i64::from(self.row) + i64::from(row) - i64::from(base.1)
        };
        (resolved_column >= 1 && resolved_row >= 1)
            .then_some((resolved_column as u32, resolved_row as u32))
    }
}

/// What a formula needs from the sheet around it.
pub(super) struct EvalContext<'a> {
    /// The cell the expression is being evaluated for, 1-indexed.
    pub(super) cell: (u32, u32),
    /// The cell a relative reference is written against. `A1` for a defined
    /// name; the top-left of the `sqref` for a rule's own formula.
    pub(super) base: (u32, u32),
    /// Workbook and sheet defined names, keyed by upper-case name.
    pub(super) names: &'a HashMap<String, String>,
    /// Reads one cell's value, 1-indexed `(column, row)`.
    pub(super) value_at: &'a dyn Fn(u32, u32) -> Value,
}

/// Evaluate `formula` for the context's cell. `None` when it uses anything
/// this evaluator does not model, so the caller can leave the cell alone.
pub(super) fn evaluate(formula: &str, ctx: &EvalContext<'_>) -> Option<Value> {
    let tokens: Vec<Token> = tokenize(formula.trim().trim_start_matches('='))?;
    let mut parser = Parser {
        tokens: &tokens,
        position: 0,
        ctx,
        depth: 0,
    };
    let value: Value = parser.parse_comparison()?;
    parser.at_end().then_some(value)
}

// ── Tokens ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    /// A bare word: a function name, a defined name, or a cell reference,
    /// with any `Sheet!` qualifier already stripped.
    Word(String),
    Operator(String),
    Comma,
    Open,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut index: usize = 0;
    while index < chars.len() {
        let ch: char = chars[index];
        match ch {
            ' ' | '\t' | '\n' | '\r' => index += 1,
            '(' => {
                tokens.push(Token::Open);
                index += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                index += 1;
            }
            ',' | ';' => {
                tokens.push(Token::Comma);
                index += 1;
            }
            '+' | '-' | '*' | '/' | '^' | '&' | '=' => {
                tokens.push(Token::Operator(ch.to_string()));
                index += 1;
            }
            '<' | '>' => {
                let next: Option<char> = chars.get(index + 1).copied();
                let (operator, width) = match (ch, next) {
                    ('<', Some('=')) => ("<=", 2),
                    ('<', Some('>')) => ("<>", 2),
                    ('>', Some('=')) => (">=", 2),
                    _ => (if ch == '<' { "<" } else { ">" }, 1),
                };
                tokens.push(Token::Operator(operator.to_string()));
                index += width;
            }
            '"' => {
                // A quoted string; doubled quotes escape one.
                let mut text_value = String::new();
                index += 1;
                loop {
                    match chars.get(index) {
                        None => return None,
                        Some('"') if chars.get(index + 1) == Some(&'"') => {
                            text_value.push('"');
                            index += 2;
                        }
                        Some('"') => {
                            index += 1;
                            break;
                        }
                        Some(other) => {
                            text_value.push(*other);
                            index += 1;
                        }
                    }
                }
                tokens.push(Token::Word(format!("\"{text_value}")));
            }
            '0'..='9' | '.' => {
                let start: usize = index;
                while chars
                    .get(index)
                    .is_some_and(|c| c.is_ascii_digit() || *c == '.')
                {
                    index += 1;
                }
                let number: f64 = chars[start..index]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()?;
                tokens.push(Token::Number(number));
            }
            _ if ch.is_alphanumeric() || ch == '$' || ch == '_' || ch == '.' || ch == '\'' => {
                let start: usize = index;
                while chars.get(index).is_some_and(|c| {
                    c.is_alphanumeric()
                        || *c == '$'
                        || *c == '_'
                        || *c == '.'
                        || *c == '!'
                        || *c == '\''
                        || *c == ' ' && false
                }) {
                    index += 1;
                }
                let word: String = chars[start..index].iter().collect();
                // `Sheet!A$4` and `'My Sheet'!A$4` both reduce to the
                // reference; this evaluator reads one sheet at a time.
                let bare: &str = word.rsplit('!').next().unwrap_or(&word);
                tokens.push(Token::Word(bare.to_string()));
            }
            _ => return None,
        }
    }
    Some(tokens)
}

// ── Parser / evaluator ─────────────────────────────────────────────────

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    ctx: &'a EvalContext<'a>,
    depth: usize,
}

impl Parser<'_> {
    fn at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn take_operator(&mut self, wanted: &[&str]) -> Option<String> {
        match self.peek() {
            Some(Token::Operator(operator)) if wanted.contains(&operator.as_str()) => {
                let operator = operator.clone();
                self.position += 1;
                Some(operator)
            }
            _ => None,
        }
    }

    /// `=`, `<>`, `<`, `<=`, `>`, `>=` — lowest precedence, left to right.
    fn parse_comparison(&mut self) -> Option<Value> {
        let mut left: Value = self.parse_sum()?;
        while let Some(operator) = self.take_operator(&["=", "<>", "<", "<=", ">", ">="]) {
            let right: Value = self.parse_sum()?;
            left = Value::Bool(compare(&left, &right, &operator)?);
        }
        Some(left)
    }

    fn parse_sum(&mut self) -> Option<Value> {
        let mut left: Value = self.parse_product()?;
        while let Some(operator) = self.take_operator(&["+", "-", "&"]) {
            let right: Value = self.parse_product()?;
            left = match operator.as_str() {
                "&" => Value::Text(format!("{}{}", display(&left), display(&right))),
                "+" => Value::Number(left.as_number()? + right.as_number()?),
                _ => Value::Number(left.as_number()? - right.as_number()?),
            };
        }
        Some(left)
    }

    fn parse_product(&mut self) -> Option<Value> {
        let mut left: Value = self.parse_unary()?;
        while let Some(operator) = self.take_operator(&["*", "/"]) {
            let right: Value = self.parse_unary()?;
            let divisor: f64 = right.as_number()?;
            left = Value::Number(if operator == "*" {
                left.as_number()? * divisor
            } else {
                if divisor == 0.0 {
                    return None; // #DIV/0!
                }
                left.as_number()? / divisor
            });
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Value> {
        if let Some(operator) = self.take_operator(&["-", "+"]) {
            let value: Value = self.parse_unary()?;
            let number: f64 = value.as_number()?;
            return Some(Value::Number(if operator == "-" {
                -number
            } else {
                number
            }));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Option<Value> {
        match self.tokens.get(self.position)?.clone() {
            Token::Number(number) => {
                self.position += 1;
                Some(Value::Number(number))
            }
            Token::Open => {
                self.position += 1;
                let value: Value = self.parse_comparison()?;
                matches!(self.peek(), Some(Token::Close)).then(|| self.position += 1)?;
                Some(value)
            }
            Token::Word(word) => {
                self.position += 1;
                if let Some(literal) = word.strip_prefix('"') {
                    return Some(Value::Text(literal.to_string()));
                }
                if matches!(self.peek(), Some(Token::Open)) {
                    self.position += 1;
                    let arguments: Vec<Value> = self.parse_arguments()?;
                    return call(&word, &arguments, self.ctx);
                }
                self.resolve_word(&word)
            }
            _ => None,
        }
    }

    fn parse_arguments(&mut self) -> Option<Vec<Value>> {
        let mut arguments: Vec<Value> = Vec::new();
        if matches!(self.peek(), Some(Token::Close)) {
            self.position += 1;
            return Some(arguments);
        }
        loop {
            arguments.push(self.parse_comparison()?);
            match self.peek() {
                Some(Token::Comma) => self.position += 1,
                Some(Token::Close) => {
                    self.position += 1;
                    return Some(arguments);
                }
                _ => return None,
            }
        }
    }

    /// A bare word is a boolean literal, a cell reference, or a defined name.
    fn resolve_word(&mut self, word: &str) -> Option<Value> {
        if word.eq_ignore_ascii_case("TRUE") {
            return Some(Value::Bool(true));
        }
        if word.eq_ignore_ascii_case("FALSE") {
            return Some(Value::Bool(false));
        }
        if let Some(reference) = parse_reference(word) {
            let (column, row) =
                reference.rebase(self.ctx.cell.0, self.ctx.cell.1, self.ctx.base)?;
            return Some((self.ctx.value_at)(column, row));
        }
        if self.depth >= MAX_NAME_DEPTH {
            return None;
        }
        let definition: &String = self.ctx.names.get(&word.to_ascii_uppercase())?;
        // A name's own relative references are written against A1, whatever
        // base the formula using it has.
        let inner_ctx = EvalContext {
            cell: self.ctx.cell,
            base: (1, 1),
            names: self.ctx.names,
            value_at: self.ctx.value_at,
        };
        let tokens: Vec<Token> = tokenize(definition.trim().trim_start_matches('='))?;
        let mut parser = Parser {
            tokens: &tokens,
            position: 0,
            ctx: &inner_ctx,
            depth: self.depth + 1,
        };
        let value: Value = parser.parse_comparison()?;
        parser.at_end().then_some(value)
    }
}

fn display(value: &Value) -> String {
    match value {
        Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Bool(flag) => if *flag { "TRUE" } else { "FALSE" }.to_string(),
        Value::Text(text) => text.clone(),
        Value::Blank => String::new(),
    }
}

fn compare(left: &Value, right: &Value, operator: &str) -> Option<bool> {
    // Two texts compare as text; anything else compares as a number, which is
    // what makes `A$4=period_selected` work when one side is a blank cell.
    let ordering: std::cmp::Ordering = match (left, right) {
        (Value::Text(a), Value::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        _ => left.as_number()?.partial_cmp(&right.as_number()?)?,
    };
    Some(match operator {
        "=" => ordering.is_eq(),
        "<>" => !ordering.is_eq(),
        "<" => ordering.is_lt(),
        "<=" => ordering.is_le(),
        ">" => ordering.is_gt(),
        _ => ordering.is_ge(),
    })
}

fn call(name: &str, arguments: &[Value], ctx: &EvalContext<'_>) -> Option<Value> {
    let numbers = || -> Option<Vec<f64>> { arguments.iter().map(Value::as_number).collect() };
    match name.to_ascii_uppercase().as_str() {
        "COLUMN" => Some(Value::Number(f64::from(ctx.cell.0))),
        "ROW" => Some(Value::Number(f64::from(ctx.cell.1))),
        "MOD" => {
            let values: Vec<f64> = numbers()?;
            let [dividend, divisor] = values[..] else {
                return None;
            };
            (divisor != 0.0)
                .then(|| Value::Number(dividend - divisor * (dividend / divisor).floor()))
        }
        "INT" => Some(Value::Number(numbers()?.first()?.floor())),
        "ABS" => Some(Value::Number(numbers()?.first()?.abs())),
        "MEDIAN" => {
            let mut values: Vec<f64> = numbers()?;
            if values.is_empty() {
                return None;
            }
            values.sort_by(f64::total_cmp);
            let middle: usize = values.len() / 2;
            Some(Value::Number(if values.len().is_multiple_of(2) {
                (values[middle - 1] + values[middle]) / 2.0
            } else {
                values[middle]
            }))
        }
        "MIN" => Some(Value::Number(
            numbers()?.into_iter().fold(f64::INFINITY, f64::min),
        )),
        "MAX" => Some(Value::Number(
            numbers()?.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )),
        "SUM" => Some(Value::Number(numbers()?.into_iter().sum())),
        "AND" => Some(Value::Bool(arguments.iter().all(Value::is_truthy))),
        "OR" => Some(Value::Bool(arguments.iter().any(Value::is_truthy))),
        "NOT" => Some(Value::Bool(!arguments.first()?.is_truthy())),
        "IF" => {
            let condition: bool = arguments.first()?.is_truthy();
            let branch: Option<&Value> = if condition {
                arguments.get(1)
            } else {
                arguments.get(2)
            };
            Some(branch.cloned().unwrap_or(Value::Bool(condition)))
        }
        _ => None,
    }
}

/// Parse `A1`, `$C1`, `A$4`, `$H$2` into a reference. `None` for a word that
/// is not one, which is how a defined name is told apart from a reference.
fn parse_reference(word: &str) -> Option<Reference> {
    let bytes: &[u8] = word.as_bytes();
    let mut index: usize = 0;
    let column_absolute: bool = bytes.first() == Some(&b'$');
    if column_absolute {
        index += 1;
    }
    let letters_start: usize = index;
    while bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
        index += 1;
    }
    if index == letters_start || index - letters_start > 3 {
        return None;
    }
    let row_absolute: bool = bytes.get(index) == Some(&b'$');
    if row_absolute {
        index += 1;
    }
    let digits_start: usize = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    if index != bytes.len() || index == digits_start {
        return None;
    }
    let mut column: u32 = 0;
    for letter in &bytes[letters_start..digits_start.saturating_sub(usize::from(row_absolute))] {
        if !letter.is_ascii_alphabetic() {
            return None;
        }
        column = column * 26 + u32::from(letter.to_ascii_uppercase() - b'A') + 1;
    }
    let row: u32 = word[digits_start..].parse().ok()?;
    (column > 0 && row > 0).then_some(Reference {
        column,
        row,
        column_absolute,
        row_absolute,
    })
}

#[cfg(test)]
#[path = "xlsx_formula_tests.rs"]
mod tests;
