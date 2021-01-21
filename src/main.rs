use anyhow::{Context, Result};
use hsmusicifier::{bandcamp, locate::*};
use std::env::args_os;
use std::fs::File;
use std::io::{prelude::*, BufReader, SeekFrom};
use walkdir::WalkDir;

fn main() -> Result<()> {
    let path = args_os().nth(1).context("missing path")?;

    let json = args_os().nth(2).context("missing json")?;
    let json_file = File::open(json)?;
    let json_reader = BufReader::new(json_file);
    let bandcamp_albums: Vec<bandcamp::Album> = serde_json::from_reader(json_reader)?;

    for entry in WalkDir::new(path) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        println!("{:?}", entry.path());

        let file = File::open(entry.path())?;
        let mut reader = BufReader::new(file);
        let mut header = [0; 3];
        reader.read(&mut header)?;
        reader.seek(SeekFrom::Start(0))?;

        if &header == b"ID3" {
            println!("id3");
            let tag = id3::Tag::read_from(&mut reader)?;
            if tag.title() == Some("Frustracean") {
                println!("frustracean");
                continue;
            }

            let track = find_id3_bandcamp(&tag, &bandcamp_albums)?;
            println!("{:?}", track);
        } else {
            println!("not id3");
        }
    }

    Ok(())
}
