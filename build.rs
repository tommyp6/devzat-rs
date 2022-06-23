fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !std::env::var("DOCS_RS").is_ok() {
        tonic_build::compile_protos("proto/devzat.proto")?;
    }

    Ok(())
}
