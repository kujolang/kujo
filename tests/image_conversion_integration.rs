use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn run_interpreter(code: &str) -> Interpreter {
    let tokens = tokenize(code).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    let mut interp = Interpreter::new();
    interp.eval_stmts(&program);
    interp
}

fn run_vm(code: &str, env: Arc<Mutex<Environment>>) -> Result<Value, String> {
    let tokens = tokenize(code).map_err(|diagnostics| {
        diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "unknown lexer error".to_string())
    })?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse();

    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&program)?;

    let mut vm = VM::new();
    vm.set_globals(env);
    vm.execute(chunk)
}

fn vm_env_with_builtins() -> Arc<Mutex<Environment>> {
    let interp = Interpreter::new();
    Arc::new(Mutex::new(interp.env))
}

fn escape_kujo_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"")
}

fn write_fixture(path: &Path, format: ImageFormat) {
    let image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(16, 16, Rgb([30, 120, 220])));
    image.save_with_format(path, format).expect("failed to write fixture image");
}

fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    std::fs::create_dir_all(&dir).expect("failed to create temp test directory");
    dir
}

#[test]
fn image_conversion_roundtrip_works_in_interpreter_and_vm() {
    let root = unique_test_dir("kujo_image_conversion");

    let in_png = root.join("source.png");
    let in_jpg = root.join("source.jpg");
    let in_webp = root.join("source.webp");
    write_fixture(&in_png, ImageFormat::Png);
    write_fixture(&in_jpg, ImageFormat::Jpeg);
    write_fixture(&in_webp, ImageFormat::WebP);

    let scenarios = vec![
        (&in_png, root.join("out_from_png_interp.webp"), root.join("out_from_png_vm.webp")),
        (&in_jpg, root.join("out_from_jpg_interp.webp"), root.join("out_from_jpg_vm.webp")),
        (&in_webp, root.join("out_from_webp_interp.png"), root.join("out_from_webp_vm.png")),
    ];

    for (input, out_interp, out_vm) in scenarios {
        let interp_script = format!(
            "img := load_image(\"{}\")\nok := img.save(\"{}\")\n",
            escape_kujo_string(input),
            escape_kujo_string(&out_interp)
        );
        let interp = run_interpreter(&interp_script);
        assert!(
            matches!(interp.env.get("ok"), Some(Value::Bool(true))),
            "interpreter save() did not return true"
        );
        assert!(out_interp.exists(), "interpreter output file missing: {:?}", out_interp);
        let interp_size =
            std::fs::metadata(&out_interp).expect("failed to stat interpreter output").len();
        assert!(interp_size > 0, "interpreter output file is empty");
        image::open(&out_interp).expect("interpreter output is not loadable image");

        let vm_script = format!(
            "img := load_image(\"{}\")\nok := img.save(\"{}\")\n",
            escape_kujo_string(input),
            escape_kujo_string(&out_vm)
        );
        let env = vm_env_with_builtins();
        let vm_result = run_vm(&vm_script, env.clone());
        assert!(vm_result.is_ok(), "vm script failed: {:?}", vm_result.err());
        let saved_flag = env.lock().unwrap().get("ok");
        assert!(matches!(saved_flag, Some(Value::Bool(true))), "vm save() did not return true");
        assert!(out_vm.exists(), "vm output file missing: {:?}", out_vm);
        let vm_size = std::fs::metadata(&out_vm).expect("failed to stat vm output").len();
        assert!(vm_size > 0, "vm output file is empty");
        image::open(&out_vm).expect("vm output is not loadable image");
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn image_conversion_failure_paths_are_reported() {
    let root = unique_test_dir("kujo_image_conversion_failures");
    let input_png = root.join("input.png");
    write_fixture(&input_png, ImageFormat::Png);

    // Missing input path
    let missing_path = root.join("missing_input.jpg");
    let interp_missing = run_interpreter(&format!(
        "missing_result := load_image(\"{}\")\n",
        escape_kujo_string(&missing_path)
    ));
    assert!(matches!(
        interp_missing.return_value,
        Some(Value::Error(ref msg)) if msg.contains("Cannot load image")
    ));

    let vm_missing = run_vm(
        &format!("missing_result := load_image(\"{}\")\n", escape_kujo_string(&missing_path)),
        vm_env_with_builtins(),
    );
    assert!(matches!(vm_missing, Err(msg) if msg.contains("Cannot load image")));

    // Unsupported output extension
    let unsupported_output = root.join("out.invalidext");
    let interp_unsupported = run_interpreter(&format!(
        "img := load_image(\"{}\")\nunsupported_result := img.save(\"{}\")\n",
        escape_kujo_string(&input_png),
        escape_kujo_string(&unsupported_output)
    ));
    assert!(matches!(
        interp_unsupported.return_value,
        Some(Value::Error(ref msg)) if msg.contains("Failed to save image")
    ));

    let vm_unsupported = run_vm(
        &format!(
            "img := load_image(\"{}\")\nunsupported_result := img.save(\"{}\")\n",
            escape_kujo_string(&input_png),
            escape_kujo_string(&unsupported_output)
        ),
        vm_env_with_builtins(),
    );
    assert!(matches!(vm_unsupported, Err(msg) if msg.contains("Failed to save image")));

    // Invalid argument types for method call
    let interp_invalid_args = run_interpreter(&format!(
        "img := load_image(\"{}\")\ninvalid_resize := img.resize(\"wide\", 50)\n",
        escape_kujo_string(&input_png)
    ));
    assert!(matches!(
        interp_invalid_args.return_value,
        Some(Value::Error(ref msg)) if msg.contains("resize requires numeric width and height")
    ));

    let vm_invalid_args = run_vm(
        &format!(
            "img := load_image(\"{}\")\ninvalid_resize := img.resize(\"wide\", 50)\n",
            escape_kujo_string(&input_png)
        ),
        vm_env_with_builtins(),
    );
    assert!(
        matches!(vm_invalid_args, Err(msg) if msg.contains("resize requires numeric width and height"))
    );

    let _ = std::fs::remove_dir_all(&root);
}
