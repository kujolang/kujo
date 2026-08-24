//! Fixture-first ecosystem evidence runner.
//!
//! The runner deliberately owns only orchestration and evidence accounting. It
//! does not reimplement AI SDK, Dispatch, Workcell, Eval, or ShipCheck logic.

use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const SCHEMA: &str = "kujo-ecosystem-golden-path/v1";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageResult {
    pub id: String,
    pub status: StageStatus,
    pub evidence: String,
    pub command: Option<String>,
    pub working_directory: Option<String>,
    pub duration_ms: u128,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub reason: Option<String>,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactEntry {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct GoldenPathReport {
    schema: &'static str,
    kujo_version: &'static str,
    repository_root: String,
    output_root: String,
    allow_blocked: bool,
    evidence_policy: &'static str,
    stages: Vec<StageResult>,
    artifact_manifest: String,
    artifact_manifest_sha256: String,
    failed_stages: usize,
    blocked_stages: usize,
    ok: bool,
}

#[derive(Debug)]
struct ProcessCapture {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    duration: Duration,
}

#[derive(Debug)]
struct StageCommand<'a> {
    id: &'a str,
    evidence: &'a str,
    program: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    timeout: Duration,
    blocked_on_timeout: bool,
    blocked_on_failure: bool,
    env: Vec<(&'a str, &'a str)>,
}

pub fn run(
    repo_root: &Path,
    output_root: Option<&Path>,
    timeout: Duration,
    allow_blocked: bool,
    json_output: bool,
) -> i32 {
    let repo_root = match repo_root.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("golden path: repository root is unavailable: {error}");
            return 5;
        }
    };

    let output_root = match prepare_output_root(output_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("golden path: {error}");
            return 5;
        }
    };

    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("golden path: cannot locate Kujo executable: {error}");
            return 6;
        }
    };
    let mut stages = Vec::new();
    stages.push(write_identity_stage(&output_root, &current_exe));

    let kujo_args = |script: &Path| vec!["run".to_string(), script.to_string_lossy().into_owned()];
    let stage = |id: &'static str,
                 evidence: &'static str,
                 script: PathBuf,
                 cwd: PathBuf,
                 blocked_on_timeout: bool| {
        StageCommand {
            id,
            evidence,
            program: current_exe.clone(),
            args: kujo_args(&script),
            cwd,
            timeout,
            blocked_on_timeout,
            blocked_on_failure: false,
            env: Vec::new(),
        }
    };

    let ai_sdk = repo_root.join("ai-sdk");
    stages.push(run_script_stage(
        &output_root,
        stage("ai-sdk", "fixture", ai_sdk.join("examples/main.kujo"), ai_sdk, false),
    ));

    let agents_sdk = repo_root.join("agents-sdk");
    stages.push(run_script_stage(
        &output_root,
        stage(
            "agents-sdk",
            "fixture",
            agents_sdk.join("examples/examples_smoke_runner.kujo"),
            agents_sdk,
            false,
        ),
    ));

    let dispatch = repo_root.join("dispatch");
    let mut dispatch_stage =
        stage("dispatch", "fixture", dispatch.join("dispatch.kujo"), dispatch, true);
    dispatch_stage.args.extend([
        "demo".to_string(),
        "Golden path fixture".to_string(),
        "--yes".to_string(),
        "--non-interactive".to_string(),
    ]);
    dispatch_stage.env.push(("DISPATCH_OFFLINE_FIXTURE", "true"));
    stages.push(run_script_stage(&output_root, dispatch_stage));

    stages.push(run_workcell_stage(&repo_root, &output_root, timeout));

    let watchdog = repo_root.join("watchdog");
    stages.push(run_external_stage(
        &output_root,
        StageCommand {
            id: "watchdog",
            evidence: "fixture",
            program: PathBuf::from("node"),
            args: vec!["tests/telemetry_redaction_check.js".to_string()],
            cwd: watchdog,
            timeout,
            blocked_on_timeout: false,
            blocked_on_failure: true,
            env: Vec::new(),
        },
    ));

    let eval = repo_root.join("eval");
    let mut eval_args = kujo_args(&eval.join("main.kujo"));
    eval_args.extend([
        "run".to_string(),
        "examples/release_gate_suite.json".to_string(),
        "--output-dir".to_string(),
        output_root.join("eval").to_string_lossy().into_owned(),
        "--artifact-checksums".to_string(),
        "--json".to_string(),
    ]);
    stages.push(run_external_stage(
        &output_root,
        StageCommand {
            id: "eval",
            evidence: "fixture",
            program: current_exe.clone(),
            args: eval_args,
            cwd: eval,
            timeout,
            blocked_on_timeout: false,
            blocked_on_failure: false,
            env: Vec::new(),
        },
    ));

    let shipcheck = repo_root.join("shipcheck");
    stages.push(run_script_stage(&output_root, {
        let mut shipcheck_stage =
            stage("shipcheck", "local_real", shipcheck.join("shipcheck.kujo"), shipcheck, false);
        shipcheck_stage.args.extend([
            "gate".to_string(),
            "--dir".to_string(),
            ".".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ]);
        shipcheck_stage
    }));

    stages.push(run_runledger_stage(&repo_root, &output_root, timeout, &stages));
    stages.push(write_casefile_reference(&output_root, &stages));

    let handoff_path = output_root.join("evidence-handoff.json");
    let handoff = json!({
        "schema": "kujo-ecosystem-evidence-handoff/v1",
        "evidence_policy": "fixture-first; external-live claims are forbidden",
        "runledger": "stages/runledger/ledger",
        "casefile_reference": "stages/casefile-reference/reference.json",
        "stages": stages.iter().map(|item| json!({
            "id": item.id,
            "status": item.status,
            "evidence": item.evidence,
            "result": format!("stages/{}/result.json", item.id),
            "artifacts": item.artifacts,
        })).collect::<Vec<_>>(),
    });
    if let Err(error) = write_json(&handoff_path, &handoff) {
        eprintln!("golden path: cannot write evidence handoff: {error}");
        return 5;
    }
    stages.push(StageResult {
        id: "evidence-handoff".to_string(),
        status: StageStatus::Passed,
        evidence: "local_real".to_string(),
        command: None,
        working_directory: None,
        duration_ms: 0,
        exit_code: Some(0),
        timed_out: false,
        reason: Some("local evidence-handoff.json references every stage result".to_string()),
        artifacts: vec![relative_path(&output_root, &handoff_path)],
    });

    let artifact_entries = match collect_artifacts(&output_root) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("golden path: cannot hash evidence artifacts: {error}");
            return 5;
        }
    };
    let manifest_path = output_root.join("artifacts.json");
    if let Err(error) = write_json(
        &manifest_path,
        &json!({
            "schema": "kujo-ecosystem-artifacts/v1",
            "artifacts": artifact_entries,
        }),
    ) {
        eprintln!("golden path: cannot write artifact manifest: {error}");
        return 5;
    }
    let manifest_hash = match sha256_file(&manifest_path) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("golden path: cannot hash artifact manifest: {error}");
            return 5;
        }
    };

    let failed_stages = stages.iter().filter(|stage| stage.status == StageStatus::Failed).count();
    let blocked_stages = stages.iter().filter(|stage| stage.status == StageStatus::Blocked).count();
    let ok = report_ok(failed_stages, blocked_stages, allow_blocked);
    let report = GoldenPathReport {
        schema: SCHEMA,
        kujo_version: env!("CARGO_PKG_VERSION"),
        repository_root: repo_root.display().to_string(),
        output_root: output_root.display().to_string(),
        allow_blocked,
        evidence_policy: "fixture-first; no live credentials or external certification claims",
        stages,
        artifact_manifest: relative_path(&output_root, &manifest_path),
        artifact_manifest_sha256: manifest_hash,
        failed_stages,
        blocked_stages,
        ok,
    };
    if let Err(error) = write_json(&output_root.join("golden-path.json"), &report) {
        eprintln!("golden path: cannot write report: {error}");
        return 5;
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()));
    } else {
        println!("Golden path: {}", if ok { "passed" } else { "blocked or failed" });
        println!("Evidence root: {}", output_root.display());
        println!(
            "Stages: {} passed, {} blocked, {} failed",
            report.stages.iter().filter(|s| s.status == StageStatus::Passed).count(),
            blocked_stages,
            failed_stages
        );
    }
    if ok {
        0
    } else {
        4
    }
}

