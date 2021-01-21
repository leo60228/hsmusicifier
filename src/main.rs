use anyhow::{Context, Result};
use hsmusicifier::{bandcamp, hsmusic, locate::*};
use std::env::args_os;
use std::fs::{read_dir, read_to_string, File};
use std::io::{prelude::*, BufReader, BufWriter, SeekFrom};
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let in_dir = args_os().nth(1).context("missing in dir")?;
    let out_dir: PathBuf = args_os().nth(2).context("missing out dir")?.into();

    let json = args_os().nth(3).context("missing json")?;
    let json_file = File::open(json)?;
    let json_reader = BufReader::new(json_file);
    let bandcamp_albums: Vec<bandcamp::Album> = serde_json::from_reader(json_reader)?;

    let hsmusic: PathBuf = args_os().nth(4).context("missing hsmusic")?.into();
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

    for entry in WalkDir::new(&in_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let in_path = entry.path();
        let rel_path = in_path.strip_prefix(&in_dir)?;
        let out_path = out_dir.join(&rel_path);

        println!("{:?} -> {:?}", in_path, out_path);

        let file = File::open(in_path)?;
        let mut reader = BufReader::new(file);
        let mut header = [0; 3];
        reader.read(&mut header)?;
        reader.seek(SeekFrom::Start(0))?;

        if &header == b"ID3" {
            let out_file = File::create(out_path)?;
            let mut writer = BufWriter::new(out_file);

            let tag = id3::Tag::read_from(&mut reader)?;

            let bandcamp = find_bandcamp_from_id3(&tag, &bandcamp_albums);
            println!("bandcamp: {:?}", bandcamp);

            let hsmusic = find_hsmusic_from_id3(&tag, &bandcamp_albums, &hsmusic_albums)?;
            println!("hsmusic ({:?}): {:?}", hsmusic.0.name, hsmusic.1);

            tag.write_to(&mut writer, id3::Version::Id3v23)?; // write id3
            std::io::copy(&mut reader, &mut writer)?; // write mp3
        } else {
            println!("not id3");
        }
    }

    Ok(())
}
