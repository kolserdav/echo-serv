///! Module [`Header`]
use crate::http::CRLF;
#[allow(unused_imports)]
use napi_derive::napi;
use regex::Regex;
use serde::Serialize;
use std::{
    fmt,
    io::{Error, ErrorKind, Result},
    str,
};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Serialize, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Serialize, Clone)]
pub struct Headers {
    pub raw: String,
    pub list: Vec<Header>,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

impl Headers {
    /// Parse headers
    pub fn from_string(raw: String) -> Self {
        let d = Headers::get_headers_prefix(&raw);
        println!("{:?}", d);

        let mut res: Headers = Headers {
            raw: raw.clone(),
            list: vec![],
        };
        let heads = raw.split(CRLF);
        for h in heads {
            let reg_name = Regex::new(r"^.+: ").unwrap();
            let capts_name = reg_name.captures(h);
            if let None = capts_name {
                continue;
            }
            let capts_name = capts_name.unwrap();
            let name = capts_name
                .get(0)
                .unwrap()
                .as_str()
                .to_string()
                .replace(": ", "");

            let reg_value = Regex::new(r": *.*$").unwrap();
            let capts_value = reg_value.captures(h);
            if let None = capts_value {
                res.list.push(Header {
                    name,
                    value: "".to_string(),
                });
                continue;
            }
            let capts_value = capts_value.unwrap();
            let value = capts_value
                .get(0)
                .unwrap()
                .as_str()
                .to_string()
                .replace(": ", "");
            res.list.push(Header {
                name,
                value: value.to_lowercase(),
            });
        }
        res
    }

    /// Create headers from bytes
    pub fn from_bytes(heads: &Vec<u8>) -> Result<Self> {
        let res = str::from_utf8(heads);
        if let Err(err) = res {
            return Err(Error::new(ErrorKind::InvalidInput, err));
        }
        let res = res.unwrap().to_string();
        Ok(Headers::from_string(res))
    }

    /// For change request headers host to host of target
    pub fn change_host(heads: String, target: &str) -> String {
        let reg = Regex::new(r"Host: *.*\r\n").unwrap();
        let capts = reg.captures(heads.as_str());
        if let None = capts {
            return heads;
        }
        let capts = capts.unwrap();
        let old_host = capts.get(0).unwrap().as_str();
        heads.replace(old_host, format!("Host: {}\r\n", target).as_str())
    }

    /// Parse content length from request headers
    pub fn get_content_length(raw: &String) -> Option<u32> {
        let low = Regex::new(r"(c|C)ontent-(l|L)ength:\s*\d+")
            .unwrap()
            .captures(&raw);

        #[allow(unused_assignments)]
        let mut check: Option<&str> = None;
        if let Some(v) = low {
            let low = v.get(0).unwrap();
            check = Some(low.as_str());
        }

        if let None = check {
            return None;
        }

        let cont_len = check.unwrap();

        let num = Regex::new(r"\d+").unwrap().captures(cont_len);
        if let None = num {
            return None;
        }
        let capts = num.unwrap();
        let num = capts.get(0);
        let num_str = num.unwrap().as_str();
        let num = num_str.parse::<u32>();
        if let Err(e) = num {
            println!("Failed parse content lenght from str: {}: {}", num_str, e);
            return None;
        }
        Some(num.unwrap())
    }

    /// Get url from raw headers
    pub fn get_url(raw: &String) -> String {
        let reg = Regex::new(r"\/[a-zA-Z0-9_\-\/]*").unwrap();
        let capts = reg.captures(raw.as_str());
        if let None = capts {
            return "/".to_string();
        }
        let capts = capts.unwrap();
        let url = capts.get(0).unwrap().as_str();
        url.to_string()
    }

    // Get protocol from raw headers
    pub fn get_protocol(raw: &String) -> String {
        let reg = Regex::new(r"HTTPS?\/\d+\.\d+").unwrap();
        let capts = reg.captures(raw.as_str());
        if let None = capts {
            return "OPTIONS".to_string();
        }
        let capts = capts.unwrap();
        let protocol = capts.get(0).unwrap().as_str();
        protocol.to_string()
    }

    // Get request prefix
    fn get_headers_prefix(raw: &String) -> Result<String> {
        let reg = Regex::new(format!(r".+{CRLF}").as_str()).unwrap();
        let capts = reg.captures(raw.as_str());
        if let None = capts {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Wrong HTTP protocol headers: {}", raw),
            ));
        }
        let capts = capts.unwrap();
        let result = capts.get(0).unwrap().as_str();
        Ok(result.to_string())
    }

    /// Get method from raw headers
    pub fn get_method(raw: &String) -> String {
        let reg = Regex::new(r"\w+").unwrap();
        let capts = reg.captures(raw.as_str());
        if let None = capts {
            return "OPTIONS".to_string();
        }
        let capts = capts.unwrap();
        let method = capts.get(0).unwrap().as_str();
        method.to_string()
    }
}
