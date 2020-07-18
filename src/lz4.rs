use std::error::Error;

/// Decompress lz4 data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    lz_fear::framed::decompress_frame(data).map_err(|err| err.into())
}

/// Compress lz4 data
// TODO: lz-fear does not yet support level
pub fn compress(data: &[u8], _level: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = vec![];
    lz_fear::framed::CompressionSettings::default().compress(data, &mut buf)?;
    Ok(buf)
}
