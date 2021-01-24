use anyhow::Result;
use hsmusicifier::{add_art, ArtType, ArtTypes, Edits};
use std::path::PathBuf;
use structopt::StructOpt;

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

    /// Don't add art
    #[structopt(long)]
    pub no_art: bool,

    /// Use album or track art for first song in album
    #[structopt(long, default_value = "album", conflicts_with = "no_art")]
    pub first_art: ArtType,

    /// Use album or track art for remaining songs in album
    #[structopt(long, default_value = "track", conflicts_with = "no_art")]
    pub rest_art: ArtType,

    /// Don't add artists
    #[structopt(long)]
    pub no_artists: bool,

    /// Add album
    #[structopt(long)]
    pub album: bool,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let Opt {
        bandcamp_json,
        hsmusic,
        verbose,
        in_dir,
        out_dir,
        no_art,
        first_art,
        rest_art,
        no_artists,
        album,
    } = opt;

    let edits = Edits {
        add_art: if no_art {
            None
        } else {
            Some(ArtTypes {
                first: first_art,
                rest: rest_art,
            })
        },
        add_artists: !no_artists,
        add_album: album,
    };

    add_art(
        bandcamp_json,
        hsmusic,
        edits,
        verbose,
        in_dir,
        out_dir,
        drop,
    )?;

    Ok(())
}
