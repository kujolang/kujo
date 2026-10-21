#[path = "support/native_api.rs"]
mod support;

#[test]
fn consuming_bytes_keep_aliases_closures_and_nested_snapshots() {
    support::success(
        r#"
        struct State { data: bytes }
        func work() {
            let join := concat
            mut b := utf8_encode("a")
            let old := b
            let snapshot := func() { return b }
            b = join(b, utf8_encode("b"))
            assert_equal(utf8_decode(b), "ab")
            assert_equal(utf8_decode(old), "a")
            assert_equal(utf8_decode(snapshot()), "a")
            b = concat(b, b)
            assert_equal(utf8_decode(b), "abab")
            mut state := State { data: b }
            let saved := state
            state.data = concat(state.data, utf8_encode("!"))
            assert_equal(utf8_decode(saved.data), "abab")
            assert_equal(utf8_decode(state.data), "abab!")
            mut rows := {"rows": [b]}
            let before := rows
            rows["rows"][-1] = concat(rows["rows"][-1], utf8_encode("?"))
            assert_equal(utf8_decode(rows["rows"][0]), "abab?")
            assert_equal(utf8_decode(before["rows"][0]), "abab")
            let frozen := state
            mut caught := false
            try { frozen.data = concat(frozen.data, b) } except err { caught = true }
            assert(caught)
            assert_equal(utf8_decode(frozen.data), "abab!")
            mut arrays := [[1]]
            arrays[0] = concat(arrays[0], [2])
            assert_equal(arrays, [[1, 2]])
            let custom := func(a, b) { return utf8_encode("custom") }
            state.data = custom(state.data, b)
            assert_equal(utf8_decode(state.data), "custom")
        }
        work()
    "#,
    );
}

#[test]
fn consuming_bytes_preserve_argument_and_destination_effects() {
    support::success(
        r#"
        mut rows := [utf8_encode("a"), utf8_encode("b")]
        mut calls := 0
        func index() { let old := calls calls += 1 return old }
        func work() {
            rows[index()] = concat(rows[index()], utf8_encode("!"))
            assert_equal(calls, 2)
            assert_equal(utf8_decode(rows[0]), "a")
            assert_equal(utf8_decode(rows[1]), "a!")
        }
        work()
        mut b := utf8_encode("old")
        func suffix() { b = utf8_encode("replaced") return utf8_encode("+") }
        func change() { b = concat(b, suffix()) }
        change()
        assert_equal(utf8_decode(b), "old+")
        mut caught := false
        try { rows[index()] = concat(rows[0], 1) } except err { caught = true }
        assert(caught)
        assert_equal(calls, 2)
    "#,
    );
}

#[test]
fn large_byte_growth_uses_the_same_contract_in_both_runtimes() {
    support::success(
        r#"
        func work() {
            let chunk := utf8_encode(repeat("x", 4096))
            mut b := bytes([])
            mut i := 0
            while i < 768 { b = concat(b, chunk) i += 1 }
            assert_equal(byte_length(b), 3145728)
            b[-1] = 0
            assert_equal(b[0], 120)
            assert_equal(b[-1], 0)
            assert_equal(chunk[-1], 120)
        }
        work()
    "#,
    );
}

#[test]
#[ignore = "manual byte growth scaling measurements"]
fn profile_byte_growth() {
    let binary = std::env::var("KUJO_PROFILE_BINARY")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_kujo").to_string());
    for interpreter in [false, true] {
        for n in [16_000, 64_000] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("growth.kujo");
            std::fs::write(
                &path,
                format!(
                    r#"
                func work() {{
                    let chunk := utf8_encode("0123456789abcdef")
                    mut b := bytes([])
                    let start := time_ns()
                    mut i := 0
                    while i < {n} {{ b = concat(b, chunk) i += 1 }}
                    assert_equal(byte_length(b), {size})
                    assert_equal(b[-1], 102)
                    print(time_ns() - start)
                }}
                work()
            "#,
                    size = n * 16
                ),
            )
            .unwrap();
            let mut samples = Vec::new();
            for _ in 0..3 {
                let mut command = std::process::Command::new(&binary);
                command.arg("run").arg(&path).arg("--allow-clock");
                if interpreter {
                    command.arg("--interpreter");
                }
                let output = command.output().unwrap();
                assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
                samples
                    .push(String::from_utf8(output.stdout).unwrap().trim().parse::<u64>().unwrap());
            }
            samples.sort_unstable();
            println!(
                "bytes_concat,runtime={},n={n},median_ns={}",
                if interpreter { "interpreter" } else { "vm" },
                samples[1]
            );
        }
    }
}
