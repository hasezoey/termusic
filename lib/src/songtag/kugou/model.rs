use crate::songtag::UrlTypes;

/**
 * MIT License
 *
 * termusic - Copyright (c) 2021 Larry Hao
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use super::super::{ServiceProvider, SongTag};
use base64::{engine::general_purpose, Engine as _};
use serde_json::{from_str, json, Value};

pub fn to_lyric(json: &str) -> Option<String> {
    if let Ok(value) = from_str::<Value>(json) {
        if value.get("status")?.eq(&200) {
            let lyric = value.get("content")?.as_str()?.to_owned();
            if let Ok(lyric_decoded) = general_purpose::STANDARD.decode(lyric) {
                if let Ok(s) = String::from_utf8(lyric_decoded) {
                    return Some(s);
                }
            }
        }
    }
    None
}

pub fn to_lyric_id_accesskey(json: &str) -> Option<(String, String)> {
    if let Ok(value) = from_str::<Value>(json) {
        if value.get("errcode")?.eq(&200) {
            let v = value.get("candidates")?.get(0)?;
            let accesskey = v
                .get("accesskey")
                .unwrap_or(&json!("Unknown Access Key"))
                .as_str()
                .unwrap_or("Unknown Access Key")
                .to_owned();
            let id = v.get("id")?.as_str()?.to_owned();

            return Some((accesskey, id));
        }
    }
    None
}

pub fn to_song_url(json: &str) -> Option<String> {
    if let Ok(value) = from_str::<Value>(json) {
        if value.get("status")?.eq(&1) {
            let url = value
                .get("data")?
                .get("play_url")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_owned();
            return Some(url);
        }
    }
    None
}

pub fn to_pic_url(json: &str) -> Option<String> {
    if let Ok(value) = from_str::<Value>(json) {
        if value.get("status")?.eq(&1) {
            let url = value
                .get("data")?
                .get("img")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_owned();
            return Some(url);
        }
    }
    None
}

// parse: 解析方式
pub fn to_song_info(json: &str) -> Option<Vec<SongTag>> {
    if let Ok(value) = from_str::<Value>(json) {
        if value.get("status")?.eq(&1) {
            let mut vec: Vec<SongTag> = Vec::new();
            let array = value.get("data")?.as_object()?.get("info")?.as_array()?;
            for v in array {
                if let Some(item) = parse_song_info(v) {
                    vec.push(item);
                }
            }
            return Some(vec);
        }
    }
    None
}

fn parse_song_info(v: &Value) -> Option<SongTag> {
    let price = v
        .get("price")
        .unwrap_or(&json!("Unknown Price"))
        .as_u64()
        .unwrap_or(0);
    let url = if price == 0 {
        UrlTypes::AvailableRequiresFetching
    } else {
        UrlTypes::Protected
    };

    Some(SongTag {
        song_id: v.get("hash")?.as_str()?.to_owned(),
        title: Some(v.get("songname")?.as_str()?.to_owned()),
        artist: Some(
            v.get("singername")
                .unwrap_or(&json!("Unknown Artist"))
                .as_str()
                .unwrap_or("Unknown Artist")
                .to_owned(),
        ),
        album: Some(
            v.get("album_name")
                .unwrap_or(&json!("Unknown Album"))
                .as_str()
                .unwrap_or("")
                .to_owned(),
        ),
        pic_id: Some(v.get("hash")?.as_str()?.to_owned()),
        lang_ext: Some("kugou".to_string()),
        service_provider: ServiceProvider::Kugou,
        lyric_id: Some(v.get("hash")?.as_str()?.to_owned()),
        url: Some(url),
        album_id: Some(v.get("album_id")?.as_str()?.to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // TODO: get some actual test data like migu or netease
    #[test]
    fn should_parse_songinfo() {
        let sample_data = r#"{
            "status": 1,
            "errcode": 0,
            "data": {
              "timestamp": 1111111111,
              "tab": "",
              "forcecorrection": 0,
              "correctiontype": 0,
              "total": 0,
              "istag": 0,
              "allowerr": 0,
              "info": [],
              "aggregation": [],
              "correctiontip": "",
              "istagresult": 0
            },
            "error": ""
          }"#;

        let res = to_song_info(sample_data).unwrap();

        assert_eq!(res.len(), 0);
    }
}
