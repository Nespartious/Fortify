use std::fs;
use std::path::Path;

fn main() {
    let version_file = "VERSION";
    
    // Read current version or start at 0.01
    let version = if Path::new(version_file).exists() {
        let content = fs::read_to_string(version_file).unwrap_or_else(|_| "0.01".to_string());
        let version_num: f64 = content.trim().parse().unwrap_or(0.01);
        version_num + 0.01
    } else {
        0.01
    };
    
    // Write new version
    fs::write(version_file, format!("{:.2}", version)).expect("Unable to write VERSION file");
    
    // Set environment variable for compile time
    println!("cargo:rustc-env=FORTIFY_VERSION={:.2}", version);
    println!("cargo:rerun-if-changed=build.rs");
}
