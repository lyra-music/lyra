pub fn main() {
    emit().unwrap_or_else(|e| panic!("emit error: {e:?}"));
}

fn emit() -> Result<(), Box<dyn std::error::Error>> {
    Ok(vergen_git2::Emitter::default()
        .add_instructions(&vergen_git2::BuildBuilder::all_build()?)?
        .add_instructions(&vergen_git2::CargoBuilder::all_cargo()?)?
        .add_instructions(&vergen_git2::Git2Builder::all_git()?)?
        .add_instructions(&vergen_git2::RustcBuilder::all_rustc()?)?
        .add_instructions(&vergen_git2::SysinfoBuilder::all_sysinfo()?)?
        .emit()?)
}
