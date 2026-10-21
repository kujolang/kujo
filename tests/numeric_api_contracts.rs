#[path = "support/native_api.rs"]
mod support;
use support::{failure, success};

#[test]
fn fixed_decimal_formatting_is_exact_and_locale_independent() {
    success(
        r#"
    assert_equal(format_fixed(1.25, 7), "1.2500000")
    assert_equal(format_fixed(1.0 / 8.0, 10), "0.1250000000")
    assert_equal(format_fixed(2.5, 0), "2")
    assert_equal(format_fixed(3.5, 0), "4")
    assert_equal(format_fixed(-0.0, 2), "-0.00")
    assert_equal(format_fixed(-0.001, 2), "-0.00")
    assert_equal(format_fixed(9223372036854775807, 2), "9223372036854775807.00")
    assert_equal(format_fixed(-9223372036854775807 - 1, 0), "-9223372036854775808")
    let f := format_fixed
    assert_equal(f(1, 2), "1.00")
    assert_equal(format("%s %d %f", "x", 2, 1.5), "x 2 1.5")
    "#,
    );
}

#[test]
fn fixed_decimal_rejects_invalid_arguments() {
    for source in [
        "format_fixed(1, -1)",
        "format_fixed(1, 101)",
        "format_fixed(1, 1.5)",
        "format_fixed(\"x\", 2)",
    ] {
        failure(source, "format_fixed");
    }
    failure("format_fixed(1)", "argument");
    failure("format_fixed(to_float(\"NaN\"), 2)", "finite");
    failure("format_fixed(to_float(\"inf\"), 2)", "finite");
}

#[test]
fn numeric_names_and_arities_are_registered() {
    for name in ["format_fixed", "wrapping_add", "wrapping_sub", "wrapping_mul"] {
        assert!(kujo::interpreter::Interpreter::is_builtin_name(name));
        let arity = kujo::interpreter::Interpreter::native_function_arity(name).unwrap();
        assert!(arity.validate(2).is_ok());
        assert!(arity.validate(1).is_err());
        assert!(arity.validate(3).is_err());
    }
}

#[test]
fn explicit_wrapping_preserves_checked_ordinary_arithmetic() {
    success(
        r#"
    let max := 9223372036854775807
    let min := -max - 1
    assert_equal(wrapping_add(max, 1), min)
    assert_equal(wrapping_sub(min, 1), max)
    assert_equal(wrapping_mul(min, -1), min)
    assert_equal(wrapping_mul(max, max), 1)
    assert_equal(wrapping_mul(4294967296, 4294967296), 0)
    assert_equal(wrapping_sub(0, 1), -1)
    let add := wrapping_add
    assert_equal(add(7, 8), 15)
    assert_equal(bit_and(wrapping_mul(4294967295, 4294967295), 4294967295), 1)
    "#,
    );
    failure("9223372036854775807 + 1", "overflow");
    failure("wrapping_add(1.0, 2)", "integer");
    failure("wrapping_mul(1)", "argument");
}
