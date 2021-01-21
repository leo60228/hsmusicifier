use anyhow::Result;
use hsmusicifier::{bandcamp, hsmusic, locate::*};
use id3::{frame::PictureType, Tag};
use std::fs::{read_dir, read_to_string, File};
use std::io::{prelude::*, BufReader, BufWriter, SeekFrom};
use std::path::PathBuf;
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(StructOpt)]
#[structopt(
    name = "hsmusicifier",
    about = "A tool to add track art to Homestuck music."
)]
struct Opt {
    /// Location of dumped bandcamp json
    #[structopt(short, long = "bandcamp-json", parse(from_os_str))]
    pub bandcamp_json: PathBuf,

    /// Location of hsmusic
    #[structopt(short = "m", long, parse(from_os_str))]
    pub hsmusic: PathBuf,

    /// Verbosity
    #[structopt(short, long)]
    pub verbose: bool,

    /// Input directory
    #[structopt(parse(from_os_str))]
    pub in_dir: PathBuf,

    /// Output directory
    #[structopt(parse(from_os_str))]
    pub out_dir: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let Opt {
        bandcamp_json,
        hsmusic,
        verbose,
        in_dir,
        out_dir,
    } = opt;

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

        let mut out_file = File::create(out_path)?;

        if &header == b"ID3" {
            let mut writer = BufWriter::new(out_file);

            let mut tag = Tag::read_from(&mut reader)?;

            let (album, track) = find_hsmusic_from_id3(&tag, &bandcamp_albums, &hsmusic_albums)?;

            if verbose {
                println!("hsmusic ({:?}): {:?}", album.name, track);
            }

            tag.remove_picture_by_type(PictureType::CoverFront);
            tag.add_picture(track.picture(&album, &hsmusic)?);

            tag.write_to(&mut writer, id3::Version::Id3v23)?; // write id3
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
