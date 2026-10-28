//! Builtin Tierkreis task implementations.
//!
//! The implementations in this module are shared by the in-memory executor and
//! the standalone `tkr-builtins` subprocess worker.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use miette::{Context, IntoDiagnostic, miette};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Raw, serialized values keyed by task port name.
pub type PortValues = HashMap<String, Vec<u8>>;

/// Every builtin task name accepted by [`execute`], including legacy aliases.
pub const SUPPORTED_TASKS: &[&str] = &[
    "add",
    "append",
    "at",
    "concat",
    "concat_lists",
    "conjugate",
    "divide",
    "drop",
    "eq",
    "fabs",
    "fadd",
    "fceil",
    "feq",
    "ffloor",
    "fgt",
    "fgte",
    "flatten",
    "flt",
    "flte",
    "fmax",
    "fmin",
    "fmul",
    "fne",
    "fneg",
    "fsub",
    "ftostring",
    "gt",
    "head",
    "iabs",
    "iadd",
    "iand",
    "idiv",
    "idiv_s",
    "idivide",
    "ieq",
    "igt",
    "igt_s",
    "igte_s",
    "ilt_s",
    "ilte_s",
    "imax_s",
    "imin_s",
    "imod_s",
    "imul",
    "ine",
    "ineg",
    "ior",
    "ipow",
    "isub",
    "isubtract",
    "itimes",
    "itostring_s",
    "ixor",
    "lt",
    "max_item",
    "mean",
    "min_item",
    "mod",
    "ne",
    "neg",
    "neq",
    "prod_list",
    "rand_int",
    "range",
    "sleep",
    "sort_number_list",
    "sort_string_list",
    "str",
    "str_eq",
    "str_neq",
    "subtract",
    "sum_list",
    "take",
    "times",
    "tkr_abs",
    "tkr_all",
    "tkr_and",
    "tkr_any",
    "tkr_decode",
    "tkr_encode",
    "tkr_extend",
    "tkr_id",
    "tkr_int",
    "tkr_len",
    "tkr_or",
    "tkr_pow",
    "tkr_range",
    "tkr_reversed",
    "tkr_round",
    "tkr_sleep",
    "tkr_str",
    "tkr_tuple",
    "tkr_zip",
    "tuple",
    "untuple",
    "unzip",
];

/// Return every task name accepted by [`execute`].
#[must_use]
pub const fn supported_tasks() -> &'static [&'static str] {
    SUPPORTED_TASKS
}

fn extract_value<'a, T>(inputs: &'a PortValues, name: &str) -> miette::Result<T>
where
    T: Deserialize<'a>,
{
    let asset = inputs
        .get(name)
        .ok_or_else(|| miette!("Missing input: {name}"))?;
    serde_json::from_slice(asset)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Invalid JSON for input `{name}`"))
}

fn extract_optional_value<'a, T>(inputs: &'a PortValues, name: &str) -> miette::Result<Option<T>>
where
    T: Deserialize<'a>,
{
    inputs
        .get(name)
        .map(|asset| serde_json::from_slice(asset).into_diagnostic())
        .transpose()
        .map(Option::flatten)
}

fn output_value(outputs: &mut PortValues, value: impl Serialize) -> miette::Result<()> {
    outputs.insert(
        "value".to_string(),
        serde_json::to_vec(&value).into_diagnostic()?,
    );
    Ok(())
}

fn numeric_value(value: &Value) -> miette::Result<f64> {
    value
        .as_f64()
        .ok_or_else(|| miette!("Expected a number, found {value}"))
}

fn numeric_binary(
    inputs: &PortValues,
    integer: impl FnOnce(i64, i64) -> miette::Result<i64>,
    float: impl FnOnce(f64, f64) -> f64,
) -> miette::Result<Value> {
    let a: Value = extract_value(inputs, "a")?;
    let b: Value = extract_value(inputs, "b")?;
    if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        return Ok(Value::from(integer(a, b)?));
    }
    let result = float(numeric_value(&a)?, numeric_value(&b)?);
    serde_json::to_value(result).into_diagnostic()
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64() != Some(0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

fn slice_index(index: i64, len: usize) -> usize {
    if index >= 0 {
        usize::try_from(index).unwrap_or(usize::MAX).min(len)
    } else {
        len.saturating_sub(index.unsigned_abs().try_into().unwrap_or(usize::MAX))
    }
}

macro_rules! un_op {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;
        output_value(&mut $outputs, $op a)?;
    }};
}

