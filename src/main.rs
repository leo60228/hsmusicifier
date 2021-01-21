use anyhow::{Context, Result};
use hsmusicifier::{bandcamp, hsmusic, locate::*};
use std::env::args_os;
use std::fs::{read_dir, read_to_string, File};
use std::io::{prelude::*, BufReader, SeekFrom};
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let path = args_os().nth(1).context("missing path")?;

    let json = args_os().nth(2).context("missing json")?;
    let json_file = File::open(json)?;
    let json_reader = BufReader::new(json_file);
    let bandcamp_albums: Vec<bandcamp::Album> = serde_json::from_reader(json_reader)?;

    let hsmusic: PathBuf = args_os().nth(3).context("missing hsmusic")?.into();
    let hsmusic_albums_path = {
        let mut p = hsmusic.clone();
        p.push("data");
        p.push("album");
        p
    };

    let hsmusic_album_texts: Vec<_> = read_dir(hsmusic_albums_path)?
        .map(|ent| {
            let ent = ent?;
            Ok(read_to_string(ent.path())?)
        })
        .collect::<Result<_>>()?;

    let hsmusic_albums: Vec<_> = hsmusic_album_texts
        .iter()
        .map(|x| hsmusic::parse_album(x))
        .collect::<Result<_>>()?;

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
            let tag = id3::Tag::read_from(&mut reader)?;

            let bandcamp = find_bandcamp_from_id3(&tag, &bandcamp_albums);
            println!("bandcamp: {:?}", bandcamp);

            let hsmusic = find_hsmusic_from_id3(&tag, &bandcamp_albums, &hsmusic_albums)?;
            println!("hsmusic ({:?}): {:?}", hsmusic.0.name, hsmusic.1);
        } else {
            println!("not id3");
        }
    }

    Ok(())
}
