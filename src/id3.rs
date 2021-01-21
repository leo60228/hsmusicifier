use crate::hsmusic::{Album, Track};
use anyhow::Result;
use id3::frame::{Picture, PictureType};
use std::path::{Path, PathBuf};

impl Track<'_> {
    pub fn picture(&self, album: &Album, path: impl AsRef<Path>) -> Result<Picture> {
        let mut path: PathBuf = path.as_ref().into();
        path.push("media");
        path.push("album-art");
        path.push(&*album.directory);
        path.push(&*self.directory);
        path.set_extension("jpg");

        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                path.set_file_name("cover.jpg");
                std::fs::read(&path)?
            }
            Err(err) => return Err(err.into()),
        };

        Ok(Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: PictureType::CoverFront,
            description: "Cover (Front)".to_string(),
            data,
        })
    }
}
