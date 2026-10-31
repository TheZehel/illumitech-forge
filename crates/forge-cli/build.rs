use std::{env, fs, path::Path};
fn walk(path: &Path, base: &Path, out: &mut String) {
    let mut entries: Vec<_> = fs::read_dir(path)
        .expect("read ui-dist")
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        assert!(
            !p.symlink_metadata().unwrap().file_type().is_symlink(),
            "no symlink assets"
        );
        if p.is_dir() {
            walk(&p, base, out);
        } else {
            let relative = p
                .strip_prefix(base)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            let mime = match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8",
                "js" => "text/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                _ => "application/octet-stream",
            };
            out.push_str(&format!(
                "{key:?} => Some(({mime:?}, include_bytes!({file:?}))),\n",
                key = format!("/{relative}"),
                file = p.canonicalize().unwrap()
            ));
        }
    }
}
fn main() {
    let base = Path::new("ui-dist");
    println!("cargo:rerun-if-changed=ui-dist");
    for required in ["index.html", "logo.webp", "THIRD_PARTY_LICENSES.txt"] {
        assert!(
            base.join(required).is_file(),
            "required standalone UI asset missing: {required}; run the locked frontend build"
        );
    }
    let html = fs::read_to_string(base.join("index.html")).expect("read embedded UI entry point");
    for part in html.split('"').filter(|part| part.starts_with("/assets/")) {
        assert!(
            !part.contains("..") && base.join(&part[1..]).is_file(),
            "referenced embedded UI asset missing: {part}"
        );
    }
    let mut out = String::from(
        "fn asset(path: &str) -> Option<(&'static str, &'static [u8])> {match path {\n",
    );
    walk(base, base, &mut out);
    out.push_str("_ => None, }}");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("workbench_assets.rs"),
        out,
    )
    .unwrap();
}
