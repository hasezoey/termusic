//
// model.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use super::NCMResult;
use crate::lyric::SongTag;
// , NCM_CACHE};
// use async_std::io;
use custom_error::custom_error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
// use std::path::PathBuf;

#[allow(unused)]
pub fn to_lyric(json: String) -> NCMResult<String> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("msg").ok_or(Errors::NoneError)?.eq("成功") {
        let mut vec: Vec<String> = Vec::new();
        let lyric = value
            .get("lyric")
            .ok_or(Errors::NoneError)?
            .as_str()
            .ok_or(Errors::NoneError)?
            .to_owned();
        return Ok(lyric);
    }
    Err(Errors::NoneError)
}

#[allow(unused)]
pub fn to_lyric_id_accesskey(json: String) -> NCMResult<(String, String)> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("errcode").ok_or(Errors::NoneError)?.eq(&200) {
        let v = value
            .get("candidates")
            .ok_or(Errors::NoneError)?
            .get(0)
            .ok_or(Errors::NoneError)?;
        let accesskey = v
            .get("accesskey")
            .unwrap_or(&json!("未知"))
            .as_str()
            .unwrap_or(&"未知")
            .to_owned();
        let id = v
            .get("id")
            .ok_or(Errors::NoneError)?
            .as_str()
            .ok_or(Errors::NoneError)?
            .to_owned();

        return Ok((accesskey, id));
    }
    Err(Errors::NoneError)
}
// 歌手信息
#[derive(Debug, Deserialize, Serialize)]
pub struct SingerInfo {
    // 歌手 id
    pub id: u64,
    // 歌手姓名
    pub name: String,
    // 歌手照片
    pub pic_url: String,
}

#[allow(unused)]
pub fn to_singer_info(json: String) -> NCMResult<Vec<SingerInfo>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("code").ok_or(Errors::NoneError)?.eq(&200) {
        let mut vec: Vec<SingerInfo> = Vec::new();
        let array = value
            .get("result")
            .ok_or(Errors::NoneError)?
            .get("artists")
            .ok_or(Errors::NoneError)?
            .as_array()
            .ok_or(Errors::NoneError)?;
        for v in array.iter() {
            vec.push(SingerInfo {
                id: v
                    .get("id")
                    .ok_or(Errors::NoneError)?
                    .as_u64()
                    .ok_or(Errors::NoneError)? as u64,
                name: v
                    .get("name")
                    .ok_or(Errors::NoneError)?
                    .as_str()
                    .ok_or(Errors::NoneError)?
                    .to_owned(),
                pic_url: v
                    .get("picUrl")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
            });
        }
        return Ok(vec);
    }
    Err(Errors::NoneError)
}

// 歌曲 URL
#[derive(Debug, Deserialize, Serialize)]
pub struct SongUrl {
    // 歌曲 id
    pub id: u64,
    // 歌曲 URL
    pub url: String,
    // 码率
    pub rate: u32,
}

#[allow(unused)]
pub fn to_song_url(json: String) -> NCMResult<Vec<SongUrl>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("status").ok_or(Errors::NoneError)?.eq(&1) {
        let mut vec: Vec<SongUrl> = Vec::new();
        let array = value
            .get("data")
            .ok_or(Errors::NoneError)?
            .get("info")
            .ok_or(Errors::NoneError)?
            .as_array()
            .ok_or(Errors::NoneError)?;
        for v in array.iter() {
            let url = v
                .get("url")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_owned();
            if !url.is_empty() {
                vec.push(SongUrl {
                    id: v
                        .get("id")
                        .ok_or(Errors::NoneError)?
                        .as_u64()
                        .ok_or(Errors::NoneError)? as u64,
                    url,
                    rate: v
                        .get("br")
                        .ok_or(Errors::NoneError)?
                        .as_u64()
                        .ok_or(Errors::NoneError)? as u32,
                });
            }
        }
        return Ok(vec);
    }
    Err(Errors::NoneError)
}

// 歌曲信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongInfo {
    // 歌曲 id
    pub id: u64,
    // 歌名
    pub name: String,
    // 歌手
    pub singer: String,
    // 专辑
    pub album: String,
    // 封面图
    pub pic_url: String,
    // 歌曲时长
    pub duration: String,
    // 歌曲链接
    pub song_url: String,
}