fn prepare_output_root(requested: Option<&Path>) -> Result<PathBuf, String> {
    let path = requested.map(PathBuf::from).unwrap_or_else(|| {
        std::env::temp_dir().join(format!("kujo-golden-path-{}", Uuid::new_v4()))
    });
    if path.exists() {
        let mut entries = fs::read_dir(&path)
            .map_err(|error| format!("cannot inspect output root '{}': {error}", path.display()))?;
        if entries.next().is_some() {
            return Err(format!("output root '{}' must be new and empty", path.display()));
        }
    } else {
        fs::create_dir_all(&path)
            .map_err(|error| format!("cannot create output root '{}': {error}", path.display()))?;
    }
    Ok(path)
}

fn write_identity_stage(output_root: &Path, current_exe: &Path) -> StageResult {
    let stage_dir = output_root.join("stages/runtime");
    let _ = fs::create_dir_all(&stage_dir);
    let identity = json!({
        "stage": "runtime",
        "kujo_version": env!("CARGO_PKG_VERSION"),
        "executable": current_exe,
        "evidence": "local_real",
    });
    let result_path = stage_dir.join("result.json");
    let status = if write_json(&result_path, &identity).is_ok() {
        StageStatus::Passed
    } else {
        StageStatus::Failed
    };
    StageResult {
        id: "runtime".to_string(),
        status,
        evidence: "local_real".to_string(),
        command: Some(format!("{} --version", current_exe.display())),
        working_directory: std::env::current_dir().ok().map(|path| path.display().to_string()),
        duration_ms: 0,
        exit_code: Some(0),
        timed_out: false,
        reason: None,
        artifacts: vec![relative_path(output_root, &result_path)],
    }
}

