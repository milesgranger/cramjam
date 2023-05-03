use std::any::Any;
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, StdoutLock, Write};
use std::time::{Duration, Instant};

use bytesize::ByteSize;
use clap::Parser;

#[derive(Parser)]
#[command(author = "Miles Granger, miles59923@gmail.com")]
#[command(about = "CLI interface to many different de/compression algorithms")]
#[command(after_long_help = "Example: cramjam snappy compress myfile.txt out.txt.snappy")]
struct Cli {
    #[arg(help = "The de/compression algorithm name")]
    codec: String,
    #[arg(help = "Either 'compress' or 'decompress'")]
    action: String,
    #[arg(short, long, help = "Input file, if empty then read from stdin")]
    input: Option<String>,
    #[arg(short, long, help = "Output file, if empty then write to stdout")]
    output: Option<String>,
    #[arg(short, long, help = "Compression level, if relevant to the algorithm")]
    level: Option<isize>,
    #[arg(short, long, help = "Remove all informational output", action = clap::ArgAction::SetTrue)]
    quiet: bool,
}

trait ReadableDowncast: Read + Any {
    fn as_any(&self) -> &dyn Any;
}
impl<T: Read + Any> ReadableDowncast for T {
    fn as_any(&self) -> &dyn Any {
        &*self
    }
}
trait WritableDowncast: Write + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: Write + Any> WritableDowncast for T {
    fn as_any(&self) -> &dyn Any {
        &*self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut *self
    }
}

#[derive(Debug)]
enum Error {
    Other(String),
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Other(err)
    }
}
impl<'a> From<&'a str> for Error {
    fn from(err: &'a str) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}
impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn main() -> io::Result<()> {
    let mut m = Cli::parse();

    let input: Box<dyn ReadableDowncast> = match m.input {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(std::io::stdin().lock()),
    };
    let mut output: Box<dyn WritableDowncast> = match m.output {
        Some(path) => Box::new(File::create(path)?),
        None => {
            m.quiet = true; // Don't echo anything in stdout that isn't actual data output
            Box::new(std::io::stdout().lock())
        }
    };

    // if input is a file, then we can probably get the input length for stats
    let maybe_len = input
        .as_any()
        .downcast_ref::<&File>()
        .map(|file| file.metadata().ok().map(|m| m.len()).unwrap_or_default());

    let start = Instant::now();
    let nbytes = match m.action.as_str() {
        "compress" => match m.codec.as_str() {
            "snappy" => libcramjam::snappy::compress(input, &mut output),
            "lz4" => {
                // TODO: lz4 doesn't impl Read for their Encoder, so cannot determine
                // number of bytes compressed without using Seek, which stdout doesn't have,
                // as it's streaming. So here, we'll go ahead and read everything in then
                // send it in as a cursor, file can remain as is.
                // When lz4 implements Reader for the Encoder, then all this can go away.
                // along with the `Seek` trait bound on the internal::compress function
                if let Some(stdout) = ((&mut *output).as_any_mut()).downcast_mut::<StdoutLock>() {
                    let mut data = vec![];
                    libcramjam::lz4::compress(input, &mut Cursor::new(&mut data), m.level.map(|v| v as _))?;
                    std::io::copy(&mut Cursor::new(data), stdout).map(|v| v as usize)
                } else {
                    match ((&mut *output).as_any_mut()).downcast_mut::<File>() {
                        Some(file) => libcramjam::lz4::compress(input, file, m.level.map(|v| v as _)),
                        None => unreachable!("Did we implement something other than Stdout and File for output?"),
                    }
                }
            }
            "bzip2" => libcramjam::bzip2::compress(input, &mut output, m.level.map(|v| v as _)),
            "gzip" => libcramjam::gzip::compress(input, &mut output, m.level.map(|v| v as _)),
            "zstd" => libcramjam::zstd::compress(input, &mut output, m.level.map(|v| v as _)),
            "deflate" => libcramjam::deflate::compress(input, &mut output, m.level.map(|v| v as _)),
            "brotli" => libcramjam::brotli::compress(input, &mut output, m.level.map(|v| v as _)),
            _ => Err(Error::from("codec not recognized").into()),
        },
        "decompress" => match m.codec.as_str() {
            "snappy" => libcramjam::snappy::decompress(input, &mut output),
            "lz4" => libcramjam::lz4::decompress(input, &mut output),
            "bzip2" => libcramjam::bzip2::decompress(input, &mut output),
            "gzip" => libcramjam::gzip::decompress(input, &mut output),
            "zstd" => libcramjam::zstd::decompress(input, &mut output),
            "deflate" => libcramjam::deflate::decompress(input, &mut output),
            "brotli" => libcramjam::brotli::decompress(input, &mut output),
            _ => return Err(Error::from("codec not recognized").into()),
        },
        _ => return Err(Error::from("'action' must be either 'compress' or 'decompress'").into()),
    }?;
    let duration = start.elapsed();

    if !m.quiet {
        if let Some(len) = maybe_len {
            println!("Input:      {}", ByteSize(len as _));
            println!("Output:     {}", ByteSize(nbytes as _));
            println!("Change:     {:.2}%", ((nbytes as f32 - len as f32) / len as f32) * 100.,);
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
