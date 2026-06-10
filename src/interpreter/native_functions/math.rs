// File: src/interpreter/native_functions/math.rs
//
// Math-related native functions

use crate::builtins;
use crate::interpreter::Value;

fn number_arg(name: &str, arg_name: &str, value: &Value) -> Result<f64, Value> {
    match value {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        _ => Err(Value::Error(format!(
            "{}() expects numeric argument '{}' , got {:?}",
            name, arg_name, value
        ))),
    }
}

fn int_arg(name: &str, arg_name: &str, value: &Value) -> Result<i64, Value> {
    match value {
        Value::Int(n) => Ok(*n),
        _ => Err(Value::Error(format!(
            "{}() expects integer argument '{}' , got {:?}",
            name, arg_name, value
        ))),
    }
}

/// Handle math-related function calls  
/// Returns Some(value) if the function was handled, None if not recognized
pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        // Math functions - single argument
        "abs" | "sqrt" | "floor" | "ceil" | "round" | "sin" | "cos" | "tan" | "log" | "exp" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!("{}() expects 1 argument", name)));
            }

            let x = match number_arg(name, "value", &arg_values[0]) {
                Ok(value) => value,
                Err(error) => return Some(error),
            };

            if "sqrt" == name && x < 0.0 {
                return Some(Value::Error(format!("{}() domain error: value must be >= 0", name)));
            }

            if "log" == name && x <= 0.0 {
                return Some(Value::Error(format!("{}() domain error: value must be > 0", name)));
            }

            let result = match name {
                "abs" => builtins::abs(x),
                "sqrt" => builtins::sqrt(x),
                "floor" => builtins::floor(x),
                "ceil" => builtins::ceil(x),
                "round" => builtins::round(x),
                "sin" => builtins::sin(x),
                "cos" => builtins::cos(x),
                "tan" => builtins::tan(x),
                "log" => builtins::log(x),
                "exp" => builtins::exp(x),
                _ => 0.0,
            };
            Value::Float(result)
        }

        // Math functions - two arguments
        "pow" | "min" | "max" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!("{}() expects 2 arguments", name)));
            }

            let a = match number_arg(name, "a", &arg_values[0]) {
                Ok(value) => value,
                Err(_) => {
                    return Some(Value::Error(format!("{}() expects numeric arguments", name)))
                }
            };
            let b = match number_arg(name, "b", &arg_values[1]) {
                Ok(value) => value,
                Err(_) => {
                    return Some(Value::Error(format!("{}() expects numeric arguments", name)))
                }
            };
            let result = match name {
                "pow" => builtins::pow(a, b),
                "min" => builtins::min(a, b),
                "max" => builtins::max(a, b),
                _ => 0.0,
            };
            Value::Float(result)
        }

        "bit_and" | "bit_or" | "bit_xor" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!("{}() expects 2 arguments", name)));
            }

            let left = match int_arg(name, "left", &arg_values[0]) {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let right = match int_arg(name, "right", &arg_values[1]) {
                Ok(value) => value,
                Err(error) => return Some(error),
            };

            let result = match name {
                "bit_and" => left & right,
                "bit_or" => left | right,
                "bit_xor" => left ^ right,
                _ => unreachable!(),
            };
            Value::Int(result)
        }

        "bit_not" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("bit_not() expects 1 argument".to_string()));
            }

            let value = match int_arg(name, "value", &arg_values[0]) {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            Value::Int(!value)
        }

        "bit_shl" | "bit_shr" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!("{}() expects 2 arguments", name)));
            }

            let left = match int_arg(name, "left", &arg_values[0]) {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let shift = match int_arg(name, "right", &arg_values[1]) {
                Ok(value) => value,
                Err(error) => return Some(error),
            };

            let amount = match u32::try_from(shift) {
                Ok(value) if value < 64 => value,
                _ => {
                    return Some(Value::Error(format!(
                        "{}() expects a shift amount between 0 and 63",
                        name
                    )));
                }
            };

            let result = match name {
                "bit_shl" => left << amount,
                "bit_shr" => left >> amount,
                _ => unreachable!(),
            };
            Value::Int(result)
        }

        _ => return None,
    };
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::interpreter::Value;

    #[test]
    fn test_math_api_strict_arity_rejects_extra_arguments() {
        let abs_extra =
            handle("abs", &[Value::Int(-7), Value::Int(1)]).expect("abs should return a result");
        assert!(
            matches!(abs_extra, Value::Error(message) if message.contains("abs() expects 1 argument"))
        );

        let sqrt_extra =
            handle("sqrt", &[Value::Int(9), Value::Int(1)]).expect("sqrt should return a result");
        assert!(
            matches!(sqrt_extra, Value::Error(message) if message.contains("sqrt() expects 1 argument"))
        );

        let exp_extra =
            handle("exp", &[Value::Int(1), Value::Int(2)]).expect("exp should return a result");
        assert!(
            matches!(exp_extra, Value::Error(message) if message.contains("exp() expects 1 argument"))
        );

        let pow_extra = handle("pow", &[Value::Int(2), Value::Int(8), Value::Int(1)])
            .expect("pow should return a result");
        assert!(
            matches!(pow_extra, Value::Error(message) if message.contains("pow() expects 2 arguments"))
        );

        let min_extra = handle("min", &[Value::Int(1), Value::Int(2), Value::Int(3)])
            .expect("min should return a result");
        assert!(
            matches!(min_extra, Value::Error(message) if message.contains("min() expects 2 arguments"))
        );

        let max_extra = handle("max", &[Value::Int(1), Value::Int(2), Value::Int(3)])
            .expect("max should return a result");
        assert!(
            matches!(max_extra, Value::Error(message) if message.contains("max() expects 2 arguments"))
        );
    }

    #[test]
    fn test_math_api_rejects_missing_and_invalid_arguments() {
        let abs_missing = handle("abs", &[]).expect("abs should return a result");
        assert!(
            matches!(abs_missing, Value::Error(message) if message.contains("abs() expects 1 argument"))
        );

        let pow_missing = handle("pow", &[Value::Int(2)]).expect("pow should return a result");
        assert!(
            matches!(pow_missing, Value::Error(message) if message.contains("pow() expects 2 arguments"))
        );

        let pow_invalid = handle("pow", &[Value::Int(2), Value::Str("bad".to_string().into())])
            .expect("pow should return a result");
        assert!(
            matches!(pow_invalid, Value::Error(message) if message.contains("pow() expects numeric arguments"))
        );
    }

    #[test]
    fn test_math_api_domain_and_success_contracts() {
        let sqrt_negative = handle("sqrt", &[Value::Int(-1)]).expect("sqrt should return a result");
        assert!(
            matches!(sqrt_negative, Value::Error(message) if message.contains("sqrt() domain error"))
        );

        let log_zero = handle("log", &[Value::Int(0)]).expect("log should return a result");
        assert!(
            matches!(log_zero, Value::Error(message) if message.contains("log() domain error"))
        );

        let sqrt_success = handle("sqrt", &[Value::Int(9)]).expect("sqrt should return a result");
        assert!(matches!(sqrt_success, Value::Float(value) if (value - 3.0).abs() < f64::EPSILON));

        let pow_success =
            handle("pow", &[Value::Int(2), Value::Int(8)]).expect("pow should return a result");
        assert!(matches!(pow_success, Value::Float(value) if (value - 256.0).abs() < f64::EPSILON));
    }

    #[test]
    fn test_bitwise_api_contracts() {
        let bit_and = handle("bit_and", &[Value::Int(0b1100), Value::Int(0b1010)])
            .expect("bit_and should return a result");
        assert!(matches!(bit_and, Value::Int(value) if value == 0b1000));

        let bit_or = handle("bit_or", &[Value::Int(0b1100), Value::Int(0b1010)])
            .expect("bit_or should return a result");
        assert!(matches!(bit_or, Value::Int(value) if value == 0b1110));

        let bit_xor = handle("bit_xor", &[Value::Int(0b1100), Value::Int(0b1010)])
            .expect("bit_xor should return a result");
        assert!(matches!(bit_xor, Value::Int(value) if value == 0b0110));

        let bit_not = handle("bit_not", &[Value::Int(0)]).expect("bit_not should return a result");
        assert!(matches!(bit_not, Value::Int(value) if value == -1));

        let bit_shl = handle("bit_shl", &[Value::Int(3), Value::Int(4)])
            .expect("bit_shl should return a result");
        assert!(matches!(bit_shl, Value::Int(value) if value == 48));

        let bit_shr = handle("bit_shr", &[Value::Int(16), Value::Int(2)])
            .expect("bit_shr should return a result");
        assert!(matches!(bit_shr, Value::Int(value) if value == 4));

        let bit_shr_bad = handle("bit_shr", &[Value::Int(16), Value::Int(64)])
            .expect("bit_shr should return a result");
        assert!(
            matches!(bit_shr_bad, Value::Error(message) if message.contains("shift amount between 0 and 63"))
        );
    }
}
