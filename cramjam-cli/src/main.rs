use std::any::Any;
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, StdoutLock, Write};
use std::time::{Duration, Instant};

use bytesize::ByteSize;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Parser)]
#[command(author, version, about)]
#[command(after_long_help = "Example: cramjam snappy compress --input myfile.txt --output out.txt.snappy")]
struct Cli {
    #[command(subcommand)]
    codec: Codec,
    #[arg(short, long, global = true, help = "Input file, if not set will read from stdin")]
    input: Option<String>,
    #[arg(short, long, global = true, help = "Output file, if not set will write to stdout")]
    output: Option<String>,
    #[arg(short, long, global = true, help = "Remove all informational output", action = clap::ArgAction::SetTrue)]
    quiet: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Action {
    Compress,
    Decompress,
}

// TODO: Config per algorithm, matching it's specific possible parameters (level, speed, block, etc)
#[derive(Args, Clone)]
struct Config {
    #[arg(value_enum)]
    action: Action,
    #[arg(short, long, help = "Level, if relevant to the algorithm")]
    level: Option<isize>,
}

#[derive(Clone, Subcommand)]
enum Codec {
    Lz4(Config),
    Snappy(Config),
    ZSTD(Config),
    Brotli(Config),
    Gzip(Config),
    Deflate(Config),
    Bzip2(Config),
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
    let maybe_len = (&*input)
        .as_any()
        .downcast_ref::<File>()
        .map(|file| file.metadata().ok().map(|m| m.len()).unwrap_or_default());

    let start = Instant::now();
    let nbytes = match m.codec {
        Codec::Snappy(conf) => match conf.action {
            Action::Compress => libcramjam::snappy::compress(input, &mut output),
            Action::Decompress => libcramjam::snappy::decompress(input, &mut output),
        },
        Codec::Lz4(conf) => {
            match conf.action {
                Action::Compress => {
                    // TODO: lz4 doesn't impl Read for their Encoder, so cannot determine
                    // number of bytes compressed without using Seek, which stdout doesn't have,
                    // as it's streaming. So here, we'll go ahead and read everything in then
                    // send it in as a cursor, file can remain as is.
                    // When lz4 implements Reader for the Encoder, then all this can go away.
                    // along with the `Seek` trait bound on the internal::compress function
                    if let Some(stdout) = ((&mut *output).as_any_mut()).downcast_mut::<StdoutLock>() {
                        let mut data = vec![];
                        libcramjam::lz4::compress(input, &mut Cursor::new(&mut data), conf.level.map(|v| v as _))?;
                        io::copy(&mut Cursor::new(data), stdout).map(|v| v as usize)
                    } else {
                        match ((&mut *output).as_any_mut()).downcast_mut::<File>() {
                            Some(file) => libcramjam::lz4::compress(input, file, conf.level.map(|v| v as _)),
                            None => unreachable!("Did we implement something other than Stdout and File for output?"),
                        }
                    }
                }
                Action::Decompress => libcramjam::lz4::decompress(input, &mut output),
            }
        }
        Codec::Bzip2(conf) => match conf.action {
            Action::Compress => libcramjam::bzip2::compress(input, &mut output, conf.level.map(|v| v as _)),
            Action::Decompress => libcramjam::bzip2::decompress(input, &mut output),
        },
        Codec::Gzip(conf) => match conf.action {
            Action::Compress => libcramjam::gzip::compress(input, &mut output, conf.level.map(|v| v as _)),
            Action::Decompress => libcramjam::gzip::decompress(input, &mut output),
        },
        Codec::ZSTD(conf) => match conf.action {
            Action::Compress => libcramjam::zstd::compress(input, &mut output, conf.level.map(|v| v as _)),
            Action::Decompress => libcramjam::zstd::decompress(input, &mut output),
        },
        Codec::Deflate(conf) => match conf.action {
            Action::Compress => libcramjam::deflate::compress(input, &mut output, conf.level.map(|v| v as _)),
            Action::Decompress => libcramjam::deflate::decompress(input, &mut output),
        },
        Codec::Brotli(conf) => match conf.action {
            Action::Compress => libcramjam::brotli::compress(input, &mut output, conf.level.map(|v| v as _)),
            Action::Decompress => libcramjam::brotli::decompress(input, &mut output),
        },
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