fn run_script_stage(output_root: &Path, command: StageCommand<'_>) -> StageResult {
    if !command.cwd.is_dir()
        || !command.args.get(1).map(|path| Path::new(path).is_file()).unwrap_or(true)
    {
        return blocked_stage(
            output_root,
            command.id,
            command.evidence,
            "required repository or script is missing",
        );
    }
    run_external_stage(output_root, command)
}

fn run_workcell_stage(repo_root: &Path, output_root: &Path, timeout: Duration) -> StageResult {
    let workcell = repo_root.join("workcell");
    let binary = workcell.join("bin/workcell");
    if !binary.is_file() {
        return blocked_stage(output_root, "workcell", "local_real", "Workcell CLI is missing");
    }
    let doctor =
        run_process(&binary, &["doctor", "--backend", "docker", "--json"], &workcell, timeout, &[]);
    if doctor.timed_out || doctor.exit_code != Some(0) {
        let reason = if doctor.timed_out {
            "Docker doctor timed out"
        } else {
            "Docker backend is unavailable or failed doctor"
        };
        return write_process_stage(
            output_root,
            "workcell",
            "local_real",
            &workcell,
            "workcell doctor --backend docker --json".to_string(),
            doctor,
            StageStatus::Blocked,
            reason,
        );
    }
    let stage_output = output_root.join("stages/workcell/output");
    let _ = fs::create_dir_all(&stage_output);
    let args = vec![
        "run".to_string(),
        "--file".to_string(),
        "examples/hello/workcell.json".to_string(),
        "--repo".to_string(),
        workcell.display().to_string(),
        "--output".to_string(),
        stage_output.display().to_string(),
        "--no-pull".to_string(),
        "--json".to_string(),
    ];
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = run_process(&binary, &arg_refs, &workcell, timeout, &[]);
    let status = if result.timed_out {
        StageStatus::Blocked
    } else if result.exit_code == Some(0) {
        StageStatus::Passed
    } else {
        StageStatus::Failed
    };
    write_process_stage(
        output_root,
        "workcell",
        "local_real",
        &workcell,
        display_command(&binary, &arg_refs),
        result,
        status,
        "Workcell execution and receipt stage",
    )
}