#[allow(unused)]
pub fn to_song_info(json: String, parse: Parse) -> NCMResult<Vec<SongTag>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("success").ok_or(Errors::NoneError)?.eq(&true) {
        let mut vec: Vec<SongTag> = Vec::new();
        let list = json!([]);
        if let Parse::SEARCH = parse {
            let mut array = value
                .get("musics")
                .unwrap_or(&list)
                .as_array()
                .ok_or(Errors::NoneError)?;
            for v in array.iter() {
                let pic_id = v
                    .get("cover")
                    .ok_or(Errors::NoneError)?
                    .as_str()
                    .ok_or(Errors::NoneError)?
                    .to_owned();
                let artist = v
                    .get("singerName")
                    .unwrap_or(&json!("未知"))
                    .as_str()
                    .unwrap_or(&"未知")
                    .to_owned();
                let title = v
                    .get("songName")
                    .ok_or(Errors::NoneError)?
                    .as_str()
                    .ok_or(Errors::NoneError)?
                    .to_owned();

                let album_id = v
                    .get("albumId")
                    .ok_or(Errors::NoneError)?
                    .as_str()
                    .ok_or(Errors::NoneError)?
                    .to_owned();

                vec.push(SongTag {
                    song_id: Some(
                        v.get("id")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                    ),
                    title: Some(title),
                    artist: Some(artist),
                    album: Some(
                        v.get("albumName")
                            .unwrap_or(&json!("未知"))
                            .as_str()
                            .unwrap_or(&"")
                            .to_owned(),
                    ),
                    pic_id: Some(pic_id),
                    lang_ext: Some("chi".to_string()),
                    service_provider: Some("migu".to_string()),
                    lyric_id: Some(
                        v.get("copyrightId")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                    ),
                    url: Some(
                        v.get("mp3")
                            .unwrap_or(&json!("N/A"))
                            .as_str()
                            .unwrap_or(&"")
                            .to_owned(),
                    ),
                    album_id: Some(album_id),
                });
            }
            return Ok(vec);
        }
    }
    Err(Errors::NoneError)
}

// 歌单信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongList {
    // 歌单 id
    pub id: u64,
    // 歌单名
    pub name: String,
    // 歌单封面
    pub cover_img_url: String,
}

// parse: 解析方式
#[allow(unused)]
pub fn to_song_list(json: String, parse: Parse) -> NCMResult<Vec<SongList>> {
    let value = serde_json::from_str::<Value>(&json)?;
    if value.get("code").ok_or(Errors::NoneError)?.eq(&200) {
        let mut vec: Vec<SongList> = Vec::new();
        match parse {
            Parse::USL => {
                let array = value
                    .get("playlist")
                    .ok_or(Errors::NoneError)?
                    .as_array()
                    .ok_or(Errors::NoneError)?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or(Errors::NoneError)?
                            .as_u64()
                            .ok_or(Errors::NoneError)? as u64,
                        name: v
                            .get("name")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                        cover_img_url: v
                            .get("coverImgUrl")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                    });
                }
            }
            Parse::RMD => {
                let array = value
                    .get("recommend")
                    .ok_or(Errors::NoneError)?
                    .as_array()
                    .ok_or(Errors::NoneError)?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or(Errors::NoneError)?
                            .as_u64()
                            .ok_or(Errors::NoneError)? as u64,
                        name: v
                            .get("name")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                        cover_img_url: v
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                    });
                }
            }
            Parse::ALBUM => {
                let array = value
                    .get("albums")
                    .ok_or(Errors::NoneError)?
                    .as_array()
                    .ok_or(Errors::NoneError)?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or(Errors::NoneError)?
                            .as_u64()
                            .ok_or(Errors::NoneError)? as u64,
                        name: v
                            .get("name")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                        cover_img_url: v
                            .get("picUrl")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                    });
                }
            }
            Parse::TOP => {
                let array = value
                    .get("playlists")
                    .ok_or(Errors::NoneError)?
                    .as_array()
                    .ok_or(Errors::NoneError)?;
                for v in array.iter() {
                    vec.push(SongList {
                        id: v
                            .get("id")
                            .ok_or(Errors::NoneError)?
                            .as_u64()
                            .ok_or(Errors::NoneError)? as u64,
                        name: v
                            .get("name")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                        cover_img_url: v
                            .get("coverImgUrl")
                            .ok_or(Errors::NoneError)?
                            .as_str()
                            .ok_or(Errors::NoneError)?
                            .to_owned(),
                    });
                }
            }
            _ => {}
        }
        return Ok(vec);
    }
    Err(Errors::NoneError)
}

