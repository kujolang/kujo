//! POSIX process replacement and advisory publication mechanisms; no package policy.
use crate::interpreter::capabilities::NativeCapability;
use crate::interpreter::{Interpreter, Value};
use fs2::FileExt;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const BUILTINS: &[(&str, usize)] = &[
    ("file_lock", 2),
    ("file_unlock", 1),
    ("symlink_atomic", 2),
    ("exec_process", 2),
    ("path_owned", 1),
];
static LOCKS: OnceLock<Mutex<HashMap<i64, File>>> = OnceLock::new();
static NEXT: AtomicI64 = AtomicI64::new(1);
fn text(v: &Value) -> Result<&str, String> {
    if let Value::Str(s) = v {
        Ok(s)
    } else {
        Err("expected string".into())
    }
}
fn io(e: std::io::Error) -> String {
    e.to_string()
}

pub fn handle(interp: &mut Interpreter, name: &str, args: &[Value]) -> Option<Value> {
    let (_, n) = BUILTINS.iter().find(|(s, _)| *s == name)?;
    let result = (|| -> Result<Value, String> {
        if args.len() != *n {
            return Err(format!("{name} requires {n} arguments"));
        }
        let cap = match name {
            "exec_process" => NativeCapability::ProcessExec,
            "path_owned" => NativeCapability::FilesystemRead,
            _ => NativeCapability::FilesystemWrite,
        };
        interp.require_capability(cap, name).map_err(|e| format!("{e:?}"))?;
        #[cfg(not(unix))]
        {
            return Err(format!("{name} requires POSIX"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
            match name {
                "path_owned" => {
                    let m = std::fs::symlink_metadata(text(&args[0])?).map_err(io)?;
                    // SAFETY: geteuid has no arguments or memory preconditions.
                    let uid = unsafe { libc::geteuid() };
                    Ok(Value::Bool(!m.file_type().is_symlink() && m.uid() == uid))
                }
                "file_lock" => {
                    let ms = match args[1] {
                        Value::Int(n) if (0..=600000).contains(&n) => n as u64,
                        _ => return Err("lock timeout must be 0..600000 ms".into()),
                    };
                    let file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .mode(0o600)
                        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
                        .open(text(&args[0])?)
                        .map_err(io)?;
                    let m = file.metadata().map_err(io)?;
                    // SAFETY: geteuid has no arguments or memory preconditions.
                    let uid = unsafe { libc::geteuid() };
                    if !m.is_file() || m.uid() != uid || m.mode() & 0o022 != 0 {
                        return Err("lock must be a private user-owned regular file".into());
                    }
                    let start = Instant::now();
                    loop {
                        match file.try_lock_exclusive() {
                            Ok(()) => break,
                            Err(e)
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    && start.elapsed() < Duration::from_millis(ms) =>
                            {
                                std::thread::sleep(Duration::from_millis(10))
                            }
                            Err(e) => return Err(format!("cannot acquire file lock: {e}")),
                        }
                    }
                    let mut locks = LOCKS
                        .get_or_init(Default::default)
                        .lock()
                        .map_err(|_| "lock table unavailable")?;
                    if locks.len() >= 64 {
                        return Err("too many file locks".into());
                    }
                    let id = NEXT.fetch_add(1, Ordering::Relaxed);
                    locks.insert(id, file);
                    Ok(Value::Int(id))
                }
                "file_unlock" => {
                    let id = match args[0] {
                        Value::Int(id) => id,
                        _ => return Err("expected lock handle".into()),
                    };
                    let file = LOCKS
                        .get_or_init(Default::default)
                        .lock()
                        .map_err(|_| "lock table unavailable")?
                        .remove(&id)
                        .ok_or("unknown lock handle")?;
                    FileExt::unlock(&file).map_err(io)?;
                    Ok(Value::Bool(true))
                }
                "symlink_atomic" => {
                    interp
                        .require_capability(NativeCapability::FilesystemDelete, name)
                        .map_err(|e| format!("{e:?}"))?;
                    let target = text(&args[0])?;
                    let destination = std::path::Path::new(text(&args[1])?);
                    if let Ok(m) = std::fs::symlink_metadata(destination) {
                        if !m.file_type().is_symlink() {
                            return Err("refusing to replace non-symlink destination".into());
                        }
                    }
                    let parent = destination.parent().ok_or("symlink destination needs parent")?;
                    let stage = tempfile::Builder::new()
                        .prefix(".symlink-")
                        .tempdir_in(parent)
                        .map_err(io)?;
                    let link = stage.path().join("link");
                    std::os::unix::fs::symlink(target, &link).map_err(io)?;
                    std::fs::rename(link, destination).map_err(io)?;
                    Ok(Value::Bool(true))
                }
                "exec_process" => {
                    use std::os::unix::process::CommandExt;
                    let Value::Array(argv) = &args[0] else {
                        return Err("exec_process expects argv array".into());
                    };
                    if argv.is_empty() || argv.len() > 65536 {
                        return Err("invalid argv length".into());
                    }
                    let strings = argv.iter().map(text).collect::<Result<Vec<_>, _>>()?;
                    if strings.iter().map(|s| s.len()).sum::<usize>() > 8 * 1024 * 1024
                        || strings[0].is_empty()
                    {
                        return Err("invalid argv size".into());
                    }
                    let options_json = crate::builtins::kujo_value_to_json(&args[1])?;
                    let options = options_json
                        .as_object()
                        .ok_or("exec_process expects options dictionary")?;
                    if options.keys().any(|s| s != "env" && s != "env_deny") {
                        return Err("unknown exec_process option".into());
                    }
                    let mut command = std::process::Command::new(strings[0]);
                    command.args(&strings[1..]);
                    if let Some(env) = options.get("env") {
                        for (k, v) in env.as_object().ok_or("env must be dictionary")? {
                            command.env(k, v.as_str().ok_or("environment values must be strings")?);
                        }
                    }
                    if let Some(deny) = options.get("env_deny") {
                        for k in deny.as_array().ok_or("env_deny must be array")? {
                            command
                                .env_remove(k.as_str().ok_or("environment names must be strings")?);
                        }
                    }
                    Err(format!("exec_process failed: {}", command.exec()))
                }
                _ => unreachable!(),
            }
        }
    })();
    Some(result.unwrap_or_else(Value::Error))
}
