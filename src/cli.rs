use std::fs::File;
use std::time::{Duration, Instant};

use bytesize::ByteSize;
use clap::Parser;
use pyo3::prelude::*;

use crate::*;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(main, m)?)?;
    Ok(())
}

#[derive(Parser)]
#[command(author = "Miles Granger, miles59923@gmail.com")]
#[command(about = "CLI interface to many different de/compression algorithms")]
#[command(after_long_help = "Example: cramjam snappy compress myfile.txt out.txt.snappy")]
#[command(arg_required_else_help = true)]
struct Cli {
    #[arg()]
    _python: String,
    #[arg(help = "The de/compression algorithm name")]
    codec: String,
    #[arg(help = "Either 'compress' or 'decompress'")]
    action: String,
    #[arg(help = "Input file")]
    input: String,
    #[arg(help = "Output file")]
    output: String,
    #[arg(short, long, help = "Remove all informational output", action = clap::ArgAction::SetTrue)]
    quiet: bool,
}

#[pyfunction]
pub fn main() -> PyResult<()> {
    let m = Cli::parse();

    let input = File::open(m.input)?;
    let mut output = File::create(m.output)?;
    let len_result = input.metadata().map(|m| m.len());

    let start = Instant::now();

    let nbytes = snappy::internal::compress(input, &mut output)?;
    let duration = start.elapsed();

    if !m.quiet {
        if let Ok(len) = len_result {
            println!("Input:      {}", ByteSize(len as _));
            println!("Output:     {}", ByteSize(nbytes as _));
            println!("Reduction:  {:.2}%", (1. - (nbytes as f32 / len as f32)) * 100.,);
            println!("Ratio:      {:.2}", (len as f32 / nbytes as f32));
            println!("Throughput: {}/sec", calc_throughput_sec(duration, len as _));
        }
    }
    Ok(())
}

fn calc_throughput_sec(duration: Duration, nbytes: usize) -> ByteSize {
    if duration.as_millis() > 0 {
        ByteSize(((nbytes as u128 / (duration.as_millis())) as u64) * 1_000)
    } else if duration.as_micros() > 0 {
        ByteSize(((nbytes as u128 / (duration.as_micros())) as u64) * 10_000)
    } else {
        ByteSize(((nbytes as u128 / (duration.as_nanos())) as u64) * 100_000)
    }
}
