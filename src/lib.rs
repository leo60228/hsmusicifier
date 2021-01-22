use ::id3::{frame::PictureType, Tag, Version};
use anyhow::Result;
use locate::*;
use std::fs::{create_dir_all, read_dir, read_to_string, File};
use std::io::{prelude::*, BufReader, BufWriter, SeekFrom};
use std::path::PathBuf;
use walkdir::WalkDir;

pub mod bandcamp;
pub mod hsmusic;
mod id3;
pub mod locate;

pub fn add_art(
    bandcamp_json: PathBuf,
    hsmusic: PathBuf,
    verbose: bool,
    in_dir: PathBuf,
    out_dir: PathBuf,
    mut progress: impl FnMut(usize, usize),
) -> Result<()> {
    let bandcamp_file = File::open(bandcamp_json)?;
    let bandcamp_reader = BufReader::new(bandcamp_file);
    let bandcamp_albums: Vec<bandcamp::Album> = serde_json::from_reader(bandcamp_reader)?;

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

    let entries: Vec<_> = WalkDir::new(&in_dir)
        .into_iter()
        .filter(|x| {
            if let Ok(x) = x {
                x.file_type().is_file()
            } else {
                true
            }
        })
        .collect::<std::result::Result<_, _>>()?;
    let entries_count = entries.len();
    for (i, entry) in entries.into_iter().enumerate() {
        progress(i, entries_count);

        let in_path = entry.path();
        let rel_path = in_path.strip_prefix(&in_dir)?;
        let out_path = out_dir.join(&rel_path);

        println!("{:?} -> {:?}", in_path, out_path);

        let file = File::open(in_path)?;
        let mut reader = BufReader::new(file);
        let mut header = [0; 3];
        reader.read_exact(&mut header)?;
        reader.seek(SeekFrom::Start(0))?;

        if let Some(parent) = out_path.parent() {
            create_dir_all(parent)?;
        }

        let mut out_file = File::create(out_path)?;

        if &header == b"ID3" {
            let mut writer = BufWriter::new(out_file);

            let mut tag = Tag::read_from(&mut reader)?;

            let (album, track) = find_hsmusic_from_id3(&tag, &bandcamp_albums, &hsmusic_albums)?;

            if verbose {
                println!("hsmusic ({:?}): {:?}", album.name, track.name);
            }

            tag.remove_picture_by_type(PictureType::CoverFront);
            tag.add_picture(track.picture(&album, &hsmusic)?);

            tag.write_to(&mut writer, Version::Id3v23)?; // write id3
            std::io::copy(&mut reader, &mut writer)?; // write mp3
        } else {
            if verbose {
                println!("not id3");
            }

            std::io::copy(&mut reader, &mut out_file)?;
        }
    }

    Ok(())
}
