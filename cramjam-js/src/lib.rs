use libcramjam;
use wasm_bindgen::prelude::*;

type Result<T> = std::result::Result<T, JsValue>;

#[inline(always)]
fn err_into_jsvalue<E: ToString>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[wasm_bindgen]
pub struct Compress {}

#[wasm_bindgen]
impl Compress {
    pub fn brotli(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![];
        libcramjam::brotli::compress(input, &mut out, None).map_err(err_into_jsvalue)?;
        Ok(out)
    }
    pub fn snappy(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![];
        libcramjam::snappy::compress(input, &mut out).map_err(err_into_jsvalue)?;
        Ok(out)
    }
    pub fn lz4(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![];
        libcramjam::lz4::compress(input, &mut out, None).map_err(err_into_jsvalue)?;
        Ok(out)
    }
}

#[wasm_bindgen]
pub struct Decompress {}

#[wasm_bindgen]
impl Decompress {
    pub fn brotli(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![];
        libcramjam::brotli::decompress(input, &mut out).map_err(err_into_jsvalue)?;
        Ok(out)
    }
    pub fn snappy(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![];
        libcramjam::snappy::decompress(input, &mut out).map_err(err_into_jsvalue)?;
        Ok(out)
    }
    pub fn lz4(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![];
        libcramjam::lz4::decompress(input, &mut out).map_err(err_into_jsvalue)?;
        Ok(out)
    }
}
