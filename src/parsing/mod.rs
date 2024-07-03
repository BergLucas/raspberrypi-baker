use std::{fs::File, io::Read};

use parser::BakerFile;

pub mod parser;

pub(crate) fn load_bakerfile(path: &str) -> Result<BakerFile, std::io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    parser::parse_baker_file::<()>(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
        .map(|(_, bakerfile)| bakerfile)
        .into()
}
