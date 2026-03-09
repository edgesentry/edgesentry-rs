fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let output = std::path::PathBuf::from(&crate_dir).join("include/edgesentry_bridge.h");

    let config = cbindgen::Config::from_file(
        std::path::PathBuf::from(&crate_dir).join("cbindgen.toml"),
    )
    .unwrap_or_default();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("unable to generate C bindings")
        .write_to_file(output);
}
