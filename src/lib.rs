//! Low level proxy server
//! To implement request proxying, only standard [`TcpStream`] was used without additional libraries
//! # Examples
//! With default params:
//! ```no_run
//! use proxy_server::Builder;                                                                       
//!      
//! fn main() {
//!     Builder::new().bind().expect("Error in proxy");
//! }
//! ```
//! With custom params:
//! ```no_run
//! use proxy_server::{log::LogLevel, Builder};
//!
//! fn main() {
//!     Builder::new()
//!         .with_address("127.0.0.1:3000")
//!         .with_target("127.0.0.1:3001")
//!         .with_log_level(LogLevel::Warn)
//!         .with_threads(4)
//!         .bind()
//!         .expect("Error in proxy");
//! }
//! ```

use std::{
    io::{ErrorKind, Read, Result, Write},
    net::{TcpListener, TcpStream},
    str,
    thread::sleep,
    time::Duration,
};
mod thread_pool;
use thread_pool::ThreadPool;
mod http;
use http::{Http, Status};
pub mod log;
use log::{Log, LogLevel};
mod prelude;
use prelude::*;

#[cfg(test)]
mod tests;

pub const CHUNK_SIZE: usize = 1024;

#[allow(inactive_code)]
#[cfg(feature = "chunk-10KB")]
pub const CHUNK_SIZE: usize = 10240;

pub const TARGET_ADDRESS: &str = "127.0.0.1:3001";
pub const THREADS: usize = 4;
pub const LOG_LEVEL: LogLevel = LogLevel::Info;
pub const PROXY_ADDRESS: &str = "127.0.0.1:3000";

/// Structure for proxy server configuration
#[derive(Debug, Clone, Copy)]
pub struct Builder {
    pub address: &'static str,
    pub target: &'static str,
    pub log_level: LogLevel,
    pub threads: usize,
    pub chunk_size: usize,
}

impl Builder {
    /// Create new proxy server builder
    pub fn new() -> Self {
        Self {
            address: PROXY_ADDRESS,
            target: TARGET_ADDRESS,
            log_level: LOG_LEVEL,
            threads: THREADS,
            chunk_size: CHUNK_SIZE,
        }
    }

    /// Set proxy server address
    pub fn with_address(mut self, address: &'static str) -> Self {
        self.address = address;
        self
    }

    /// Set proxy server target address
    pub fn with_target(mut self, target: &'static str) -> Self {
        self.target = target;
        self
    }

    /// Set log level of proxy server
    pub fn with_log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }

    /// Set proxy server count of used threads
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    /// Proxy server listener releasing [`std::net::TcpListener`] via thread pool
    pub fn bind(self) -> Result<()> {
        let listener = TcpListener::bind(&self.address)?;

        let _log = Log::new(&self.log_level);
        println!("Listening proxy on {}", self.address);

        let pool = ThreadPool::new(self.threads);
        for stream in listener.incoming() {
            let cl = Handler::new(self);
            pool.execute(|| {
                cl.handle_proxy(stream.expect("Error in incoming stream"))
                    .expect("Failed handle proxy");
            });
        }
        Ok(())
    }
}

struct Handler {
    config: Builder,
}

impl Handler {
    fn new(config: Builder) -> Self {
        Self { config }
    }

    fn handle_proxy(self, client: TcpStream) -> Result<()> {
        let _log = Log::new(&self.config.log_level);
        _log.println(LogLevel::Info, "handle proxy", &client);

        let mut client = Http::from(client);
        let mut heads = vec![];
        client.read_to_end(&mut heads)?;
        let heads_n = change_header_host(&heads);
        if let None = heads_n {
            _log.println(LogLevel::Warn, "Header host is missing", &heads);
            client.set_status(Status::BadRequest)?;
            client.write("Content-Length: 0\r\n\r\n".as_bytes())?;
            return Ok(());
        }
        let heads_n = heads_n.unwrap();

        let http = Http::connect(&self.config.target);
        if let Err(e) = &http {
            _log.println(LogLevel::Warn, "Failed proxy", e);
            client.set_status(Status::BadGateway)?;
            client.write("Content-Length: 0\r\n\r\n".as_bytes())?;
            client.flush()?;
            sleep(Duration::from_millis(100));
            return Ok(());
        }
        let mut http = http?;

        _log.println(LogLevel::Info, "write headers to target", &heads_n);
        http.write(heads_n.as_bytes())?;
        let mut h = vec![];
        http.read_to_end(&mut h)?;

        _log.println(
            LogLevel::Info,
            "send headers to client",
            str::from_utf8(&h).expect("failed decode bytes"),
        );
        client.write(&h).expect("failed send headers");

        loop {
            let mut b = [0; CHUNK_SIZE];
            let r_res = http.read(&mut b);
            if let Err(e) = r_res {
                let log_l = match e.kind() {
                    ErrorKind::ConnectionReset => LogLevel::Info,
                    _ => LogLevel::Error,
                };
                _log.println(log_l, "Failed read chunk", e);
            }
            let mut buf = vec![];
            b.map(|_b| {
                if _b != 0 {
                    buf.push(_b);
                    return true;
                }
                false
            });
            if buf.len() == 0 {
                break;
            }
            client.write(&buf).expect("Failed write chunk");
        }
        Ok(())
    }
}
