#[path = "support/native_api.rs"]
mod support;
use support::{failure, success};

#[test]
fn csv_rows_enforces_resource_limits() {
    assert!(kujo::builtins::parse_csv_rows(&"x".repeat(8 * 1024 * 1024 + 1))
        .unwrap_err()
        .contains("bytes"));
    assert!(kujo::builtins::parse_csv_rows(&",".repeat(1_000_000)).unwrap_err().contains("fields"));
    let arity = kujo::interpreter::Interpreter::native_function_arity("parse_csv_rows").unwrap();
    assert!(arity.validate(1).is_ok());
    assert!(arity.validate(0).is_err());
}

#[test]
fn positional_csv_retains_every_row_and_exact_text() {
    success(
        r#"
    assert_equal(parse_csv_rows("1,2\n3,4\n"), [["1", "2"], ["3", "4"]])
    assert_equal(parse_csv_rows("001,1.20,\r\n002,3.40,\r\n"), [["001","1.20",""],["002","3.40",""]])
    assert_equal(parse_csv_rows("\"a,b\",\"say \"\"hi\"\"\",\"a\nb\"\r\n"), [["a,b","say \"hi\"","a\nb"]])
    assert_equal(parse_csv_rows(""), [])
    assert_equal(parse_csv_rows("\"\""), [[""]])
    let parse := parse_csv_rows
    assert_equal(parse("é,雪"), [["é", "雪"]])
    assert_equal(parse_csv("x,y\n1,2\n"), [{"x":1,"y":2}])
    "#,
    );
}

#[test]
fn malformed_csv_and_bad_arguments_fail_in_both_runtimes() {
    for input in ["\"unclosed", "a\"b,c", "\"x\"oops,y", "a,b\nc", "a,b\nc,d,e"] {
        let literal = serde_json::to_string(input).unwrap();
        failure(&format!("parse_csv_rows({literal})"), "CSV");
    }
    failure("parse_csv_rows(1)", "string");
    failure("parse_csv_rows()", "argument");
    failure("parse_csv_rows(\"a\", 1)", "argument");
}
