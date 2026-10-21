#[path = "support/native_api.rs"]
mod support;

fn source(include_main: bool) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/langarena/src");
    let mut paths: Vec<_> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    paths
        .into_iter()
        .filter(|p| include_main || p.file_name().unwrap() != "99_main.kujo")
        .map(|p| std::fs::read_to_string(p).unwrap() + "\n")
        .collect()
}

#[test]
fn registry_covers_exactly_the_fifty_pinned_benchmarks() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../benchmarks/langarena/test.json")).unwrap();
    let cases = config.as_array().unwrap();
    assert_eq!(cases.len(), 50);
    let mut script = source(false);
    script += "assert_equal(len(registry), 50)\n";
    for case in cases {
        script += &format!("assert_equal(has_key(registry, {}), 1)\n", case["name"]);
    }
    support::success(&script);
}

#[test]
fn port_kernels_preserve_input_values_and_roundtrip_binary_data() {
    let mut script = source(false);
    script += r#"
        assert_equal(quick_sort([3, 1, 2, 1], 0, 3), [1, 1, 2, 3])
        let original := [3, 1, 2, 1]
        let merged := merge_sort([original, vector(4, 0)], 0, 3)
        assert_equal(merged[0], [1, 1, 2, 3])
        assert_equal(original, [3, 1, 2, 1])
        assert_equal(bf_array("++[>+<-]>.") , 2)
        assert_equal(bf_text({"name":"Brainfuck::Recursion"}, "++[>+<-]>."), 2)
        for data in [bytes([0, 255, 128, 0, 255]), utf8_encode("ABRACADABRA"), bytes([65])] {
            assert_equal(bwt_decode(bwt_encode(data)), data)
            assert_equal(huff_decode(huff_encode(data)), data)
            assert_equal(arith_decode(arith_encode(data)), data)
        }
        let text := utf8_encode("ABRACADABRA")
        assert_equal(lzw_decode(lzw_encode(text)), text)
        let ast := calc_parse("a = 8\nb = a * 3 - 2\n")
        let first := calc_evaluate(ast[0], {})
        let second := calc_evaluate(ast[1], first[1])
        assert_equal(second[0], 22)
        assert_equal(has_key(first[1], "b"), 0)
        assert_equal(jaro("MARTHA", "MARHTA") > 0.9, true)
    "#;
    support::success(&script);
}

#[test]
#[ignore = "full upstream checksum suite; run explicitly with an optimized Kujo binary"]
fn all_fifty_upstream_checksums_in_both_runtimes() {
    let dir = tempfile::tempdir().unwrap();
    let port = dir.path().join("port.kujo");
    std::fs::write(&port, source(true)).unwrap();
    let binary = std::env::var_os("KUJO_LANGARENA_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_kujo").into());
    let config =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/langarena/test.json");
    for interpreter in [false, true] {
        let mut command = std::process::Command::new(&binary);
        command.arg("run").arg(&port).args(["--scheduler-timeout-ms", "3600000"]);
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.arg(&config).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Runtime: {}\n{stdout}", if interpreter { "interpreter" } else { "vm" });
        assert!(
            output.status.success(),
            "interpreter={interpreter}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(stdout.contains("Summary: 50, 50, 0"), "{stdout}");
        assert_eq!(stdout.lines().filter(|line| line.contains(": OK ")).count(), 50);
    }
}
