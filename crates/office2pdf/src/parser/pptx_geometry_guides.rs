//! DrawingML shape guide evaluation.
//!
//! A `<a:custGeom>` written by PowerPoint states its vertices as plain
//! numbers, but one round-tripped through LibreOffice states them as guide
//! names — `<a:pt x="f38" y="f37"/>` — and puts the arithmetic in an
//! `<a:gdLst>` of `<a:gd name= fmla=>` entries evaluated in document order
//! (issue #1205). Dropping the names left every such geometry empty, so the
//! caller's rectangle fallback stood in for a shape the deck had fully
//! described.
//!
//! The grammar is ECMA-376 Part 1 §20.1.9.11: a formula is an operator
//! followed by its operands, each operand a literal, a built-in variable, or
//! an earlier guide's name.

use std::collections::HashMap;

/// A full turn in the 60000ths of a degree DrawingML measures angles in.
const ANGLE_UNITS_PER_TURN: f64 = 21_600_000.0;

/// The shape box a guide measures itself against.
///
/// Units are whatever the geometry's own coordinates use — EMU for a path
/// that declares no coordinate space of its own — because the caller
/// normalizes the result against the same box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeExtent {
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl ShapeExtent {
    pub(crate) fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Whether the box has an area, and so can normalize a coordinate.
    pub(crate) fn is_usable(&self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }
}

/// The guides a geometry has defined so far, and the box they measure against.
#[derive(Debug, Clone)]
pub(crate) struct GuideList {
    extent: ShapeExtent,
    values: HashMap<String, f64>,
}

impl GuideList {
    pub(crate) fn new(extent: ShapeExtent) -> Self {
        Self {
            extent,
            values: HashMap::new(),
        }
    }

    /// Evaluate one `<a:gd name= fmla=>` and bind its result.
    ///
    /// Guides are evaluated as they are read: a formula may name any guide
    /// defined before it, which is how a list of a hundred entries builds a
    /// corner radius out of the adjust value at its head.
    pub(crate) fn define(&mut self, name: &str, formula: &str) {
        if let Some(value) = self.evaluate(formula) {
            self.values.insert(name.to_string(), value);
        }
    }

    /// Resolve an attribute that may be a literal, a built-in variable, or a
    /// guide name — what `<a:pt x=>` and `<a:arcTo stAng=>` each hold.
    pub(crate) fn resolve(&self, token: &str) -> Option<f64> {
        let token: &str = token.trim();
        if let Ok(literal) = token.parse::<f64>() {
            return literal.is_finite().then_some(literal);
        }
        if let Some(value) = self.values.get(token) {
            return Some(*value);
        }
        self.builtin(token)
    }

    /// Evaluate `<operator> <operand>...`, or a bare literal.
    fn evaluate(&self, formula: &str) -> Option<f64> {
        let mut parts = formula.split_whitespace();
        let operator: &str = parts.next()?;
        let operands: Vec<f64> = parts
            .map(|token| self.resolve(token))
            .collect::<Option<_>>()?;
        let value: f64 = apply(operator, &operands)?;
        value.is_finite().then_some(value)
    }

    /// The variables every geometry may use without declaring them.
    fn builtin(&self, name: &str) -> Option<f64> {
        let width: f64 = self.extent.width;
        let height: f64 = self.extent.height;
        let short_side: f64 = width.min(height);
        match name {
            "l" | "t" => Some(0.0),
            "r" | "w" => Some(width),
            "b" | "h" => Some(height),
            "hc" => Some(width / 2.0),
            "vc" => Some(height / 2.0),
            "ss" => Some(short_side),
            "ls" => Some(width.max(height)),
            _ => divided_builtin(name, width, height, short_side).or_else(|| angle_constant(name)),
        }
    }
}

/// `wd2`, `hd4`, `ssd16` — a side divided by the number that follows it. The
/// spec enumerates a fixed set; parsing the divisor covers all of them and
/// any the list forgot.
fn divided_builtin(name: &str, width: f64, height: f64, short_side: f64) -> Option<f64> {
    let (side, rest): (f64, &str) = [("ssd", short_side), ("wd", width), ("hd", height)]
        .into_iter()
        .find_map(|(prefix, side)| name.strip_prefix(prefix).map(|rest| (side, rest)))?;
    let divisor: f64 = rest.parse::<f64>().ok()?;
    (divisor != 0.0).then(|| side / divisor)
}

/// `cd2`, `cd4`, `3cd4`, `7cd8` — that many of a circle divided that many
/// ways, in 60000ths of a degree.
fn angle_constant(name: &str) -> Option<f64> {
    let (numerator, rest): (f64, &str) = match name.find("cd") {
        Some(0) => (1.0, name),
        Some(split) => (name[..split].parse::<f64>().ok()?, &name[split..]),
        None => return None,
    };
    let divisor: f64 = rest.strip_prefix("cd")?.parse::<f64>().ok()?;
    (divisor != 0.0).then(|| numerator * ANGLE_UNITS_PER_TURN / divisor)
}

/// Apply one formula operator to its operands, or `None` when the operator is
/// unknown or the wrong number of operands arrived.
fn apply(operator: &str, operands: &[f64]) -> Option<f64> {
    match (operator, operands) {
        ("val", [x]) => Some(*x),
        ("*/", [x, y, z]) => Some(x * y / z),
        ("+-", [x, y, z]) => Some(x + y - z),
        ("+/", [x, y, z]) => Some((x + y) / z),
        // "If x > 0, then y, else z."
        ("?:", [x, y, z]) => Some(if *x > 0.0 { *y } else { *z }),
        ("abs", [x]) => Some(x.abs()),
        ("min", [x, y]) => Some(x.min(*y)),
        ("max", [x, y]) => Some(x.max(*y)),
        // "If y < x, then x; else if y > z, then z; else y."
        ("pin", [x, y, z]) => Some(if y < x {
            *x
        } else if y > z {
            *z
        } else {
            *y
        }),
        ("sqrt", [x]) => Some(x.sqrt()),
        // The 3-D distance, which the spec spells `mod` for "modulus".
        ("mod", [x, y, z]) => Some((x * x + y * y + z * z).sqrt()),
        ("sin", [x, y]) => Some(x * to_radians(*y).sin()),
        ("cos", [x, y]) => Some(x * to_radians(*y).cos()),
        ("tan", [x, y]) => Some(x * to_radians(*y).tan()),
        ("at2", [x, y]) => Some(to_angle_units(y.atan2(*x))),
        ("cat2", [x, y, z]) => Some(x * z.atan2(*y).cos()),
        ("sat2", [x, y, z]) => Some(x * z.atan2(*y).sin()),
        _ => None,
    }
}

/// DrawingML states an angle in 60000ths of a degree.
pub(crate) fn to_radians(angle_units: f64) -> f64 {
    (angle_units / ANGLE_UNITS_PER_TURN) * std::f64::consts::TAU
}

fn to_angle_units(radians: f64) -> f64 {
    (radians / std::f64::consts::TAU) * ANGLE_UNITS_PER_TURN
}

#[cfg(test)]
#[path = "pptx_geometry_guides_tests.rs"]
mod tests;
