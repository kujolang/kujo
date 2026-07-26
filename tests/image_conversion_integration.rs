use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, Rgba};
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

fn write_rgba_fixture(path: &Path) {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(3, 2, |x, y| {
        Rgba([10 + x as u8, 20 + y as u8, 30, 40 + (x + y) as u8])
    }));
    image.save_with_format(path, ImageFormat::Png).expect("failed to write RGBA fixture");
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
        assert!(out_interp.exists(), "interpreter output file missing: {out_interp:?}");
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
        assert!(out_vm.exists(), "vm output file missing: {out_vm:?}");
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

#[test]
fn image_pixel_api_roundtrips_rgba_in_interpreter_and_vm() {
    let root = unique_test_dir("kujo_image_pixels");
    let input = root.join("source.png");
    write_rgba_fixture(&input);

    for (runtime, output) in [("interpreter", root.join("interp.png")), ("vm", root.join("vm.png"))]
    {
        let script = format!(
            concat!(
                "img := load_image(\"{}\")\n",
                "w := img.width()\n",
                "h := img.height()\n",
                "fmt := img.format()\n",
                "before := img.get_pixel(1, 1)\n",
                "rgb_ok := img.set_pixel(1, 1, 101, 102, 103)\n",
                "rgba_ok := img.set_pixel(2, 0, 201, 202, 203, 204)\n",
                "after_rgb := img.get_pixel(1, 1)\n",
                "after_rgba := img.get_pixel(2, 0)\n",
                "saved := img.save(\"{}\")\n"
            ),
            escape_kujo_string(&input),
            escape_kujo_string(&output)
        );

        let env = if runtime == "interpreter" {
            let interp = run_interpreter(&script);
            interp.env
        } else {
            let env = vm_env_with_builtins();
            let result = run_vm(&script, env.clone());
            assert!(result.is_ok(), "VM pixel script failed: {:?}", result.err());
            Arc::try_unwrap(env).expect("VM env still shared").into_inner().unwrap()
        };

        assert!(matches!(env.get("w"), Some(Value::Int(3))));
        assert!(matches!(env.get("h"), Some(Value::Int(2))));
        assert!(matches!(env.get("fmt"), Some(Value::Str(value)) if value.as_str() == "png"));
        assert_pixel(env.get("before"), [11, 21, 30, 42]);
        assert_pixel(env.get("after_rgb"), [101, 102, 103, 42]);
        assert_pixel(env.get("after_rgba"), [201, 202, 203, 204]);
        assert!(matches!(env.get("rgb_ok"), Some(Value::Bool(true))));
        assert!(matches!(env.get("rgba_ok"), Some(Value::Bool(true))));
        assert!(matches!(env.get("saved"), Some(Value::Bool(true))));

        let reloaded = image::open(&output).expect("saved PNG should reload").to_rgba8();
        assert_eq!(reloaded.get_pixel(1, 1).0, [101, 102, 103, 42]);
        assert_eq!(reloaded.get_pixel(2, 0).0, [201, 202, 203, 204]);
    }

    let _ = std::fs::remove_dir_all(&root);
}

fn assert_pixel(value: Option<Value>, expected: [i64; 4]) {
    let Value::Array(channels) = value.expect("pixel result should exist") else {
        panic!("pixel result should be an array")
    };
    let actual: Vec<i64> = channels
        .iter()
        .map(|value| match value {
            Value::Int(channel) => *channel,
            _ => panic!("pixel channels should be integers"),
        })
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn image_pixel_api_rejects_invalid_coordinates_and_channels() {
    let root = unique_test_dir("kujo_image_pixel_errors");
    let input = root.join("source.png");
    write_rgba_fixture(&input);

    let cases = [
        ("img.get_pixel(3, 0)", "outside image bounds"),
        ("img.get_pixel(\"0\", 0)", "x to be a non-negative integer"),
        ("img.set_pixel(0, 2, 1, 2, 3)", "outside image bounds"),
        ("img.set_pixel(0, 0, 256, 2, 3)", "integers from 0 to 255"),
        ("img.set_pixel(0, 0, 1, 2, \"3\")", "integers from 0 to 255"),
    ];

    for (expression, expected) in cases {
        let script = format!(
            "img := load_image(\"{}\")\nresult := {}\n",
            escape_kujo_string(&input),
            expression
        );
        let interp = run_interpreter(&script);
        assert!(
            matches!(interp.return_value, Some(Value::Error(ref message)) if message.contains(expected)),
            "interpreter did not report {expected:?} for {expression}"
        );

        let vm = run_vm(&script, vm_env_with_builtins());
        assert!(
            matches!(vm, Err(ref message) if message.contains(expected)),
            "VM did not report {expected:?} for {expression}: {vm:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}
