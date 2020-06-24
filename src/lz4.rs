use std::error::Error;
use std::io::{Read, Write};

/// Decompress lz4 data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut decoder = lz4::Decoder::new(data)?;
    let mut buf = vec![];
    decoder.read_to_end(&mut buf)?;
    let (_, result) = decoder.finish(); // Weird API...
    result?;
    Ok(buf)
}

/// Compress lz4 data
pub fn compress(data: &[u8], level: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = vec![];
    let mut encoder = lz4::EncoderBuilder::new().level(level).build(&mut buf)?;
    encoder.write_all(data)?;
    let (_, result) = encoder.finish(); // Weird API...
    result?;
    Ok(buf)
}