// 消息
#[derive(Debug, Deserialize, Serialize)]
pub struct Msg {
    pub code: i32,
    pub msg: String,
}

#[allow(unused)]
pub fn to_msg(json: String) -> NCMResult<Msg> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or(Errors::NoneError)?
        .as_i64()
        .ok_or(Errors::NoneError)? as i32;
    if code.eq(&200) {
        return Ok(Msg {
            code: 200,
            msg: "".to_owned(),
        });
    }
    let msg = value
        .get("msg")
        .ok_or(Errors::NoneError)?
        .as_str()
        .ok_or(Errors::NoneError)?
        .to_owned();
    Ok(Msg { code, msg })
}

// 登陆信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginInfo {
    // 登陆状态码
    pub code: i32,
    // 用户 id
    pub uid: u64,
    // 用户昵称
    pub nickname: String,
    // 用户头像
    pub avatar_url: String,
    // 状态消息
    pub msg: String,
}

#[allow(unused)]
pub fn to_login_info(json: String) -> NCMResult<LoginInfo> {
    let value = serde_json::from_str::<Value>(&json)?;
    let code = value
        .get("code")
        .ok_or(Errors::NoneError)?
        .as_i64()
        .ok_or(Errors::NoneError)? as i32;
    if code.eq(&200) {
        let profile = value
            .get("profile")
            .ok_or(Errors::NoneError)?
            .as_object()
            .ok_or(Errors::NoneError)?;
        return Ok(LoginInfo {
            code,
            uid: profile
                .get("userId")
                .ok_or(Errors::NoneError)?
                .as_u64()
                .ok_or(Errors::NoneError)? as u64,
            nickname: profile
                .get("nickname")
                .ok_or(Errors::NoneError)?
                .as_str()
                .ok_or(Errors::NoneError)?
                .to_owned(),
            avatar_url: profile
                .get("avatarUrl")
                .ok_or(Errors::NoneError)?
                .as_str()
                .ok_or(Errors::NoneError)?
                .to_owned(),
            msg: "".to_owned(),
        });
    }
    let msg = value
        .get("msg")
        .ok_or(Errors::NoneError)?
        .as_str()
        .ok_or(Errors::NoneError)?
        .to_owned();
    Ok(LoginInfo {
        code,
        uid: 0,
        nickname: "".to_owned(),
        avatar_url: "".to_owned(),
        msg,
    })
}

// 请求方式
#[allow(unused)]
#[derive(Debug)]
pub enum Method {
    POST,
    GET,
}

// 解析方式
// USL: 用户
// UCD: 云盘
// RMD: 推荐
// RMDS: 推荐歌曲
// SEARCH: 搜索
// SD: 单曲详情
// ALBUM: 专辑
// TOP: 热门
#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Parse {
    USL,
    UCD,
    RMD,
    RMDS,
    SEARCH,
    SD,
    ALBUM,
    TOP,
}

custom_error! { pub Errors
    OpenSSLError{ source: openssl::error::ErrorStack } = "openSSL Error",
    RegexError{ source: regex::Error } = "regex Error",
    SerdeJsonError{ source: serde_json::error::Error } = "serde json Error",
    ParseError{ source: std::num::ParseIntError } = "parse Error",
    // AsyncIoError{ source: io::Error } = "async io Error",
    // IsahcError{ source: isahc::Error } = "isahc Error",
    NoneError = "None Error",
    // FromUtf8Error{source: std::string::FromUtf8Error} = "UTF8 Error",
}
