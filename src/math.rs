use std::{
    cell::RefCell,
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::layout_style::Units;
use cssparser::{ParseError, Parser, Token};

thread_local! {
    static VALUES: RefCell<HashMap<usize, Expression>> = RefCell::default();
    static FREE: RefCell<Vec<usize>> = RefCell::default();
}
static NEXT: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Default)]
pub(crate) struct Arena(RefCell<Vec<usize>>);
impl Drop for Arena {
    fn drop(&mut self) {
        VALUES.with(|values| {
            let mut values = values.borrow_mut();
            FREE.with(|free| {
                let mut free = free.borrow_mut();
                for key in self.0.get_mut() {
                    if values.remove(key).is_some() {
                        free.push(*key);
                    }
                }
            });
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Number,
    Length,
    Angle,
    Any,
}
#[derive(Clone, Debug, PartialEq)]
struct Expression {
    kind: Kind,
    value: Value,
}
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Constant(f32),
    Percentage(f32),
    Binary(char, Box<Expression>, Box<Expression>),
    Function(String, Vec<Expression>, Option<f32>),
    Clamp(Box<Expression>),
}

// Calc handles belong to one layout pass; retained computed values own their expressions.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedLength(Expression);

impl ComputedLength {
    pub(crate) fn nonnegative(self) -> Self {
        Self(
            Expression {
                kind: Kind::Length,
                value: Value::Clamp(Box::new(self.0)),
            }
            .normalized(),
        )
    }

    pub(crate) fn resolve(&self, basis: f32) -> f32 {
        finite_result(self.0.evaluate(basis))
    }

    pub(crate) fn length(value: f32) -> Self {
        Self(Expression::constant(finite_result(value), Kind::Length))
    }

    pub(crate) fn percentage(value: f32) -> Self {
        Self(
            Expression {
                kind: Kind::Length,
                value: Value::Percentage(value),
            }
            .normalized(),
        )
    }

    pub(crate) fn from_handle(handle: *const ()) -> Self {
        VALUES.with(|values| Self(values.borrow()[&(handle as usize)].clone().normalized()))
    }

    pub(crate) fn from_raw(value: taffy::CompactLength) -> Option<Self> {
        if value.is_plain_calc() {
            return Some(Self::from_handle(value.calc_value()));
        }
        match value.tag() {
            taffy::CompactLength::LENGTH_TAG => Some(Self::length(value.value())),
            taffy::CompactLength::PERCENT_TAG => Some(Self::percentage(value.value())),
            _ => None,
        }
    }

    pub(crate) fn interpolate(&self, to: &Self, progress: f32) -> Self {
        let weighted = |expression: &Expression, weight| Expression {
            kind: Kind::Length,
            value: Value::Binary(
                '*',
                Box::new(expression.clone()),
                Box::new(Expression::constant(weight, Kind::Number)),
            ),
        };
        Self(
            Expression {
                kind: Kind::Length,
                value: Value::Binary(
                    '+',
                    Box::new(weighted(&self.0, 1.0 - progress)),
                    Box::new(weighted(&to.0, progress)),
                ),
            }
            .normalized(),
        )
    }

    pub(crate) fn subtract(&self, other: &Self) -> Self {
        Self(
            Expression {
                kind: Kind::Length,
                value: Value::Binary('-', Box::new(self.0.clone()), Box::new(other.0.clone())),
            }
            .normalized(),
        )
    }

    pub(crate) fn encode(&self, arena: &Arena) -> taffy::LengthPercentage {
        match self.0.value {
            Value::Constant(value) if value.is_finite() => {
                taffy::LengthPercentage::length(finite_result(value))
            }
            Value::Percentage(value) if value.abs() <= 1.0 => {
                taffy::LengthPercentage::percent(value)
            }
            _ => taffy::LengthPercentage::calc(arena.store(self.0.clone())),
        }
    }

    pub(crate) fn decoration(&self, arena: &Arena) -> crate::box_decoration::LengthPercentage {
        match self.0.value {
            Value::Constant(value) if value.is_finite() => {
                crate::box_decoration::LengthPercentage::Length(finite_result(value))
            }
            Value::Percentage(value) if value.abs() <= 1.0 => {
                crate::box_decoration::LengthPercentage::Percentage(value)
            }
            _ => crate::box_decoration::LengthPercentage::Calc(arena.store(self.0.clone())),
        }
    }
}

impl Expression {
    fn affine(&self) -> Option<(f32, f32)> {
        match &self.value {
            Value::Constant(value) => Some((*value, 0.0)),
            Value::Percentage(value) => Some((0.0, *value)),
            Value::Binary(operator, first, second) => {
                let (a, b) = first.affine()?;
                let (c, d) = second.affine()?;
                match operator {
                    '+' => Some((a + c, b + d)),
                    '-' => Some((a - c, b - d)),
                    '*' if b == 0.0 => Some((a * c, a * d)),
                    '*' if d == 0.0 => Some((a * c, b * c)),
                    '/' if d == 0.0 && c != 0.0 => Some((a / c, b / c)),
                    _ => None,
                }
            }
            Value::Function(name, arguments, _) if name == "calc" => arguments[0].affine(),
            Value::Clamp(value) => value
                .affine()
                .filter(|(length, percentage)| *length >= 0.0 && *percentage >= 0.0),
            _ => None,
        }
    }

    fn normalized(self) -> Self {
        if !self.percentage() {
            return Self::constant(finite_result(self.evaluate(0.0)), self.kind);
        }
        if let Some((length, percentage)) = self.affine() {
            let value = if percentage == 0.0 {
                Value::Constant(length)
            } else if length == 0.0 {
                Value::Percentage(percentage)
            } else {
                Value::Binary(
                    '+',
                    Box::new(Self::constant(length, self.kind)),
                    Box::new(Self {
                        kind: self.kind,
                        value: Value::Percentage(percentage),
                    }),
                )
            };
            return Self {
                kind: self.kind,
                value,
            };
        }
        self
    }

    fn constant(value: f32, kind: Kind) -> Self {
        Self {
            kind,
            value: Value::Constant(value),
        }
    }
    fn evaluate(&self, basis: f32) -> f32 {
        match &self.value {
            Value::Constant(value) => *value,
            Value::Percentage(value) => basis * value,
            Value::Clamp(value) => value.evaluate(basis).max(0.0),
            Value::Binary(op, a, b) => {
                let a = a.evaluate(basis);
                let b = b.evaluate(basis);
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    _ => a / b,
                }
            }
            Value::Function(name, args, pixels_per_point) => {
                let values: Vec<_> = args.iter().map(|arg| arg.evaluate(basis)).collect();
                if values.iter().any(|value| value.is_nan()) {
                    return f32::NAN;
                }
                let a = values[0];
                match name.as_str() {
                    "calc" => a,
                    "min" => values.iter().copied().fold(f32::INFINITY, f32::min),
                    "max" => values.iter().copied().fold(f32::NEG_INFINITY, f32::max),
                    "clamp" => a.max(values[1].min(values[2])),
                    "abs" => a.abs(),
                    "sign" => {
                        if a == 0.0 {
                            a
                        } else {
                            a.signum()
                        }
                    }
                    "hypot" => values.iter().copied().fold(0.0, f32::hypot),
                    "pow" => a.powf(values[1]),
                    "sqrt" => a.sqrt(),
                    "log" => {
                        if values.len() == 1 {
                            a.ln()
                        } else {
                            a.log(values[1])
                        }
                    }
                    "exp" => a.exp(),
                    "sin" => a.sin(),
                    "cos" => a.cos(),
                    "tan" => a.tan(),
                    "asin" => a.asin(),
                    "acos" => a.acos(),
                    "atan" => a.atan(),
                    "atan2" => a.atan2(values[1]),
                    "mod" => modulo(a, values[1]),
                    "rem" => a % values[1],
                    "round-line-width" => {
                        let pixels_per_point =
                            pixels_per_point.expect("line-width rounding has a pixel scale");
                        if values.len() == 1 {
                            snap_line_width(a, pixels_per_point)
                        } else {
                            round_line_width(a, values[1], pixels_per_point)
                        }
                    }
                    name if name.starts_with("round") => {
                        let step = values.get(1).copied().unwrap_or(1.0);
                        round(a, step, name)
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    fn percentage(&self) -> bool {
        match &self.value {
            Value::Percentage(_) => true,
            Value::Binary(_, a, b) => a.percentage() || b.percentage(),
            Value::Function(_, a, _) => a.iter().any(Self::percentage),
            Value::Clamp(a) => a.percentage(),
            _ => false,
        }
    }
    fn percentage_only(&self) -> bool {
        match &self.value {
            Value::Percentage(_) => true,
            Value::Binary(operator, left, right) => match operator {
                '+' | '-' => left.percentage_only() && right.percentage_only(),
                '*' => {
                    left.percentage_only() && right.kind == Kind::Number
                        || right.percentage_only() && left.kind == Kind::Number
                }
                '/' => left.percentage_only() && right.kind == Kind::Number,
                _ => false,
            },
            Value::Function(_, arguments, _) => arguments.iter().all(Self::percentage_only),
            Value::Clamp(value) => value.percentage_only(),
            _ => false,
        }
    }
}

impl Arena {
    pub(crate) fn add_pixels_dimension(
        &self,
        base: taffy::Dimension,
        pixels: f32,
    ) -> taffy::Dimension {
        let raw = base.into_raw();
        match raw.tag() {
            taffy::CompactLength::LENGTH_TAG => {
                taffy::Dimension::length((raw.value() + pixels).max(0.0))
            }
            taffy::CompactLength::PERCENT_TAG => {
                taffy::Dimension::calc(self.added_handle(raw, pixels))
            }
            _ if raw.is_plain_calc() => taffy::Dimension::calc(self.added_handle(raw, pixels)),
            _ => base,
        }
    }

    pub(crate) fn add_pixels_length_percentage_auto(
        &self,
        base: taffy::LengthPercentageAuto,
        pixels: f32,
    ) -> taffy::LengthPercentageAuto {
        let raw = base.into_raw();
        match raw.tag() {
            taffy::CompactLength::LENGTH_TAG => {
                taffy::LengthPercentageAuto::length((raw.value() + pixels).max(0.0))
            }
            taffy::CompactLength::PERCENT_TAG => {
                taffy::LengthPercentageAuto::calc(self.added_handle(raw, pixels))
            }
            _ if raw.is_plain_calc() => {
                taffy::LengthPercentageAuto::calc(self.added_handle(raw, pixels))
            }
            _ => base,
        }
    }

    pub(crate) fn add_pixels(
        &self,
        base: taffy::CompactLength,
        pixels: f32,
    ) -> taffy::LengthPercentage {
        match base.tag() {
            taffy::CompactLength::LENGTH_TAG => {
                taffy::LengthPercentage::length(base.value() + pixels)
            }
            taffy::CompactLength::PERCENT_TAG => {
                taffy::LengthPercentage::calc(self.added_handle(base, pixels))
            }
            _ if base.is_plain_calc() => {
                taffy::LengthPercentage::calc(self.added_handle(base, pixels))
            }
            _ => taffy::LengthPercentage::length(pixels),
        }
    }

    fn added_handle(&self, base: taffy::CompactLength, pixels: f32) -> *const () {
        let left = if base.tag() == taffy::CompactLength::PERCENT_TAG {
            Expression {
                kind: Kind::Length,
                value: Value::Percentage(base.value()),
            }
        } else {
            VALUES.with(|values| {
                values
                    .borrow()
                    .get(&(base.calc_value() as usize))
                    .expect("CSS expression outlived its layout pass")
                    .clone()
            })
        };
        let expression = Expression {
            kind: Kind::Length,
            value: Value::Binary(
                '+',
                Box::new(left),
                Box::new(Expression::constant(pixels, Kind::Length)),
            ),
        };
        self.store(if pixels < 0.0 {
            Expression {
                kind: Kind::Length,
                value: Value::Clamp(Box::new(expression)),
            }
        } else {
            expression
        })
    }

    fn store(&self, expression: Expression) -> *const () {
        // Taffy treats this aligned integer as an opaque handle, never as a pointer to dereference.
        let key = FREE
            .with(|free| free.borrow_mut().pop())
            .unwrap_or_else(|| {
                NEXT.fetch_add(1, Ordering::Relaxed)
                    .checked_mul(8)
                    .expect("CSS expression handle overflow")
            });
        VALUES.with(|values| values.borrow_mut().insert(key, expression));
        self.0.borrow_mut().push(key);
        key as *const ()
    }
}

pub(crate) fn resolve(handle: *const (), basis: f32) -> f32 {
    VALUES.with(|values| {
        let values = values.borrow();
        let value = values
            .get(&(handle as usize))
            .expect("CSS expression outlived its layout pass")
            .evaluate(basis);
        finite_result(value)
    })
}

fn finite_result(value: f32) -> f32 {
    if value.is_nan() || value == 0.0 {
        0.0
    } else {
        value.clamp(-1.0e12, 1.0e12)
    }
}

fn literal(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}

fn modulo(value: f32, step: f32) -> f32 {
    if !value.is_finite() || step == 0.0 {
        return f32::NAN;
    }
    if step.is_infinite() {
        return if value.is_sign_negative() == step.is_sign_negative() {
            value
        } else {
            f32::NAN
        };
    }
    let remainder = value % step;
    if remainder == 0.0 {
        0.0_f32.copysign(step)
    } else if remainder.is_sign_negative() != step.is_sign_negative() {
        remainder + step
    } else {
        remainder
    }
}

fn round(value: f32, step: f32, strategy: &str) -> f32 {
    let step = step.abs();
    if step == 0.0 || value.is_infinite() && step.is_infinite() {
        return f32::NAN;
    }
    if value.is_infinite() || value == 0.0 {
        return value;
    }
    if step.is_infinite() {
        return match strategy {
            "round-up" if value > 0.0 => f32::INFINITY,
            "round-down" if value < 0.0 => f32::NEG_INFINITY,
            _ => 0.0_f32.copysign(value),
        };
    }
    let quotient = f64::from(value) / f64::from(step);
    let multiple = match strategy {
        "round-up" => quotient.ceil(),
        "round-down" => quotient.floor(),
        "round-to-zero" => quotient.trunc(),
        _ => (quotient + 0.5).floor(),
    };
    let result = (multiple * f64::from(step)) as f32;
    if result == 0.0 {
        result.copysign(value)
    } else {
        result
    }
}

fn round_line_width(value: f32, step: f32, pixels_per_point: f32) -> f32 {
    let step = step.abs();
    if step == 0.0 {
        return f32::NAN;
    }

    let quotient = value / step;
    let lower = quotient.floor() * step;
    let upper = lower + step;
    let rounded = if value == lower || value == upper {
        value
    } else if lower == 0.0 {
        upper
    } else if upper == 0.0 || value - lower < upper - value {
        lower
    } else {
        upper
    };

    snap_line_width(rounded, pixels_per_point)
}

fn snap_line_width(value: f32, pixels_per_point: f32) -> f32 {
    if value == 0.0 {
        return value;
    }

    let pixels_per_point = pixels_per_point.max(f32::MIN_POSITIVE);
    let device_pixels = value * pixels_per_point;
    if device_pixels.fract() == 0.0 {
        value
    } else if device_pixels.abs() < 1.0 {
        device_pixels.signum() / pixels_per_point
    } else {
        device_pixels.trunc() / pixels_per_point
    }
}

pub(crate) fn length(
    value: &str,
    units: Units<'_>,
    negative: bool,
    percentage: bool,
) -> Result<Option<*const ()>, &'static str> {
    if !value.contains('(') {
        return Ok(None);
    }
    let mut parser = Parser::new(value);
    let mut result = calculation(&mut parser, units).map_err(|_| "invalid CSS math expression")?;
    parser
        .expect_exhausted()
        .map_err(|_| "unexpected CSS math input")?;
    if result.kind != Kind::Length {
        return Err("CSS length math must produce a length");
    }
    if !percentage && result.percentage() {
        return Err("percentage not allowed here");
    }
    if !negative {
        result = Expression {
            kind: Kind::Length,
            value: Value::Clamp(Box::new(result)),
        };
    }
    Ok(Some(units.math.store(result)))
}

pub(crate) fn number(value: &str, units: Units<'_>) -> Result<Option<f32>, &'static str> {
    if !value.contains('(') {
        return Ok(None);
    }
    let mut parser = Parser::new(value);
    let result = calculation(&mut parser, units).map_err(|_| "invalid CSS number expression")?;
    parser
        .expect_exhausted()
        .map_err(|_| "unexpected CSS math input")?;
    if result.kind != Kind::Number {
        return Err("CSS number math must produce a number");
    }
    Ok(Some(finite_result(result.evaluate(0.0))))
}

pub(crate) fn percentage(
    value: &str,
    units: Units<'_>,
    negative: bool,
) -> Result<Option<*const ()>, &'static str> {
    if !value.contains('(') {
        return Ok(None);
    }
    let mut parser = Parser::new(value);
    let mut result =
        calculation(&mut parser, units).map_err(|_| "invalid CSS percentage expression")?;
    parser
        .expect_exhausted()
        .map_err(|_| "unexpected CSS percentage input")?;
    if result.kind != Kind::Length || !result.percentage_only() {
        return Ok(None);
    }
    if !negative {
        result = Expression {
            kind: Kind::Length,
            value: Value::Clamp(Box::new(result)),
        };
    }
    Ok(Some(units.math.store(result)))
}

type ResultExpr = Result<Expression, ParseError<&'static str>>;
fn calculation<'i>(parser: &mut Parser<'i>, units: Units<'_>) -> ResultExpr {
    let name = parser.expect_function()?.to_ascii_lowercase();
    parser.parse_nested_block(|parser| function(&name, parser, units, &mut 128))
}

fn sum<'i>(parser: &mut Parser<'i>, units: Units<'_>, budget: &mut usize) -> ResultExpr {
    let mut left = product(parser, units, budget)?;
    loop {
        let operator = parser.try_parse(|p| match p.next()? {
            Token::Delim(op @ ('+' | '-')) => Ok(*op),
            _ => Err(ParseError::<&'static str>::custom("expected operator")),
        });
        let Ok(op) = operator else {
            break;
        };
        let right = product(parser, units, budget)?;
        if left.kind != right.kind {
            return Err(ParseError::custom("incompatible CSS math dimensions"));
        }
        left = Expression {
            kind: left.kind,
            value: Value::Binary(op, Box::new(left), Box::new(right)),
        };
    }
    Ok(left)
}
fn product<'i>(parser: &mut Parser<'i>, units: Units<'_>, budget: &mut usize) -> ResultExpr {
    let mut left = atom(parser, units, budget)?;
    loop {
        let operator = parser.try_parse(|p| match p.next()? {
            Token::Delim(op @ ('*' | '/')) => Ok(*op),
            _ => Err(ParseError::<&'static str>::custom("expected operator")),
        });
        let Ok(op) = operator else {
            break;
        };
        let right = atom(parser, units, budget)?;
        let kind = if right.kind == Kind::Number {
            left.kind
        } else if op == '*' && left.kind == Kind::Number {
            right.kind
        } else if op == '/' && left.kind == right.kind {
            Kind::Number
        } else {
            return Err(ParseError::custom("invalid CSS math product"));
        };
        left = Expression {
            kind,
            value: Value::Binary(op, Box::new(left), Box::new(right)),
        };
    }
    Ok(left)
}
fn atom<'i>(parser: &mut Parser<'i>, units: Units<'_>, budget: &mut usize) -> ResultExpr {
    *budget = budget
        .checked_sub(1)
        .ok_or_else(|| ParseError::custom("CSS math expression is too complex"))?;
    let start = parser.position();
    let token = parser.next()?.clone();
    Ok(match token {
        Token::Number { value, .. } => Expression::constant(literal(value), Kind::Number),
        Token::Percentage { unit_value, .. } => Expression {
            kind: Kind::Length,
            value: Value::Percentage(literal(unit_value)),
        },
        Token::Dimension { value, unit, .. } => {
            let value = literal(value);
            let angle = match unit.to_ascii_lowercase().as_str() {
                "deg" => Some(value.to_radians()),
                "rad" => Some(value),
                "grad" => Some(value * std::f32::consts::PI / 200.0),
                "turn" => Some(value * std::f32::consts::TAU),
                _ => None,
            };
            if let Some(angle) = angle {
                Expression::constant(angle, Kind::Angle)
            } else {
                Expression::constant(
                    literal(
                        crate::layout_style::length_value(parser.slice_from(start), units, true)
                            .map_err(ParseError::custom)?
                            .0,
                    ),
                    Kind::Length,
                )
            }
        }
        Token::ParenthesisBlock => parser.parse_nested_block(|p| sum(p, units, budget))?,
        Token::Delim(op @ ('+' | '-')) => {
            let value = atom(parser, units, budget)?;
            Expression {
                kind: value.kind,
                value: Value::Binary(
                    '*',
                    Box::new(Expression::constant(
                        if op == '-' { -1.0 } else { 1.0 },
                        Kind::Number,
                    )),
                    Box::new(value),
                ),
            }
        }
        Token::Ident(value) => {
            let value = value.to_ascii_lowercase();
            Expression::constant(
                match value.as_str() {
                    "pi" => std::f32::consts::PI,
                    "e" => std::f32::consts::E,
                    "infinity" => f32::INFINITY,
                    "-infinity" => f32::NEG_INFINITY,
                    "nan" => f32::NAN,
                    "none" => f32::NAN,
                    _ => return Err(ParseError::custom("invalid CSS math constant")),
                },
                if value == "none" {
                    Kind::Any
                } else {
                    Kind::Number
                },
            )
        }
        Token::Function(name) => {
            parser.parse_nested_block(|p| function(&name.to_ascii_lowercase(), p, units, budget))?
        }
        _ => return Err(ParseError::custom("invalid CSS math operand")),
    })
}
fn function<'i>(
    name: &str,
    parser: &mut Parser<'i>,
    units: Units<'_>,
    budget: &mut usize,
) -> ResultExpr {
    let mut name = name.to_owned();
    let mut line_width_rounding = false;
    if name == "round"
        && let Ok(strategy) = parser.try_parse(|p| {
            let value = p.expect_ident_cloned()?.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "nearest" | "up" | "down" | "to-zero" | "line-width"
            ) {
                return Err(ParseError::<&'static str>::custom("invalid round strategy"));
            }
            p.expect_comma()?;
            Ok(value)
        })
    {
        line_width_rounding = strategy == "line-width";
        name.push('-');
        name.push_str(&strategy);
    }
    let mut args = parser.parse_comma_separated(|p| sum(p, units, budget))?;
    let kind = args
        .iter()
        .find(|a| a.kind != Kind::Any)
        .ok_or_else(|| ParseError::custom("missing typed operand"))?
        .kind;
    let count = args.len();
    let output = match name.as_str() {
        "calc" | "abs" if count == 1 => kind,
        "min" | "max" | "hypot" if count > 0 => kind,
        "clamp" if count == 3 => {
            if args[0].kind == Kind::Any {
                args[0] = Expression::constant(f32::NEG_INFINITY, kind);
            }
            if args[2].kind == Kind::Any {
                args[2] = Expression::constant(f32::INFINITY, kind);
            }
            kind
        }
        "mod" | "rem" if count == 2 => kind,
        "round-line-width"
            if line_width_rounding && (1..=2).contains(&count) && kind == Kind::Length =>
        {
            kind
        }
        n if n.starts_with("round")
            && matches!(
                n,
                "round" | "round-nearest" | "round-up" | "round-down" | "round-to-zero"
            )
            && (count == 2 || count == 1 && kind == Kind::Number) =>
        {
            kind
        }
        "sign" if count == 1 => Kind::Number,
        "sin" | "cos" | "tan" if count == 1 && matches!(kind, Kind::Number | Kind::Angle) => {
            Kind::Number
        }
        "asin" | "acos" | "atan" if count == 1 && kind == Kind::Number => Kind::Angle,
        "atan2" if count == 2 => Kind::Angle,
        "sqrt" | "exp" if count == 1 && kind == Kind::Number => Kind::Number,
        "pow" if count == 2 && kind == Kind::Number => Kind::Number,
        "log" if (count == 1 || count == 2) && kind == Kind::Number => Kind::Number,
        _ => return Err(ParseError::custom("invalid CSS math function")),
    };
    if args.iter().any(|arg| arg.kind != kind) {
        return Err(ParseError::custom("incompatible function dimensions"));
    }
    let pixels_per_point = line_width_rounding.then_some(units.pixels_per_point);
    Ok(Expression {
        kind: output,
        value: Value::Function(name, args, pixels_per_point),
    })
}
