use anyhow::{anyhow, Context, Result};
use htmlescape::decode_html;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Album {
    pub name: String,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub name: String,
    pub url: String,
    pub num: usize,
}

fn album_urls() -> Result<Vec<String>> {
    let base_url = Url::parse("https://homestuck.bandcamp.com/music").unwrap();
    let html = attohttpc::get(&base_url).send()?.text()?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a[href^='/album/']").unwrap();
    document
        .select(&selector)
        .map(|x| {
            let href = x.value().attr("href").context("missing href")?.to_string();
            let url = base_url.join(&href)?;
            Ok(url.into_string())
        })
        .collect()
}

pub fn album(url: &str) -> Result<Album> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TrackItem {
        name: String,
        url: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TrackJson {
        item: TrackItem,
        position: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TrackList {
        item_list_element: Vec<TrackJson>,
    }

    #[derive(Deserialize)]
    struct AlbumJson {
        track: TrackList,
        name: String,
    }

    let html = attohttpc::get(url).send()?.text()?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("script[type='application/ld+json']").unwrap();
    let script = document
        .select(&selector)
        .nth(0)
        .context("missing JSON-LD")?;
    let json = script.inner_html();
    let album_json: AlbumJson = serde_json::from_str(&json)?;

    Ok(Album {
        tracks: album_json
            .track
            .item_list_element
            .into_iter()
            .map(|json| {
                Ok(Track {
                    name: decode_html(&json.item.name).map_err(|x| anyhow!("{:?}", x))?,
                    url: json.item.url,
                    num: json.position,
                })
            })
            .collect::<Result<_>>()?,
        name: decode_html(&album_json.name).map_err(|x| anyhow!("{:?}", x))?,
    })
}

pub fn albums() -> Result<Vec<Album>> {
    album_urls()?.into_iter().map(|x| album(&x)).collect()
}
