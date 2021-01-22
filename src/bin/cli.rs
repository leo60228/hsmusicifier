use anyhow::Result;
use hsmusicifier::add_art;
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

    add_art(bandcamp_json, hsmusic, verbose, in_dir, out_dir, |_, _| ())?;

    Ok(())
}
