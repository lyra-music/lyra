fn main() {
    vergen::EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .all_rustc()
        .all_sysinfo()
        .emit()
        .unwrap_or_else(|e| panic!("emit error: {e:?}"));
}
