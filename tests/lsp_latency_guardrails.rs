use kujo::lsp_completion;
use kujo::lsp_diagnostics;
use kujo::lsp_hover;
use std::time::{Duration, Instant};

fn representative_source() -> String {
    let mut lines = vec![
        "func compute_total(values) {".to_string(),
        "    let sum := 0".to_string(),
        "    for value in values {".to_string(),
        "        sum := sum + value".to_string(),
        "    }".to_string(),
        "    return sum".to_string(),
        "}".to_string(),
    ];

    for index in 0..1200 {
        lines.push(format!("let value_{index} := {index}"));
    }

    lines.push("let result := compute_total([1, 2, 3, 4])".to_string());
    lines.push("pri".to_string());
    lines.join("\n")
}

fn average_duration<F>(iterations: usize, mut op: F) -> Duration
where
    F: FnMut(),
{
    let start = Instant::now();
    for _ in 0..iterations {
        op();
    }

    let total = start.elapsed();
    let average_nanos = total.as_nanos() / (iterations as u128);
    let average_nanos_u64 = average_nanos.min(u128::from(u64::MAX)) as u64;
    Duration::from_nanos(average_nanos_u64)
}

#[test]
fn latency_guardrails_for_completion_diagnostics_and_hover() {
    let source = representative_source();

    let completion_avg = average_duration(20, || {
        let _ = lsp_completion::complete(&source, 1209, 4);
    });

    let diagnostics_avg = average_duration(20, || {
        let _ = lsp_diagnostics::diagnose(&source);
    });

    let hover_avg = average_duration(20, || {
        let _ = lsp_hover::hover(&source, 1208, 17);
    });

    // Conservative guardrails to catch severe regressions while staying stable on loaded CI hosts.
    assert!(
        completion_avg.as_millis() < 120,
        "completion average latency exceeded guardrail: {completion_avg:?}"
    );
    assert!(
        diagnostics_avg.as_millis() < 120,
        "diagnostics average latency exceeded guardrail: {diagnostics_avg:?}"
    );
    assert!(hover_avg.as_millis() < 120, "hover average latency exceeded guardrail: {hover_avg:?}");
}
