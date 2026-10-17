use std::{
    cell::RefCell,
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::layout_style::Units;
use cssparser::{ParseError, Parser, Token};

thread_local! { static VALUES:RefCell<HashMap<usize,Expression>>=RefCell::default(); }
static NEXT: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Default)]
pub(crate) struct Arena(RefCell<Vec<usize>>);
impl Drop for Arena {
    fn drop(&mut self) {
        VALUES.with(|values| {
            let mut values = values.borrow_mut();
            for key in self.0.get_mut() {
                values.remove(key);
            }
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
#[derive(Clone, Debug)]
struct Expression {
    kind: Kind,
    value: Value,
}
#[derive(Clone, Debug)]
enum Value {
    Constant(f32),
    Percentage(f32),
    Binary(char, Box<Expression>, Box<Expression>),
    Function(String, Vec<Expression>),
    Clamp(Box<Expression>),
}

impl Expression {
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
            Value::Function(name, args) => {
                let values: Vec<_> = args.iter().map(|arg| arg.evaluate(basis)).collect();
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
                    "hypot" => values.iter().map(|v| v * v).sum::<f32>().sqrt(),
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
                    "mod" => a - values[1] * (a / values[1]).floor(),
                    "rem" => a % values[1],
                    name if name.starts_with("round") => {
                        let step = values.get(1).copied().unwrap_or(1.0);
                        let v = a / step;
                        let n = match name {
                            "round-up" => v.ceil(),
                            "round-down" => v.floor(),
                            "round-to-zero" => v.trunc(),
                            _ => (v + 0.5).floor(),
                        };
                        n * step
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
            Value::Function(_, a) => a.iter().any(Self::percentage),
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
            Value::Function(_, arguments) => arguments.iter().all(Self::percentage_only),
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
            taffy::CompactLength::CALC_TAG => {
                taffy::Dimension::calc(self.added_handle(raw, pixels))
            }
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
            taffy::CompactLength::PERCENT_TAG | taffy::CompactLength::CALC_TAG => {
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
            taffy::CompactLength::PERCENT_TAG | taffy::CompactLength::CALC_TAG => {
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
        let key = NEXT
            .fetch_add(1, Ordering::Relaxed)
            .checked_mul(8)
            .expect("CSS expression handle overflow");
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
        if value.is_nan() {
            0.0
        } else {
            value.clamp(-1.0e12, 1.0e12)
        }
    })
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
    let mut result = sum(&mut parser, units).map_err(|_| "invalid CSS math expression")?;
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
    // Taffy treats the aligned pointer as an opaque handle; no pointer is dereferenced.
    let key = NEXT
        .fetch_add(1, Ordering::Relaxed)
        .checked_mul(8)
        .expect("CSS expression handle overflow");
    VALUES.with(|values| values.borrow_mut().insert(key, result));
    units.math.0.borrow_mut().push(key);
    Ok(Some(key as *const ()))
}

pub(crate) fn number(value: &str, units: Units<'_>) -> Result<Option<f32>, &'static str> {
    if !value.contains('(') {
        return Ok(None);
    }
    let mut parser = Parser::new(value);
    let result = sum(&mut parser, units).map_err(|_| "invalid CSS number expression")?;
    parser
        .expect_exhausted()
        .map_err(|_| "unexpected CSS math input")?;
    if result.kind != Kind::Number {
        return Err("CSS number math must produce a number");
    }
    Ok(Some(result.evaluate(0.0)))
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
    let mut result = sum(&mut parser, units).map_err(|_| "invalid CSS percentage expression")?;
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
    let key = NEXT
        .fetch_add(1, Ordering::Relaxed)
        .checked_mul(8)
        .expect("CSS expression handle overflow");
    VALUES.with(|values| values.borrow_mut().insert(key, result));
    units.math.0.borrow_mut().push(key);
    Ok(Some(key as *const ()))
}

type ResultExpr = Result<Expression, ParseError<&'static str>>;
fn sum<'i>(parser: &mut Parser<'i>, units: Units<'_>) -> ResultExpr {
    let mut left = product(parser, units)?;
    loop {
        let operator = parser.try_parse(|p| match p.next()? {
            Token::Delim(op @ ('+' | '-')) => Ok(*op),
            _ => Err(ParseError::<&'static str>::custom("expected operator")),
        });
        let Ok(op) = operator else {
            break;
        };
        let right = product(parser, units)?;
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
fn product<'i>(parser: &mut Parser<'i>, units: Units<'_>) -> ResultExpr {
    let mut left = atom(parser, units)?;
    loop {
        let operator = parser.try_parse(|p| match p.next()? {
            Token::Delim(op @ ('*' | '/')) => Ok(*op),
            _ => Err(ParseError::<&'static str>::custom("expected operator")),
        });
        let Ok(op) = operator else {
            break;
        };
        let right = atom(parser, units)?;
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
fn atom<'i>(parser: &mut Parser<'i>, units: Units<'_>) -> ResultExpr {
    let start = parser.position();
    let token = parser.next()?.clone();
    Ok(match token {
        Token::Number { value, .. } => Expression::constant(value, Kind::Number),
        Token::Percentage { unit_value, .. } => Expression {
            kind: Kind::Length,
            value: Value::Percentage(unit_value),
        },
        Token::Dimension { value, unit, .. } => {
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
                    crate::layout_style::length_value(parser.slice_from(start), units, true)
                        .map_err(ParseError::custom)?
                        .0,
                    Kind::Length,
                )
            }
        }
        Token::ParenthesisBlock => parser.parse_nested_block(|p| sum(p, units))?,
        Token::Delim(op @ ('+' | '-')) => {
            let value = atom(parser, units)?;
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
            parser.parse_nested_block(|p| function(&name.to_ascii_lowercase(), p, units))?
        }
        _ => return Err(ParseError::custom("invalid CSS math operand")),
    })
}
fn function<'i>(name: &str, parser: &mut Parser<'i>, units: Units<'_>) -> ResultExpr {
    let mut name = name.to_owned();
    if name == "round"
        && let Ok(strategy) = parser.try_parse(|p| {
            let value = p.expect_ident_cloned()?;
            if !matches!(value.as_ref(), "nearest" | "up" | "down" | "to-zero") {
                return Err(ParseError::<&'static str>::custom("invalid round strategy"));
            }
            p.expect_comma()?;
            Ok(value.to_string())
        })
    {
        name.push('-');
        name.push_str(&strategy);
    }
    let mut args = parser.parse_comma_separated(|p| sum(p, units))?;
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
    Ok(Expression {
        kind: output,
        value: Value::Function(name, args),
    })
}
