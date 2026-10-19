use kujo::interpreter::Value;
use kujo::module::ModuleLoader;
use std::fs;

#[test]
fn initializer_source_replacement_is_not_cached_as_the_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("rewriting_module.kujo");
    let replacement = "export answer := 222\n";
    fs::write(
        &path,
        format!(
            "write_file({}, {}, true)\nexport answer := 1\n",
            serde_json::to_string(path.to_str().unwrap()).unwrap(),
            serde_json::to_string(replacement).unwrap(),
        ),
    )
    .unwrap();
    let mut loader = ModuleLoader::new();
    loader.add_search_path(temp.path());
    assert!(matches!(loader.get_symbol("rewriting_module", "answer").unwrap(), Value::Int(1)));
    assert_eq!(fs::read_to_string(&path).unwrap(), replacement);
    assert!(matches!(loader.get_symbol("rewriting_module", "answer").unwrap(), Value::Int(222)));
}

#[test]
fn unchanged_source_reuses_evaluated_exports() {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("initialized.txt");
    fs::write(
        temp.path().join("stable_module.kujo"),
        format!(
            "append_file({}, \"initialized\\n\")\nexport answer := 42\n",
            serde_json::to_string(marker.to_str().unwrap()).unwrap(),
        ),
    )
    .unwrap();
    let mut loader = ModuleLoader::new();
    loader.add_search_path(temp.path());
    for _ in 0..3 {
        assert!(matches!(loader.get_symbol("stable_module", "answer").unwrap(), Value::Int(42)));
    }
    assert_eq!(fs::read_to_string(marker).unwrap(), "initialized\n");
}

#[test]
fn repeated_search_path_registration_refreshes_locked_package_discovery() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("initial_module.kujo"), "export answer := 1\n").unwrap();
    let mut loader = ModuleLoader::new();
    loader.add_search_path(temp.path());
    loader.get_symbol("initial_module", "answer").unwrap();

    let package = temp.path().join("kennel_packages/new-package");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("newly_installed_module.kujo"), "export answer := 42\n").unwrap();
    fs::write(
        temp.path().join("kennel.lock"),
        "version = 1\n[[package]]\nname = \"new-package\"\ninstall_path = \"kennel_packages/new-package\"\n",
    )
    .unwrap();
    loader.add_search_path(temp.path());
    assert!(matches!(
        loader.get_symbol("newly_installed_module", "answer").unwrap(),
        Value::Int(42)
    ));
}
