//! Small native consumer of the source-string inspection APIs.
#![forbid(unsafe_code)]

use grunkalunka_markdown::{Link, code_blocks, summarize};
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::process::ExitCode;

const SOURCE_LIMIT: usize = 1024 * 1024;
const USAGE: &str = concat!(
    "usage: grunkalunka-markdown-inspect FILE|-\n",
    "       grunkalunka-markdown-inspect --help"
);

fn read_source(reader: impl Read) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut bounded = reader.take((SOURCE_LIMIT + 1) as u64);
    bounded.read_to_end(&mut bytes)?;
    if bytes.len() > SOURCE_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "source exceeds 1048576 bytes",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "source must be valid UTF-8"))
}

fn write_links(output: &mut impl Write, kind: &str, links: &[Link]) -> io::Result<()> {
    writeln!(output, "{kind}s {}", links.len())?;
    for link in links {
        writeln!(
            output,
            "{kind} {}..{} {:?} -> {:?} title={:?}",
            link.span.start, link.span.end, link.text, link.destination, link.title
        )?;
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, USAGE));
    }
    let mut output = BufWriter::new(io::stdout().lock());
    if args[0] == "--help" {
        writeln!(output, "{USAGE}")?;
        return output.flush();
    }
    let source = if args[0] == "-" {
        read_source(io::stdin().lock())?
    } else {
        let file = File::open(&args[0]).map_err(|error| {
            io::Error::new(error.kind(), format!("cannot read {:?}: {error}", args[0]))
        })?;
        read_source(file)?
    };
    let summary = summarize(&source);
    let blocks = code_blocks(&source);
    writeln!(output, "headings {}", summary.headings.len())?;
    for heading in summary.headings {
        writeln!(
            output,
            "heading level={} {}..{} {:?}",
            heading.level, heading.span.start, heading.span.end, heading.text
        )?;
    }
    write_links(&mut output, "link", &summary.links)?;
    write_links(&mut output, "image", &summary.images)?;
    writeln!(output, "code-blocks {}", blocks.len())?;
    for block in blocks {
        writeln!(
            output,
            "code {}..{} info={:?} text-bytes={}",
            block.span.start, block.span.end, block.info, block.text.len()
        )?;
    }
    output.flush()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::FAILURE
        }
    }
}
