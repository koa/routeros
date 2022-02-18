#![crate_name = "router_os"]
#![crate_type = "lib"]

extern crate crypto;

use std::cell::RefCell;
use std::detect::__is_feature_detected::xsave;
use std::fmt::{Debug, Display, Formatter, Write};
use std::io::{Error, Stderr};
// use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::slice::Iter;
use std::sync::{Arc, Mutex};

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::routeros::model::{ResourceBuilder, RouterOsResource};
use crate::Client;

fn hex_binascii<'a>(hexstr: &str) -> Result<Vec<u8>, &'a str> {
    if hexstr.len() % 2 != 0 {
        Err("Odd number of characters")
    } else {
        let mut result: Vec<u8> = Vec::new();
        let mut i = 0;
        let c_hexstr: Vec<char> = hexstr.chars().collect();
        while i < c_hexstr.len() - 1 {
            let top = c_hexstr[i].to_digit(16).unwrap() as u8;
            let bottom = c_hexstr[i + 1].to_digit(16).unwrap() as u8;
            let r = (top << 4) + bottom;

            result.push(r);

            i += 2;
        }
        Ok(result)
    }
}

#[derive(Debug)]
pub enum RosError {
    TokioError(tokio::io::Error),
    SimpleMessage(String),
}

impl Display for RosError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RosError::TokioError(e) => Debug::fmt(&e, f),
            RosError::SimpleMessage(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for RosError {}

#[derive(Debug, Clone)]
enum ApiReplyType {
    Done,
    Data,
    Trap,
    Fatal,
}

#[derive(Debug, Clone)]
enum Query {
    HasValue(String),
    HasNoValue(String),
    Equals { key: String, value: String },
    Lt { key: String, value: String },
    Gt { key: String, value: String },
}
#[derive(Debug, Clone)]
enum ApiWord {
    Command(String),
    Attribute { key: String, value: String },
    ApiAttribute { key: String, value: String },
    Query(Query),
    Reply(ApiReplyType),
}

impl ApiWord {
    pub fn parse(word: &Vec<u8>) -> Option<ApiWord> {
        if word.is_empty() {
            None
        } else {
            match word[0] as char {
                '/' => Some(ApiWord::Command(String::from_utf8_lossy(&word[1..]).into())),
                '=' => {
                    let (key, value) = Self::split_attributes(&word, 1);

                    Some(ApiWord::Attribute {
                        key,
                        value: value.unwrap_or_else(|| String::from("")),
                    })
                }
                '.' => {
                    let (key, value) = Self::split_attributes(&word, 1);
                    Some(ApiWord::ApiAttribute {
                        key,
                        value: value.unwrap_or_else(|| String::from("")),
                    })
                }
                '!' => {
                    let cow = String::from_utf8_lossy(&word[1..]);
                    let x: &str = cow.trim();
                    let t = match x {
                        "done" => ApiReplyType::Done,
                        "re" => ApiReplyType::Data,
                        "trap" => ApiReplyType::Trap,
                        _ => ApiReplyType::Fatal,
                    };
                    Some(ApiWord::Reply(t))
                }
                '?' => {
                    if word.len() < 2 {
                        None
                    } else {
                        match word[1] as char {
                            '-' => Some(ApiWord::Query(Query::HasNoValue(
                                String::from_utf8_lossy(&word[2..]).into(),
                            ))),
                            '=' => {
                                let (key, value) = Self::split_attributes(&word, 2);
                                Some(ApiWord::Query(Query::Equals {
                                    key,
                                    value: value.unwrap_or_else(|| String::from("")),
                                }))
                            }
                            '>' => {
                                let (key, value) = Self::split_attributes(&word, 2);
                                Some(ApiWord::Query(Query::Lt {
                                    key,
                                    value: value.unwrap_or_else(|| String::from("")),
                                }))
                            }
                            '<' => {
                                let (key, value) = Self::split_attributes(&word, 2);
                                Some(ApiWord::Query(Query::Gt {
                                    key,
                                    value: value.unwrap_or_else(|| String::from("")),
                                }))
                            }
                            _ => {
                                let (key, value) = Self::split_attributes(&word, 1);
                                match value {
                                    Some(value) => {
                                        Some(ApiWord::Query(Query::Equals { key, value }))
                                    }
                                    None => Some(ApiWord::Query(Query::HasValue(key))),
                                }
                            }
                        }
                    }
                }
                _ => panic!("Unsupported word: {}", String::from_utf8_lossy(word)),
            }
        }
    }

    pub fn command<S>(cmd: S) -> ApiWord
    where
        S: ToString,
    {
        ApiWord::Command(cmd.to_string())
    }

    pub fn attribute<K, V>(key: K, value: V) -> ApiWord
    where
        K: ToString,
        V: ToString,
    {
        ApiWord::Attribute {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            ApiWord::Command(cmd) => format!("/{}", cmd).into_bytes(),
            ApiWord::Attribute { key, value } => format!("={}={}", key, value).into_bytes(),
            ApiWord::ApiAttribute { key, value } => format!(".{}={}", key, value).into_bytes(),
            ApiWord::Query(q) => match q {
                Query::HasValue(key) => format!("?{}", key).into_bytes(),
                Query::HasNoValue(key) => format!("?-{}", key).into_bytes(),
                Query::Equals { key, value } => format!("?={}={}", key, value).into_bytes(),
                Query::Lt { key, value } => format!("?<{}={}", key, value).into_bytes(),
                Query::Gt { key, value } => format!("?>{}={}", key, value).into_bytes(),
            },
            ApiWord::Reply(code) => match code {
                ApiReplyType::Done => "!done",
                ApiReplyType::Data => "!re",
                ApiReplyType::Trap => "!trap",
                ApiReplyType::Fatal => "!fatal",
            }
            .as_bytes()
            .into(),
        }
    }

    fn split_attributes(word: &&Vec<u8>, offset: usize) -> (String, Option<String>) {
        match word[offset..].iter().position(|c| *c == ('=' as u8)) {
            Some(n) => (
                String::from_utf8_lossy(&word[offset..n + 1]).into(),
                Some(String::from_utf8_lossy(&word[(n + 2)..]).into()),
            ),

            None => (String::from_utf8_lossy(&word[offset..]).into(), None),
        }
    }
}

pub struct ApiRos {
    stream: TcpStream,
}

impl<'a> ApiRos {
    pub fn new(s: TcpStream) -> ApiRos {
        ApiRos { stream: s }
    }

    pub async fn try_read(&mut self) -> bool {
        if self.stream.read(&mut [0]).await.unwrap() > 0 {
            true
        } else {
            false
        }
    }

    async fn write_bytes(&mut self, str_buff: &[u8]) -> Result<(), RosError> {
        self.stream
            .write(str_buff)
            .await
            .map_err(|e| RosError::TokioError(e));
        Ok(())
    }

    async fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>, RosError> {
        let mut buff: Vec<u8> = Vec::with_capacity(length);
        let result = self.stream.read_buf(&mut buff).await;
        match result {
            Ok(result) => {
                if result == length {
                    Ok(buff)
                } else {
                    Err(RosError::SimpleMessage(format!(
                        "Expected {}, but received {} bytes",
                        length, result
                    )))
                }
            }
            Err(e) => Err(RosError::TokioError(e)),
        }
    }

    async fn write_len(&mut self, len: u32) -> Result<(), RosError> {
        if len < 0x80 {
            self.write_bytes(&[len as u8]).await?;
        } else if len < 0x4000 {
            let l = len | 0x8000;
            self.write_bytes(&[((l >> 8) & 0xFF) as u8, (l & 0xFF) as u8])
                .await?;
        } else if len < 0x200000 {
            let l = len | 0xC00000;
            self.write_bytes(&[
                ((l >> 16) & 0xFF) as u8,
                ((l >> 8) & 0xFF) as u8,
                (l & 0xFF) as u8,
            ])
            .await?;
        } else if len < 0x10000000 {
            let l = len | 0xE0000000;
            self.write_bytes(&[
                ((l >> 16) & 0xFF) as u8,
                ((l >> 24) & 0xFF) as u8,
                ((l >> 8) & 0xFF) as u8,
                (l & 0xFF) as u8,
            ])
            .await?;
        } else {
            self.write_bytes(&[
                (0xF0) as u8,
                ((len >> 24) & 0xFF) as u8,
                ((len >> 16) & 0xFF) as u8,
                ((len >> 8) & 0xFF) as u8,
                (len & 0xFF) as u8,
            ])
            .await?;
        }
        Ok(())
    }

    async fn read_len(&mut self) -> Result<u32, RosError> {
        let c: u32 = self.read_bytes(1).await?[0] as u32;
        if c & 0x80 == 0x00 {
            return Ok(c);
        } else if c & 0xC0 == 0x80 {
            let bytes = self.read_bytes(1).await?;
            return Ok(((c & !0xC0) << 8) + bytes[0] as u32);
        } else if c & 0xE0 == 0xC0 {
            let bytes = self.read_bytes(2).await?;
            return Ok(((c & !0xE0) << 16) + ((bytes[0] as u32) << 8) + bytes[1] as u32);
        } else if c & 0xF0 == 0xE0 {
            let bytes = self.read_bytes(3).await?;
            return Ok(((c & !0xF0) << 24)
                + ((bytes[0] as u32) << 16)
                + ((bytes[1] as u32) << 8)
                + bytes[2] as u32);
        } else if c & 0xF8 == 0xF0 {
            let bytes = self.read_bytes(4).await?;
            return Ok(((c & !0xF0) << 32)
                + ((bytes[0] as u32) << 24)
                + ((bytes[1] as u32) << 16)
                + ((bytes[2] as u32) << 8)
                + bytes[3] as u32);
        }
        Err(RosError::SimpleMessage(format!(
            "Unsupported length pattern: {:x}",
            c
        )))
    }

    async fn read_word(&mut self) -> Result<Option<ApiWord>, RosError> {
        let token = self.read_token().await?;
        let parsed = ApiWord::parse(&token);
        println!(">>> {:?}", parsed);
        Ok(parsed)
    }

    async fn read_token(&mut self) -> Result<Vec<u8>, RosError> {
        let len = self.read_len().await?;
        if len == 0 {
            Ok(vec![])
        } else {
            self.read_bytes(len as usize).await
        }
    }

    async fn write_word(&mut self, w: &ApiWord) -> Result<(), RosError> {
        println!("<<< {:?}", w);

        let token = w.encode();
        self.write_token(&token).await?;
        Ok(())
    }

    async fn write_token(&mut self, token: &Vec<u8>) -> Result<(), RosError> {
        self.write_len(token.len() as u32).await?;
        self.write_bytes(&token).await?;
        Ok(())
    }

    async fn write_sentence<I>(&mut self, words: I) -> Result<u32, RosError>
    where
        I: Iterator<Item = ApiWord>,
    {
        let mut ret: u32 = 0;
        for w in words {
            self.write_word(&w).await?;
            ret += 1;
        }
        self.write_len(0).await?;
        println!("====================");
        Ok(ret)
    }

    pub async fn read_sentence(&mut self) -> Result<Vec<Vec<u8>>, RosError> {
        let mut r: Vec<Vec<u8>> = Vec::new();
        loop {
            let token = self.read_token().await?;
            if token.is_empty() {
                return Ok(r);
            }
            r.push(token);
        }
    }

    async fn talk<W, C>(&mut self, words: W, callback: &mut C) -> Result<(), RosError>
    where
        W: IntoIterator<Item = ApiWord>,
        C: FnMut(ApiWord),
    {
        self.write_sentence(words.into_iter()).await?;

        let mut read_data = false;

        loop {
            let raw_token = self.read_word().await?;
            match raw_token {
                None => {
                    if read_data {
                        read_data = false;
                    } else {
                        return Ok(());
                    }
                }
                Some(ApiWord::Reply(ApiReplyType::Data)) => {
                    callback(ApiWord::Reply(ApiReplyType::Data));
                    read_data = true
                }
                Some(token) => callback(token),
            }
        }
    }

    async fn talk_vec<W>(&mut self, words: W) -> Result<Vec<ApiWord>, RosError>
    where
        W: IntoIterator<Item = ApiWord>,
    {
        let mut r = Vec::new();
        self.talk(words, &mut |word| r.push(word)).await?;
        Ok(r)
    }

    pub async fn login(&mut self, username: String, pwd: String) -> Result<bool, RosError> {
        let login_response = self
            .talk_vec([
                ApiWord::command("login"),
                ApiWord::attribute("name", username),
                ApiWord::attribute("password", pwd),
            ])
            .await?;
        println!("Login response: {:?}", login_response);

        return if let Some(ApiWord::Reply(ApiReplyType::Done)) = login_response.get(0) {
            Ok(true)
        } else {
            Ok(false)
        };
        /*
               let mut chal: Vec<u8> = Vec::new();
               if let Some(vec) = self.talk(vec![r"/login".to_string()]).await {
                   for (_ /*reply*/, attrs) in vec {
                       chal = hex_binascii(attrs["=ret"].clone().as_str()).unwrap();
                   }
               }

               let mut md = Md5::new();
               md.input(&[0]);
               md.input(pwd.as_bytes());
               md.input(&chal[..]);

               self.talk(vec![
                   r"/login".to_string(),
                   format!("=name={}", username),
                   format!("=response=00{}", md.result_str()),
               ])
               .await;

        */
    }
}
pub struct ApiClient {
    api: ApiRos,
}
impl ApiClient {
    pub async fn new(
        target: IpAddr,
        username: String,
        password: String,
    ) -> Result<ApiClient, RosError> {
        let stream: TcpStream = TcpStream::connect(SocketAddr::new(target, 8728))
            .await
            .map_err(|e| RosError::TokioError(e))?;
        let mut api: ApiRos = ApiRos::new(stream);
        let l = api.login(username, password).await?;

        let ports = api
            .talk_vec([ApiWord::command("system/resource/print")])
            .await?;
        println!("Ports: \n{:?}", ports);

        Ok(ApiClient { api })
    }
}
use crate::routeros::client::api::RosError::SimpleMessage;
use async_trait::async_trait;

#[async_trait]
impl Client<RosError> for ApiClient {
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, RosError>
    where
        Resource: RouterOsResource,
    {
        let path = Resource::resource_path();
        let command = format!("{}/print", path);
        let mut result_builder = Mutex::new(RefCell::new(Resource::builder()));

        let mut ret = vec![];
        self.api
            .talk([ApiWord::command(command)], &mut |word| match word {
                ApiWord::Command(_) => {}
                ApiWord::Attribute { key, value } => {
                    result_builder
                        .get_mut()
                        .unwrap()
                        .get_mut()
                        .write_field(key, value);
                }
                ApiWord::Reply(ApiReplyType::Done) => {
                    ret.push(
                        result_builder
                            .get_mut()
                            .unwrap()
                            .replace(Resource::builder())
                            .build(),
                    );
                }
                ApiWord::ApiAttribute { .. } => {}
                ApiWord::Query(_) => {}
                ApiWord::Reply(_) => {}
            })
            .await?;
        Ok(ret)
    }

    async fn get<Resource>(&self) -> Result<Resource, RosError>
    where
        Resource: RouterOsResource,
    {
        todo!()
    }
}
