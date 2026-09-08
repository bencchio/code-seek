use crate::walker;
use std::path::Path;

pub fn run(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("'{}' does not exist", path.display()).into());
    }
    let files = walker::walk(path);
    for file in &files {
        println!("{}", file.display());
    }
    println!("\n{} file(s)", files.len());
    Ok(())
}
