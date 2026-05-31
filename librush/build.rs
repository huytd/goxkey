use pm_bin::build_gen;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    build_gen(Some(".".into()))
}