fn run_runledger_stage(
    repo_root: &Path,
    output_root: &Path,
    timeout: Duration,
    prior_stages: &[StageResult],
) -> StageResult {
    let ledger_repo = repo_root.join("runledger");
    let binary = ledger_repo.join("bin/runledger");
    if !binary.is_file() {
        return blocked_stage(output_root, "runledger", "local_real", "RunLedger CLI is missing");
    }
    let stage_dir = output_root.join("stages/runledger");
    let ledger_dir = stage_dir.join("ledger");
    let _ = fs::create_dir_all(&ledger_dir);
    let ledger_repo_target = if repo_root.join("kujo/.git").exists() {
        repo_root.join("kujo")
    } else {
        repo_root.to_path_buf()
    };
    let repo_arg = ledger_repo_target.to_string_lossy().into_owned();
    let ledger_arg = ledger_dir.to_string_lossy().into_owned();
    let start_args = vec![
        "start",
        "--model",
        "kujo-golden-path",
        "--provider",
        "fixture",
        "--task",
        "ecosystem-golden-path",
        "--repo",
        repo_arg.as_str(),
        "--ledger",
        ledger_arg.as_str(),
    ];
    let start = run_process(&binary, &start_args, &ledger_repo, timeout, &[]);
    let run_id = start
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("Started run: ").map(str::trim))
        .map(str::to_string);
    let Some(run_id) = run_id else {
        let status = if start.timed_out { StageStatus::Blocked } else { StageStatus::Failed };
        return write_process_stage(
            output_root,
            "runledger",
            "local_real",
            &ledger_repo,
            display_command(&binary, &start_args),
            start,
            status,
            "RunLedger start did not produce a run id",
        );
    };

    let prior_failed = prior_stages.iter().any(|stage| stage.status == StageStatus::Failed);
    let prior_blocked = prior_stages.iter().any(|stage| stage.status == StageStatus::Blocked);
    let ledger_status = if prior_failed {
        "fail"
    } else if prior_blocked {
        "partial"
    } else {
        "pass"
    };
    let verdict = if prior_failed {
        "golden path contains failed stages"
    } else if prior_blocked {
        "golden path recorded with explicit blocked stages"
    } else {
        "golden path completed"
    };
    let finish_args = vec![
        "finish",
        run_id.as_str(),
        "--status",
        ledger_status,
        "--verdict",
        verdict,
        "--ledger",
        ledger_arg.as_str(),
    ];
    let finish = run_process(&binary, &finish_args, &ledger_repo, timeout, &[]);
    let process = ProcessCapture {
        exit_code: finish.exit_code,
        stdout: format!("{}\n{}", start.stdout, finish.stdout),
        stderr: format!("{}\n{}", start.stderr, finish.stderr),
        timed_out: start.timed_out || finish.timed_out,
        duration: start.duration + finish.duration,
    };
    let status = if process.timed_out {
        StageStatus::Blocked
    } else if process.exit_code == Some(0) {
        StageStatus::Passed
    } else {
        StageStatus::Failed
    };
    write_process_stage(
        output_root,
        "runledger",
        "local_real",
        &ledger_repo,
        format!(
            "{} && {}",
            display_command(&binary, &start_args),
            display_command(&binary, &finish_args)
        ),
        process,
        status,
        "RunLedger start and finish recorded the golden-path status",
    )
}

