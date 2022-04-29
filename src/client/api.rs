use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::mem;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::client::Client;
use crate::model::{RouterOsListResource, RouterOsResource, RouterOsSingleResource, ValueFormat};
use crate::RosError;

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

    async fn write_bytes(&mut self, str_buff: &[u8]) -> Result<(), RosError> {
        self.stream.write(str_buff).await?;
        Ok(())
    }

    async fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>, RosError> {
        let mut buff: Vec<u8> = Vec::with_capacity(length);
        let result = self.stream.read_buf(&mut buff).await;
        match result {
            Ok(result) => {
                if result < length {
                    let mut pointer = result;
                    while pointer < length {
                        let mut remaining_buff: Vec<u8> = Vec::with_capacity(length - pointer);
                        let result = self.stream.read_buf(&mut remaining_buff).await;
                        match result {
                            Ok(result) => {
                                buff.append(&mut remaining_buff);
                                pointer += result;
                            }
                            Err(e) => return Err(RosError::TokioError(e)),
                        }
                    }
                    Ok(buff)
                } else if result == length {
                    Ok(buff)
                } else {
                    Err(RosError::from(format!(
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
        if cfg!(feature = "debug") {
            println!(">>> {:?}", parsed);
        }
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
        if cfg!(feature = "debug") {
            println!("<<< {:?}", w);
        }
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
        if cfg!(feature = "debug") {
            println!("====================");
        }
        Ok(ret)
    }

    async fn talk<W, C>(&mut self, words: W, callback: &mut C) -> Result<(), RosError>
    where
        W: IntoIterator<Item = ApiWord>,
        C: FnMut(ApiWord) -> Result<(), RosError>,
    {
        self.write_sentence(words.into_iter()).await?;

        let mut read_data = false;

        let mut errors: Vec<RosError> = Vec::new();

        loop {
            let raw_token = self.read_word().await?;
            match raw_token {
                None => {
                    if read_data {
                        read_data = false;
                    } else {
                        return if errors.is_empty() {
                            Ok(())
                        } else {
                            Err(RosError::Umbrella(errors))
                        };
                    }
                }
                Some(ApiWord::Reply(ApiReplyType::Data)) => {
                    Self::push_err(&mut errors, callback(ApiWord::Reply(ApiReplyType::Data)));
                    read_data = true;
                }
                Some(token) => {
                    Self::push_err(&mut errors, callback(token));
                }
            };
        }
    }

    fn push_err(errors: &mut Vec<RosError>, callback_result: Result<(), RosError>) {
        match callback_result {
            Result::Ok(_) => {}
            Result::Err(err) => errors.push(err),
        }
    }

    async fn talk_vec<W>(&mut self, words: W) -> Result<Vec<ApiWord>, RosError>
    where
        W: IntoIterator<Item = ApiWord>,
    {
        let mut r = Vec::new();
        self.talk(words, &mut |word| {
            r.push(word);
            Ok(())
        })
        .await?;
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
        if cfg!(feature = "debug") {
            println!("Login response: {:?}", login_response);
        }

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
        let login_ok = api.login(username, password).await?;
        if login_ok {
            Ok(ApiClient { api })
        } else {
            Err(RosError::SimpleMessage(String::from("Login failed")))
        }
    }

    async fn set<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsResource,
    {
        let mut request: Vec<ApiWord> = Vec::new();
        let path = Resource::resource_path();

        request.push(ApiWord::command(format!("{}/set", path)));
        if let Some((description, value)) = resource.id_field() {
            request.push(ApiWord::attribute(
                description.name,
                value.api_value(&ValueFormat::Api),
            ));
        }
        resource
            .fields()
            .filter_map(|f| f.1.modified_value(&ValueFormat::Api).map(|v| (f.0.name, v)))
            .for_each(|(key, value)| request.push(ApiWord::attribute(key, value)));

        let x = self.api.talk_vec(request).await?;
        Ok(())
    }
}

enum AttributeCollector<Resource: RouterOsResource> {
    Ressource(Resource),
    Error(HashMap<String, String>),
    None,
}

impl<Resource: RouterOsResource> AttributeCollector<Resource> {
    pub fn extract(self) -> (Option<Resource>, Option<HashMap<String, String>>) {
        match self {
            AttributeCollector::Ressource(r) => (Some(r), None),
            AttributeCollector::Error(error) => (None, Some(error)),
            AttributeCollector::None => (None, None),
        }
    }
    pub fn write_attribute(&mut self, key: String, value: String) -> Result<(), RosError> {
        match self {
            AttributeCollector::Ressource(r) => {
                if let Some(field_accessor) = r
                    .fields_mut()
                    .find(|e| e.0.name.eq(key.as_str()))
                    .map(|e| e.1)
                {
                    field_accessor.set_from_api(value.as_str())?;
                    Ok(())
                } else {
                    Err(RosError::field_missing_error(key, value))
                }
            }
            AttributeCollector::Error(error) => {
                error.insert(key, value);
                Ok(())
            }
            AttributeCollector::None => Ok(()),
        }
    }
}

#[async_trait]
impl Client for ApiClient {
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, RosError>
    where
        Resource: RouterOsResource,
    {
        let path = Resource::resource_path();
        let command = format!("{}/print", path);
        let mut result_builder = AttributeCollector::<Resource>::None;

        let mut ret = vec![];
        self.api
            .talk([ApiWord::command(command)], &mut |word| match word {
                ApiWord::Command(_) => Ok(()),
                ApiWord::Attribute { key, value } => result_builder.write_attribute(key, value),

                ApiWord::ApiAttribute { .. } => Ok(()),
                ApiWord::Reply(reply_type) => {
                    let mut new_resource = match reply_type {
                        ApiReplyType::Done => AttributeCollector::Ressource(Resource::default()),
                        ApiReplyType::Data => AttributeCollector::Ressource(Resource::default()),
                        ApiReplyType::Trap => AttributeCollector::Error(HashMap::new()),
                        ApiReplyType::Fatal => AttributeCollector::Error(HashMap::new()),
                    };
                    mem::swap(&mut new_resource, &mut result_builder);
                    let (data, error) = new_resource.extract();
                    if let Some(resource) = data {
                        ret.push(resource);
                    }
                    if let Some(error_data) = error {
                        let message = error_data.get("message").map(String::as_str).unwrap_or("");
                        if message == "no such command prefix" {
                            Ok(())
                        } else {
                            Err(RosError::SimpleMessage(message.to_owned()))
                        }
                    } else {
                        Ok(())
                    }
                }
                ApiWord::Query(_) => Ok(()),
            })
            .await?;
        Ok(ret)
    }

    async fn update<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource,
    {
        {
            self.set(resource).await
        }
    }

    async fn set<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsSingleResource,
    {
        self.set(resource).await
    }

    async fn add<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource,
    {
        let mut request: Vec<ApiWord> = Vec::new();
        let path = Resource::resource_path();

        request.push(ApiWord::command(format!("{}/add", path)));
        resource
            .fields()
            .filter_map(|f| f.1.modified_value(&ValueFormat::Api).map(|v| (f.0.name, v)))
            .for_each(|(key, value)| request.push(ApiWord::attribute(key, value)));

        let x = self.api.talk_vec(request).await?;
        println!("Result: {:?}", x);
        Ok(())
    }
    async fn delete<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource,
    {
        let mut request: Vec<ApiWord> = Vec::new();
        if let Some((description, value)) = resource.id_field() {
            let value = value.api_value(&ValueFormat::Api);
            let path = Resource::resource_path();
            request.push(ApiWord::command(format!("{}/remove", path)));
            request.push(ApiWord::attribute(description.name, value));
        }
        if !request.is_empty() {
            let x = self.api.talk_vec(request).await?;
            println!("Result: {:?}", x);
        }
        Ok(())
    }
}
