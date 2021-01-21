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

        let data = std::fs::read(&path)?;

        Ok(Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: PictureType::CoverFront,
            description: "Cover (Front)".to_string(),
            data,
        })
    }
}
