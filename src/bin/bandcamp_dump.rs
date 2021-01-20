use anyhow::{Context, Result};
use std::env::args_os;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<()> {
    let path = args_os().nth(1).context("missing path")?;
    let file = File::create(path)?;
    let w = BufWriter::new(file);

    let albums = hsmusicifier::bandcamp::albums()?;

    serde_json::to_writer_pretty(w, &albums)?;

    Ok(())
}
