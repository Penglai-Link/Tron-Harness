use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", tron_backend::openapi::document().to_yaml()?);
    Ok(())
}