fn write_casefile_reference(output_root: &Path, stages: &[StageResult]) -> StageResult {
    let stage_dir = output_root.join("stages/casefile-reference");
    let reference_path = stage_dir.join("reference.json");
    let mut source_artifacts = Vec::new();
    for stage in stages {
        for artifact in &stage.artifacts {
            let path = output_root.join(artifact);
            if let Ok(hash) = sha256_file(&path) {
                source_artifacts.push(json!({
                    "stage": stage.id,
                    "path": artifact,
                    "sha256": hash,
                }));
            }
        }
    }
    let reference = json!({
        "schema": "kujo-ecosystem-casefile-reference/v1",
        "adapter": "local_reference",
        "status": "recorded",
        "external_casefile_invoked": false,
        "reason": "The isolated runner records redacted stage paths and hashes without writing outside its evidence root.",
        "source_artifacts": source_artifacts,
    });
    let status = if write_json(&reference_path, &reference).is_ok() {
        StageStatus::Passed
    } else {
        StageStatus::Failed
    };
    StageResult {
        id: "casefile-reference".to_string(),
        status,
        evidence: "local_real".to_string(),
        command: None,
        working_directory: None,
        duration_ms: 0,
        exit_code: Some(0),
        timed_out: false,
        reason: Some("local CaseFile-compatible path/hash reference recorded".to_string()),
        artifacts: vec![relative_path(output_root, &reference_path)],
    }
}

fn run_external_stage(output_root: &Path, command: StageCommand<'_>) -> StageResult {
    if !command.cwd.is_dir()
        || !command.program.is_file() && command.program.components().count() > 1
    {
        return blocked_stage(
            output_root,
            command.id,
            command.evidence,
            "required executable or working directory is missing",
        );
    }
    let arg_refs: Vec<&str> = command.args.iter().map(String::as_str).collect();
    let result =
        run_process(&command.program, &arg_refs, &command.cwd, command.timeout, &command.env);
    let status = if result.timed_out && command.blocked_on_timeout {
        StageStatus::Blocked
    } else if result.timed_out || result.exit_code != Some(0) {
        if command.blocked_on_failure {
            StageStatus::Blocked
        } else {
            StageStatus::Failed
        }
    } else {
        StageStatus::Passed
    };
    let reason = if result.timed_out {
        "stage timed out"
    } else if status == StageStatus::Passed {
        "stage completed"
    } else {
        "stage exited unsuccessfully"
    };
    write_process_stage(
        output_root,
        command.id,
        command.evidence,
        &command.cwd,
        display_command(&command.program, &arg_refs),
        result,
        status,
        reason,
    )
}

fn run_process(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
    env: &[(&str, &str)],
) -> ProcessCapture {
    let started = Instant::now();
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd).stdout(Stdio::piped()).stderr(Stdio::piped()).env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    if let Some(home) = std::env::var_os("HOME") {
        command.env("HOME", home);
    }
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return ProcessCapture {
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
                timed_out: false,
                duration: started.elapsed(),
            }
        }
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().unwrap_or_else(|_| std::process::Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                });
                return ProcessCapture {
                    exit_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    timed_out: false,
                    duration: started.elapsed(),
                };
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child.wait_with_output().ok();
                return ProcessCapture {
                    exit_code: None,
                    stdout: output
                        .as_ref()
                        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                        .unwrap_or_default(),
                    stderr: output
                        .as_ref()
                        .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
                        .unwrap_or_default(),
                    timed_out: true,
                    duration: started.elapsed(),
                };
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                return ProcessCapture {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    timed_out: false,
                    duration: started.elapsed(),
                }
            }
        }
    }
}

fn write_process_stage(
    output_root: &Path,
    id: &str,
    evidence: &str,
    cwd: &Path,
    command_line: String,
    process: ProcessCapture,
    status: StageStatus,
    reason: &str,
) -> StageResult {
    let stage_dir = output_root.join("stages").join(id);
    let _ = fs::create_dir_all(&stage_dir);
    let stdout_path = stage_dir.join("stdout.txt");
    let stderr_path = stage_dir.join("stderr.txt");
    let result_path = stage_dir.join("result.json");
    let _ = fs::write(&stdout_path, &process.stdout);
    let _ = fs::write(&stderr_path, &process.stderr);
    let mut artifacts = vec![
        relative_path(output_root, &stdout_path),
        relative_path(output_root, &stderr_path),
        relative_path(output_root, &result_path),
    ];
    if status == StageStatus::Blocked {
        let receipt_path = stage_dir.join("blocked-receipt.json");
        let _ = write_json(
            &receipt_path,
            &json!({
                "schema": "kujo-ecosystem-blocked-receipt/v1",
                "stage": id,
                "evidence": evidence,
                "status": "blocked",
                "reason": reason,
                "command": command_line,
            }),
        );
        artifacts.push(relative_path(output_root, &receipt_path));
    }
    let result = StageResult {
        id: id.to_string(),
        status,
        evidence: evidence.to_string(),
        command: Some(command_line),
        working_directory: Some(cwd.display().to_string()),
        duration_ms: process.duration.as_millis(),
        exit_code: process.exit_code,
        timed_out: process.timed_out,
        reason: Some(reason.to_string()),
        artifacts,
    };
    let _ = write_json(&result_path, &result);
    result
}

