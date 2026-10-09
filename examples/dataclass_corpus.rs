//! Prepare the full benchmark corpus. Normal tests generate isolated copies.
#[path = "../tests/support/dataclass_corpus.rs"]
mod corpus;
fn main() {
    let destination = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "target/generated-dataclass-fixtures".into());
    let files = corpus::generate(&destination);
    println!(
        "Generated {} fixture artifacts in {}",
        files.len(),
        destination.display()
    );
}
