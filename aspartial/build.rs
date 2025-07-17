use toml::Value as TomlVal;

fn main(){
    let workspace_ver = std::env::var("CARGO_PKG_VERSION").unwrap();
    let manifest_path = std::env::var("CARGO_MANIFEST_PATH").unwrap();
    let manifest = toml::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();

    let TomlVal::Table(manifest) = manifest else {
        panic!("expected manifest to be a table");
    };
    let TomlVal::Table(ref deps) = manifest["dependencies"] else {
        panic!("expected dependencies to be a table");
    };
    let TomlVal::Table(ref aspartial_derive_dep) = deps["aspartial_derive"] else {
        panic!("expected aspartial_derive dep to be a Table")
    };
    let TomlVal::String(ref aspartial_derive_ver) = aspartial_derive_dep["version"] else {
        panic!("expected versoin to be string")
    };
    if workspace_ver != aspartial_derive_ver {
        let message = format!(
            "Crate aspartial ({workspace_ver}) has different version number from aspartial_derive({aspartial_derive_ver})"
        );
        println!("cargo::error={message}")
    }
}