fn display_command(program: &Path, args: &[&str]) -> String {
    std::iter::once(program.display().to_string())
        .chain(args.iter().map(|arg| format!("\"{}\"", arg.replace('"', "\\\""))))
        .collect::<Vec<_>>()
        .join(" ")
}

fn blocked_stage(output_root: &Path, id: &str, evidence: &str, reason: &str) -> StageResult {
    let stage_dir = output_root.join("stages").join(id);
    let _ = fs::create_dir_all(&stage_dir);
    let result_path = stage_dir.join("result.json");
    let receipt_path = stage_dir.join("blocked-receipt.json");
    let _ = write_json(
        &receipt_path,
        &json!({
            "schema": "kujo-ecosystem-blocked-receipt/v1",
            "stage": id,
            "evidence": evidence,
            "status": "blocked",
            "reason": reason,
        }),
    );
    let result = StageResult {
        id: id.to_string(),
        status: StageStatus::Blocked,
        evidence: evidence.to_string(),
        command: None,
        working_directory: None,
        duration_ms: 0,
        exit_code: None,
        timed_out: false,
        reason: Some(reason.to_string()),
        artifacts: vec![
            relative_path(output_root, &result_path),
            relative_path(output_root, &receipt_path),
        ],
    };
    let _ = write_json(&result_path, &result);
    result
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let body = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, body).map_err(|error| error.to_string())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn collect_artifacts(root: &Path) -> Result<Vec<ArtifactEntry>, String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<ArtifactEntry>,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("artifacts.json") {
            continue;
        }
        let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
        files.push(ArtifactEntry {
            path: relative_path(root, &path),
            bytes: metadata.len(),
            sha256: sha256_file(&path)?,
        });
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn report_ok(failed_stages: usize, blocked_stages: usize, allow_blocked: bool) -> bool {
    failed_stages == 0 && (allow_blocked || blocked_stages == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_root_must_be_empty_when_precreated() {
        let root = std::env::temp_dir().join(format!("kujo_golden_path_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("existing.txt"), "x").unwrap();
        let error = prepare_output_root(Some(&root)).unwrap_err();
        assert!(error.contains("new and empty"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn artifact_hash_is_stable_and_hex_encoded() {
        let root = std::env::temp_dir().join(format!("kujo_golden_path_hash_{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("value.txt");
        fs::write(&path, "kujo").unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "cc89e49d76db3627520f5bd923995954a08f0bd5885b56cb99fc68c98e6ff7d1"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn blocked_stages_require_explicit_opt_in() {
        let stages = vec![StageResult {
            id: "dispatch".to_string(),
            status: StageStatus::Blocked,
            evidence: "fixture".to_string(),
            command: None,
            working_directory: None,
            duration_ms: 0,
            exit_code: None,
            timed_out: true,
            reason: Some("fixture timeout".to_string()),
            artifacts: Vec::new(),
        }];
        let failed = stages.iter().filter(|stage| stage.status == StageStatus::Failed).count();
        let blocked = stages.iter().filter(|stage| stage.status == StageStatus::Blocked).count();
        assert!(failed == 0 && blocked > 0);
        assert!(!report_ok(failed, blocked, false));
        assert!(report_ok(failed, blocked, true));
    }

    #[test]
    fn process_runner_captures_fixture_output() {
        let root = std::env::temp_dir();
        let result =
            run_process(Path::new("echo"), &["golden-path"], &root, Duration::from_secs(2), &[]);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout.trim(), "golden-path");
        assert!(!result.timed_out);
    }
}
