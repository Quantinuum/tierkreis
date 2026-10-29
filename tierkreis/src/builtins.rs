/*!
This module defines the tierkreis builtin tasks.
*/
use std::{collections::HashMap, thread::sleep, time::Duration};

use miette::{IntoDiagnostic, miette};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn extract_value<'a, T>(inputs: &'a HashMap<String, Vec<u8>>, name: &str) -> miette::Result<T>
where
    T: Deserialize<'a>,
{
    let asset = inputs
        .get(name)
        .ok_or_else(|| miette!("Missing input: {name}"))?;
    let val = serde_json::from_slice(asset).into_diagnostic()?;

    Ok(val)
}

// Extract an value if set or return a default value if not set.
fn extract_value_with_default<'a, T>(
    inputs: &'a HashMap<String, Vec<u8>>,
    name: &str,
    default: T,
) -> miette::Result<T>
where
    T: Deserialize<'a>,
{
    let val = inputs
        .get(name)
        .map(|asset| {
            let val: T = serde_json::from_slice(asset).into_diagnostic()?;
            Ok::<_, miette::Report>(val)
        })
        .transpose()?;

    match (val, default) {
        (Some(val), _) => Ok(val),
        (None, default) => Ok(default),
    }
}

fn numeric_value(value: &Value) -> miette::Result<f64> {
    value
        .as_f64()
        .ok_or_else(|| miette!("Expected a number, found {value}"))
}

fn output_value(
    outputs: &mut HashMap<String, Vec<u8>>,
    value: impl Serialize,
) -> miette::Result<()> {
    outputs.insert(
        "value".to_string(),
        serde_json::to_vec(&value).into_diagnostic()?,
    );
    Ok(())
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

pub(crate) fn run_builtin(
    task_name: &str,
    inputs: &HashMap<String, Vec<u8>>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let mut outputs = HashMap::new();
    match task_name {
        // i64 operations
        "ieq" => bin_op!(inputs, outputs, ==, i64),
        "ine" => bin_op!(inputs, outputs, !=, i64),
        "ilt_s" => bin_op!(inputs, outputs, <, i64),
        // igt is the legacy name
        "igt_s" | "igt" => bin_op!(inputs, outputs, >, i64),
        "ilte_s" => bin_op!(inputs, outputs, <=, i64),
        "igte_s" => bin_op!(inputs, outputs, >=, i64),
        "imax_s" => bin_method!(inputs, outputs, max, i64),
        "imin_s" => bin_method!(inputs, outputs, min, i64),
        "iadd" => bin_op!(inputs, outputs, +, i64),
        "isub" => bin_op!(inputs, outputs, -, i64),
        "ineg" => un_op!(inputs, outputs, -, i64),
        // itimes is the legacy name
        "imul" | "itimes" => bin_op!(inputs, outputs, *, i64),
        // idivide is the legacy name
        "idiv_s" | "idivide" => bin_op!(inputs, outputs, /, i64),
        // mod is the legacy name
        "imod_s" | "mod" => bin_op!(inputs, outputs, %, i64),
        "iabs" => un_method!(inputs, outputs, abs, i64),
        "iand" => bin_op!(inputs, outputs, &, i64),
        "ior" => bin_op!(inputs, outputs, |, i64),
        "ixor" => bin_op!(inputs, outputs, ^, i64),
        "itostring_s" => un_method!(inputs, outputs, to_string, i64),

        // f64 operations
        #[allow(clippy::float_cmp)]
        "feq" => bin_op!(inputs, outputs, ==, f64),
        #[allow(clippy::float_cmp)]
        "fne" => bin_op!(inputs, outputs, !=, f64),
        "flt" => bin_op!(inputs, outputs, <, f64),
        // igt is the legacy name
        "fgt" => bin_op!(inputs, outputs, >, f64),
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

        // "Value" operations.
        "eq" => bin_op!(inputs, outputs, ==, Value),
        "ne" | "neq" => bin_op!(inputs, outputs, !=, Value),

        // Legacy built-in name.
        "str" | "tkr_str" => {
            let value: Value = extract_value(inputs, "value")?;

            output_value(&mut outputs, value.to_string())?;
        }
        "str_eq" => bin_op!(inputs, outputs, ==, String),
        // Legacy built-in name.
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
            let count: i32 = values.len().try_into().into_diagnostic()?;
            let sum: f64 = values.iter().sum();
            let out: f64 = if count > 0 {
                sum / f64::from(count)
            } else {
                return Err(miette!("Cannot obtain `mean` of an empty list"));
            };

            output_value(&mut outputs, out)?;
        }
        "range" | "tkr_range" => {
            range_impl(inputs, &mut outputs)?;
        }
        "conjugate" => {
            let z: Complex64 = extract_value(inputs, "z")?;

            output_value(&mut outputs, z.conj())?;
        }
        "sleep" => {
            let delay_seconds: f64 = extract_value(inputs, "delay_seconds")?;
            sleep(Duration::from_secs_f64(delay_seconds));

            output_value(&mut outputs, true)?;
        }
        "rand_int" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            let value = rand::random_range(a..=b);
            output_value(&mut outputs, value)?;
        }
        "sum_list" | "prod_list" => {
            sum_list_impl(task_name, inputs, &mut outputs)?;
        }
        task => return Err(miette!("Unknown task: `{task}`")),
    }
    Ok(outputs)
}

fn sum_list_impl(
    task_name: &str,
    inputs: &HashMap<String, Vec<u8>>,
    outputs: &mut HashMap<String, Vec<u8>>,
) -> Result<(), miette::Error> {
    let values: Vec<Value> = extract_value(inputs, "values")?;
    let product = task_name == "prod_list";
    let all_integer = values.iter().all(|value| value.as_i64().is_some());
    let _: () = if all_integer {
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
        output_value(outputs, value)?;
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
        output_value(outputs, value)?;
    };
    Ok(())
}

fn range_impl(
    inputs: &HashMap<String, Vec<u8>>,
    outputs: &mut HashMap<String, Vec<u8>>,
) -> Result<(), miette::Error> {
    let start: usize = extract_value(inputs, "start")?;
    let stop: usize = extract_value(inputs, "stop")?;
    let range_step: usize = extract_value_with_default(inputs, "step", 1)?;

    let range = start..stop;
    let out = range.step_by(range_step).collect::<Vec<_>>();

    output_value(outputs, out)?;
    Ok(())
}