macro_rules! bin_op {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;
        let b: $input_type = extract_value($inputs, "b")?;
        output_value(&mut $outputs, a $op b)?;
    }};
}

macro_rules! un_method {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;
        output_value(&mut $outputs, a.$op())?;
    }};
}

macro_rules! bin_method {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;
        let b: $input_type = extract_value($inputs, "b")?;
        output_value(&mut $outputs, a.$op(b))?;
    }};
}

/// Execute a builtin task using raw serialized input and output port values.
///
/// # Errors
///
/// Returns an error when the task is unknown, an input is absent or malformed,
/// or the requested operation is invalid.
#[allow(clippy::too_many_lines)]
pub fn execute(task_name: &str, inputs: &PortValues) -> miette::Result<PortValues> {
    let mut outputs = HashMap::new();
    match task_name {
        "ieq" => bin_op!(inputs, outputs, ==, i64),
        "ine" => bin_op!(inputs, outputs, !=, i64),
        "ilt_s" => bin_op!(inputs, outputs, <, i64),
        "igt_s" | "igt" => bin_op!(inputs, outputs, >, i64),
        "ilte_s" => bin_op!(inputs, outputs, <=, i64),
        "igte_s" => bin_op!(inputs, outputs, >=, i64),
        "imax_s" => bin_method!(inputs, outputs, max, i64),
        "imin_s" => bin_method!(inputs, outputs, min, i64),
        "iadd" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            output_value(
                &mut outputs,
                a.checked_add(b)
                    .ok_or_else(|| miette!("Integer addition overflow"))?,
            )?;
        }
        "isub" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            output_value(
                &mut outputs,
                a.checked_sub(b)
                    .ok_or_else(|| miette!("Integer subtraction overflow"))?,
            )?;
        }
        "ineg" => {
            let a: i64 = extract_value(inputs, "a")?;
            output_value(
                &mut outputs,
                a.checked_neg()
                    .ok_or_else(|| miette!("Integer negation overflow"))?,
            )?;
        }
        "imul" | "itimes" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            output_value(
                &mut outputs,
                a.checked_mul(b)
                    .ok_or_else(|| miette!("Integer multiplication overflow"))?,
            )?;
        }
        "idiv_s" | "idivide" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            let value = a
                .checked_div(b)
                .ok_or_else(|| miette!("Invalid integer division: {a} / {b}"))?;
            output_value(&mut outputs, value)?;
        }
        "imod_s" | "mod" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            let value = a
                .checked_rem(b)
                .ok_or_else(|| miette!("Invalid integer remainder: {a} % {b}"))?;
            output_value(&mut outputs, value)?;
        }
        "iabs" => {
            let a: i64 = extract_value(inputs, "a")?;
            output_value(
                &mut outputs,
                a.checked_abs()
                    .ok_or_else(|| miette!("Integer absolute-value overflow"))?,
            )?;
        }
        "iand" => bin_op!(inputs, outputs, &, i64),
        "ior" => bin_op!(inputs, outputs, |, i64),
        "ixor" => bin_op!(inputs, outputs, ^, i64),
        "itostring_s" => un_method!(inputs, outputs, to_string, i64),

        #[allow(clippy::float_cmp)]
        "feq" => bin_op!(inputs, outputs, ==, f64),
        #[allow(clippy::float_cmp)]
        "fne" => bin_op!(inputs, outputs, !=, f64),
        "flt" | "lt" => bin_op!(inputs, outputs, <, f64),
        "fgt" | "gt" => bin_op!(inputs, outputs, >, f64),
        "flte" => bin_op!(inputs, outputs, <=, f64),
        "fgte" => bin_op!(inputs, outputs, >=, f64),
        "fmax" => bin_method!(inputs, outputs, max, f64),
        "fmin" => bin_method!(inputs, outputs, min, f64),
        "fadd" => bin_op!(inputs, outputs, +, f64),
        "fsub" => bin_op!(inputs, outputs, -, f64),
        "fneg" => un_op!(inputs, outputs, -, f64),
        "fabs" => un_method!(inputs, outputs, abs, f64),
        "fmul" => bin_op!(inputs, outputs, *, f64),
        "idiv" => bin_op!(inputs, outputs, /, f64),
        "ffloor" => un_method!(inputs, outputs, floor, f64),
        "fceil" => un_method!(inputs, outputs, ceil, f64),
        "ftostring" => un_method!(inputs, outputs, to_string, f64),

        "add" => output_value(
            &mut outputs,
            numeric_binary(
                inputs,
                |a, b| {
                    a.checked_add(b)
                        .ok_or_else(|| miette!("Integer addition overflow"))
                },
                |a, b| a + b,
            )?,
        )?,
        "isubtract" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            output_value(
                &mut outputs,
                a.checked_sub(b)
                    .ok_or_else(|| miette!("Integer subtraction overflow"))?,
            )?;
        }
        "subtract" => output_value(
            &mut outputs,
            numeric_binary(
                inputs,
                |a, b| {
                    a.checked_sub(b)
                        .ok_or_else(|| miette!("Integer subtraction overflow"))
                },
                |a, b| a - b,
            )?,
        )?,
        "times" => output_value(
            &mut outputs,
            numeric_binary(
                inputs,
                |a, b| {
                    a.checked_mul(b)
                        .ok_or_else(|| miette!("Integer multiplication overflow"))
                },
                |a, b| a * b,
            )?,
        )?,
        "divide" => {
            let a: f64 = extract_value(inputs, "a")?;
            let b: f64 = extract_value(inputs, "b")?;
            if b == 0.0 {
                return Err(miette!("Division by zero"));
            }
            output_value(&mut outputs, a / b)?;
        }
        "ipow" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: u32 = extract_value::<i64>(inputs, "b")?
                .try_into()
                .into_diagnostic()
                .wrap_err("Integer exponent must be non-negative")?;
            output_value(
                &mut outputs,
                a.checked_pow(b)
                    .ok_or_else(|| miette!("Integer exponentiation overflow"))?,
            )?;
        }
        "tkr_pow" => {
            let a: Value = extract_value(inputs, "a")?;
            let b: Value = extract_value(inputs, "b")?;
            let value = if let (Some(a), Some(b)) = (a.as_i64(), b.as_u64()) {
                let exponent: u32 = b.try_into().into_diagnostic()?;
                Value::from(
                    a.checked_pow(exponent)
                        .ok_or_else(|| miette!("Integer exponentiation overflow"))?,
                )
            } else {
                serde_json::to_value(numeric_value(&a)?.powf(numeric_value(&b)?))
                    .into_diagnostic()?
            };
            output_value(&mut outputs, value)?;
        }
        "tkr_abs" => {
            let value: Value = extract_value(inputs, "a")?;
            let value = if let Some(value) = value.as_i64() {
                Value::from(
                    value
                        .checked_abs()
                        .ok_or_else(|| miette!("Integer absolute-value overflow"))?,
                )
            } else {
                serde_json::to_value(numeric_value(&value)?.abs()).into_diagnostic()?
            };
            output_value(&mut outputs, value)?;
        }
        "tkr_round" => {
            let a: f64 = extract_value(inputs, "a")?;
            if !a.is_finite() {
                return Err(miette!("Cannot round {a} to an integer"));
            }
            let rounded: i64 = a
                .round_ties_even()
                .to_string()
                .parse()
                .into_diagnostic()
                .wrap_err_with(|| miette!("Cannot round {a} to an integer"))?;
            output_value(&mut outputs, rounded)?;
        }
        "neg" => {
            let a: bool = extract_value(inputs, "a")?;
            output_value(&mut outputs, !a)?;
        }
        "tkr_and" => {
            let a: bool = extract_value(inputs, "a")?;
            let b: bool = extract_value(inputs, "b")?;
            output_value(&mut outputs, a && b)?;
        }
        "tkr_or" => {
            let a: bool = extract_value(inputs, "a")?;
            let b: bool = extract_value(inputs, "b")?;
            output_value(&mut outputs, a || b)?;
        }

        "eq" => bin_op!(inputs, outputs, ==, Value),
        "ne" | "neq" => bin_op!(inputs, outputs, !=, Value),
        "str" | "tkr_str" => {
            let value: Value = extract_value(inputs, "value")?;
            output_value(&mut outputs, value.to_string())?;
        }
        "str_eq" => bin_op!(inputs, outputs, ==, String),
        "str_neq" => bin_op!(inputs, outputs, !=, String),
        "tkr_id" => {
            let value = inputs
                .get("value")
                .ok_or_else(|| miette!("Missing input: value"))?;
            outputs.insert("value".to_string(), value.clone());
        }
        "append" => {
            let mut values: Vec<Value> = extract_value(inputs, "v")?;
            values.push(extract_value(inputs, "a")?);
            output_value(&mut outputs, values)?;
        }
        "head" => {
            let mut values: Vec<Value> = extract_value(inputs, "v")?;
            if values.is_empty() {
                return Err(miette!("Cannot take the head of an empty list"));
            }
            let head = values.remove(0);
            outputs.insert(
                "head".to_string(),
                serde_json::to_vec(&head).into_diagnostic()?,
            );
            outputs.insert(
                "rest".to_string(),
                serde_json::to_vec(&values).into_diagnostic()?,
            );
        }
        "tkr_len" => {
            let values: Vec<Value> = extract_value(inputs, "v")?;
            output_value(&mut outputs, i64::try_from(values.len()).into_diagnostic()?)?;
        }
        "concat" => {
            let lhs: String = extract_value(inputs, "lhs")?;
            let rhs: String = extract_value(inputs, "rhs")?;
            output_value(&mut outputs, lhs + &rhs)?;
        }
        "tkr_zip" => {
            let a: Vec<Value> = extract_value(inputs, "a")?;
            let b: Vec<Value> = extract_value(inputs, "b")?;
            output_value(&mut outputs, a.into_iter().zip(b).collect::<Vec<_>>())?;
        }
        "unzip" => {
            let values: Vec<(Value, Value)> = extract_value(inputs, "value")?;
            if values.is_empty() {
                return Err(miette!("Cannot unzip an empty list"));
            }
            let (a, b): (Vec<_>, Vec<_>) = values.into_iter().unzip();
            outputs.insert("a".to_string(), serde_json::to_vec(&a).into_diagnostic()?);
            outputs.insert("b".to_string(), serde_json::to_vec(&b).into_diagnostic()?);
        }
        "tuple" | "tkr_tuple" => {
            let a: Value = extract_value(inputs, "a")?;
            let b: Value = extract_value(inputs, "b")?;
            output_value(&mut outputs, [a, b])?;
        }
        "untuple" => {
            let value: (Value, Value) = extract_value(inputs, "value")?;
            outputs.insert(
                "a".to_string(),
                serde_json::to_vec(&value.0).into_diagnostic()?,
            );
            outputs.insert(
                "b".to_string(),
                serde_json::to_vec(&value.1).into_diagnostic()?,
            );
        }
        "mean" => {
            let values: Vec<f64> = extract_value(inputs, "values")?;
            if values.is_empty() {
                return Err(miette!("Cannot obtain `mean` of an empty list"));
            }
            let count: u32 = values.len().try_into().into_diagnostic()?;
            output_value(&mut outputs, values.iter().sum::<f64>() / f64::from(count))?;
        }
        "range" | "tkr_range" => range_impl(inputs, &mut outputs)?,
        "conjugate" => {
            let z: Complex64 = extract_value(inputs, "z")?;
            output_value(&mut outputs, z.conj())?;
        }
        "sleep" | "tkr_sleep" => {
            let delay_seconds: f64 = extract_value(inputs, "delay_seconds")?;
            if !delay_seconds.is_finite() || delay_seconds.is_sign_negative() {
                return Err(miette!("Sleep delay must be a finite, non-negative number"));
            }
            sleep(Duration::from_secs_f64(delay_seconds));
            output_value(&mut outputs, true)?;
        }
        "rand_int" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;
            if a > b {
                return Err(miette!("Random integer range is empty: {a}..={b}"));
            }
            output_value(&mut outputs, rand::random_range(a..=b))?;
        }
        "tkr_encode" => {
            let string: String = extract_value(inputs, "string")?;
            outputs.insert("value".to_string(), string.into_bytes());
        }
        "tkr_decode" => {
            let bytes = inputs
                .get("value_bytes")
                .ok_or_else(|| miette!("Missing input: value_bytes"))?;
            output_value(
                &mut outputs,
                String::from_utf8(bytes.clone()).into_diagnostic()?,
            )?;
        }
        "tkr_all" => {
            let values: Vec<Value> = extract_value(inputs, "values")?;
            output_value(&mut outputs, values.iter().all(truthy))?;
        }
        "tkr_any" => {
            let values: Vec<Value> = extract_value(inputs, "values")?;
            output_value(&mut outputs, values.iter().any(truthy))?;
        }
        "tkr_reversed" => {
            let mut values: Vec<Value> = extract_value(inputs, "values")?;
            values.reverse();
            output_value(&mut outputs, values)?;
        }
        "tkr_extend" | "concat_lists" => {
            let mut first: Vec<Value> = extract_value(inputs, "first")?;
            first.extend(extract_value::<Vec<Value>>(inputs, "second")?);
            output_value(&mut outputs, first)?;
        }
        "tkr_int" => {
            let value: Value = extract_value(inputs, "value")?;
            let value = match value {
                Value::Bool(value) => i64::from(value),
                Value::Number(value) => {
                    if let Some(value) = value.as_i64() {
                        value
                    } else {
                        let value = value.as_f64().ok_or_else(|| miette!("Invalid number"))?;
                        if !value.is_finite() {
                            return Err(miette!("Integer conversion is out of range"));
                        }
                        value
                            .trunc()
                            .to_string()
                            .parse()
                            .into_diagnostic()
                            .wrap_err("Integer conversion is out of range")?
                    }
                }
                Value::String(value) => value.parse().into_diagnostic()?,
                value => return Err(miette!("Cannot convert {value} to an integer")),
            };
            output_value(&mut outputs, value)?;
        }
        "sum_list" | "prod_list" => {
            let values: Vec<Value> = extract_value(inputs, "values")?;
            let product = task_name == "prod_list";
            let all_integer = values.iter().all(|value| value.as_i64().is_some());
            if all_integer {
                let initial = i64::from(product);
                let value = values.into_iter().try_fold(initial, |total, value| {
                    let value = value
                        .as_i64()
                        .ok_or_else(|| miette!("Expected an integer"))?;
                    if product {
                        total.checked_mul(value)
                    } else {
                        total.checked_add(value)
                    }
                    .ok_or_else(|| miette!("Integer aggregation overflow"))
                })?;
                output_value(&mut outputs, value)?;
            } else {
                let initial = if product { 1.0 } else { 0.0 };
                let value = values.into_iter().try_fold(initial, |total, value| {
                    numeric_value(&value).map(|value| {
                        if product {
                            total * value
                        } else {
                            total + value
                        }
                    })
                })?;
                output_value(&mut outputs, value)?;
            }
        }
        "max_item" | "min_item" => {
            let values: Vec<Value> = extract_value(inputs, "values")?;
            let mut values = values.into_iter();
            let mut selected = values
                .next()
                .ok_or_else(|| miette!("Cannot select from an empty list"))?;
            for value in values {
                let ordering = numeric_value(&value)?.total_cmp(&numeric_value(&selected)?);
                if (task_name == "max_item" && ordering.is_gt())
                    || (task_name == "min_item" && ordering.is_lt())
                {
                    selected = value;
                }
            }
            output_value(&mut outputs, selected)?;
        }
        "sort_number_list" => {
            let mut values: Vec<Value> = extract_value(inputs, "values")?;
            for value in &values {
                numeric_value(value)?;
            }
            values.sort_by(|a, b| {
                numeric_value(a)
                    .and_then(|a| numeric_value(b).map(|b| a.total_cmp(&b)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            output_value(&mut outputs, values)?;
        }
        "sort_string_list" => {
            let mut values: Vec<String> = extract_value(inputs, "values")?;
            values.sort();
            output_value(&mut outputs, values)?;
        }
        "flatten" => {
            let values: Vec<Vec<Value>> = extract_value(inputs, "values")?;
            output_value(
                &mut outputs,
                values.into_iter().flatten().collect::<Vec<_>>(),
            )?;
        }
        "take" | "drop" => {
            let values: Vec<Value> = extract_value(inputs, "values")?;
            let n: i64 = extract_value(inputs, "n")?;
            let index = slice_index(n, values.len());
            let value = if task_name == "take" {
                values[..index].to_vec()
            } else {
                values[index..].to_vec()
            };
            output_value(&mut outputs, value)?;
        }
        "at" => {
            let values: Vec<Value> = extract_value(inputs, "values")?;
            let n: i64 = extract_value(inputs, "n")?;
            let index = if n < 0 {
                values
                    .len()
                    .checked_sub(n.unsigned_abs().try_into().unwrap_or(usize::MAX))
            } else {
                usize::try_from(n)
                    .ok()
                    .filter(|index| *index < values.len())
            }
            .ok_or_else(|| miette!("List index out of range: {n}"))?;
            output_value(&mut outputs, values[index].clone())?;
        }
        task => return Err(miette!("Unknown task: `{task}`")),
    }
    Ok(outputs)
}

fn range_impl(inputs: &PortValues, outputs: &mut PortValues) -> miette::Result<()> {
    let start: i64 = extract_value(inputs, "start")?;
    let stop: i64 = extract_value(inputs, "stop")?;
    let maybe_step: Option<i64> = extract_optional_value(inputs, "step")?;
    let range = start..stop;
    let out = if let Some(step) = maybe_step {
        let step_size: usize = step
            .try_into()
            .into_diagnostic()
            .wrap_err("Range step must be positive")?;
        if step_size == 0 {
            return Err(miette!("Range step must be positive"));
        }
        range.step_by(step_size).collect::<Vec<_>>()
    } else {
        range.collect::<Vec<_>>()
    };
    output_value(outputs, out)
}

/// File protocol used to invoke a Tierkreis subprocess worker.
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct WorkerCallArgs {
    pub(crate) function_name: String,
    pub(crate) inputs: HashMap<String, PathBuf>,
    pub(crate) outputs: HashMap<String, PathBuf>,
    pub(crate) output_dir: PathBuf,
    pub(crate) done_path: PathBuf,
    pub(crate) error_path: PathBuf,
    pub(crate) logs_path: Option<PathBuf>,
}

fn worker_storage_root() -> PathBuf {
    std::env::var_os("TKR_DIR").map_or_else(
        || crate::config::tierkreis_home_dir().join("checkpoints"),
        PathBuf::from,
    )
}

fn resolve_worker_path(storage_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        storage_root.join(path)
    }
}

fn run_worker_call_inner(args: &WorkerCallArgs, storage_root: &Path) -> miette::Result<()> {
    let inputs = args
        .inputs
        .iter()
        .map(|(name, path)| {
            let path = resolve_worker_path(storage_root, path);
            fs::read(&path)
                .into_diagnostic()
                .wrap_err_with(|| miette!("Failed to read input `{name}` from {}", path.display()))
                .map(|value| (name.clone(), value))
        })
        .collect::<miette::Result<PortValues>>()?;
    let mut values = execute(&args.function_name, &inputs)?;

    for (name, path) in &args.outputs {
        let path = resolve_worker_path(storage_root, path);
        let value = values.remove(name).ok_or_else(|| {
            miette!(
                "Builtin `{}` did not produce output `{name}`",
                args.function_name
            )
        })?;
        fs::write(&path, value)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to write output `{name}` to {}", path.display()))?;
    }

    let done_path = resolve_worker_path(storage_root, &args.done_path);
    fs::File::create(&done_path)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to mark worker complete at {}", done_path.display()))?;
    Ok(())
}

/// Run a worker call encoded using the Tierkreis subprocess worker protocol.
///
/// This is public only so the `tkr-builtins` Cargo target can remain a thin
/// wrapper around the library implementation.
///
/// # Errors
///
/// Returns an error when the call definition cannot be read, the builtin fails,
/// or an output or status file cannot be written.
#[doc(hidden)]
pub fn run_worker_call(path: &Path) -> miette::Result<()> {
    let storage_root = worker_storage_root();
    let call_path = resolve_worker_path(&storage_root, path);
    let encoded = fs::read(&call_path)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to read worker call from {}", call_path.display()))?;
    let args: WorkerCallArgs = serde_json::from_slice(&encoded)
        .into_diagnostic()
        .wrap_err("Failed to parse worker call")?;
    if let Err(err) = run_worker_call_inner(&args, &storage_root) {
        let error_path = resolve_worker_path(&storage_root, &args.error_path);
        let _ = fs::write(error_path, err.to_string());
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn supported_tasks_are_sorted_and_unique() {
        assert!(SUPPORTED_TASKS.is_sorted());
        assert!(SUPPORTED_TASKS.windows(2).all(|names| names[0] != names[1]));
    }

    #[test]
    fn execute_iadd() {
        let inputs = HashMap::from([
            ("a".to_string(), serde_json::to_vec(&2).unwrap()),
            ("b".to_string(), serde_json::to_vec(&3).unwrap()),
        ]);
        let outputs = execute("iadd", &inputs).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&outputs["value"]).unwrap(),
            json!(5)
        );
    }

    #[test]
    fn invalid_integer_division_is_an_error() {
        let inputs = HashMap::from([
            ("a".to_string(), serde_json::to_vec(&2).unwrap()),
            ("b".to_string(), serde_json::to_vec(&0).unwrap()),
        ]);
        assert!(execute("idiv_s", &inputs).is_err());
    }
}
