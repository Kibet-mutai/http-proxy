//httprequest
//httpresponse

pub mod dns;

use std::{
    collections::HashMap,
    fmt::{self, write},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Shutdown, TcpListener, TcpStream},
    vec,
};

use dns::resolve::Query;

#[derive(Debug)]
pub enum Error {
    ParseUrlError,
    StreamconnectionError(String),
    ResponseParseError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ParseUrlError => {
                write!(f, "URL parsing error",)
            }
            Error::StreamconnectionError(string) => {
                write!(f, "Error establishing connection: {string}")
            }
            Error::ResponseParseError => {
                write!(f, "Response parsing error",)
            }
        }
    }
}

enum Method {
    GET,
    POST,
    PUT,
    DELETE,
}

#[derive(Debug)]
struct URL {
    scheme: String,
    host: String,
    //port: String,
    path: String,
}

impl fmt::Display for URL {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.host)
    }
}

impl URL {
    pub fn from(url: &str) -> Result<Self, Error> {
        let addr = if url.starts_with("http") || url.starts_with("https") {
            url.to_owned()
        } else {
            format!("http://{}", url)
        };

        let mut split = addr.split("://");
        let scheme = match split.next() {
            Some(v) => v.to_string(),
            None => return Err(Error::ParseUrlError),
        };
        split = match split.next() {
            Some(v) => v.split("/"),
            None => return Err(Error::ParseUrlError),
        };

        let host = match split.next() {
            Some(v) => v.to_string(),
            None => return Err(Error::ParseUrlError),
        };

        let mut path = String::new();

        loop {
            match split.next() {
                Some(v) => path.push_str(format!("/{}", v).as_str()),
                None => {
                    if path.is_empty() {
                        path.push('/');
                    }
                    break;
                }
            }
        }
        Ok(URL {
            scheme: scheme,
            host: host,
            path: path,
        })
    }
}

struct HttpRequest {
    method: Method,
    headers: HashMap<String, String>,
    protocol_version: String,
    body: Option<Vec<u8>>,
}

struct HttpResponse {
    //status_code: u32,
    status_line: String,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Debug)]
struct ClientConnection {
    url: URL,
    stream: TcpStream,
}

struct ServerConnection {
    addr: IpAddr,
    listener: TcpListener,
}

impl HttpRequest {
    pub fn new() -> Self {
        HttpRequest {
            method: Method::GET,
            headers: HashMap::new(),
            body: None,
            protocol_version: String::new(),
        }
    }
    pub fn set_method(mut self, method: Method) -> HttpRequest {
        self.method = method;
        self
    }

    pub fn set_headers(mut self, headers: HashMap<String, String>) -> HttpRequest {
        self.headers = headers;
        self
    }

    pub fn get_headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn get_body(&self) -> &Option<Vec<u8>> {
        &self.body
    }

    pub fn get_content_length(&self) -> usize {
        if let Some(body) = &self.body {
            body.len()
        } else {
            0
        }
    }
}

impl ClientConnection {
    pub fn new(url: &str) -> Result<ClientConnection, Error> {
        let parsed_url = URL::from(url)?;
        let stream = TcpStream::connect(format!("{}:80", parsed_url.host));
        println!("Coonection : {:?}", stream);
        match stream {
            Ok(s) => Ok(ClientConnection {
                url: parsed_url,
                stream: s,
            }),
            Err(e) => Err(Error::StreamconnectionError(e.to_string())),
        }
    }
    pub fn set_headers(&mut self) -> Result<(), Error> {
        let request = HttpRequest::new();
        self.stream
            .write_all(format!("GET {} HTTP/1.1\r\n", self.url.path).as_bytes())
            .unwrap();
        self.stream
            .write_all(format!("HOST: {}\r\n", self.url.host).as_bytes())
            .unwrap();
        for header in request.get_headers() {
            self.stream
                .write_all(format!("{}: {}\r\n", header.0, header.1).as_bytes())
                .unwrap();
        }
        self.stream
            .write_all(format!("Content-Length: {}\r\n", request.get_content_length()).as_bytes())
            .unwrap();
        // if let Some(range) = request.get_range() {
        //     self.stream
        //         .write_all(format!("Range: bytes={}-{}\r\n", range.start, range.end).as_bytes())
        //         .unwrap();
        // }

        self.stream.write_all(b"Connection: Close\r\n").unwrap();
        self.stream.write_all(b"\r\n").unwrap();

        if let Some(body) = request.get_body() {
            self.stream.write_all(body.as_slice()).unwrap();
        }

        self.stream.write_all(b"\r\n\r\n").unwrap();
        Ok(())
    }

    pub fn get_response(&mut self, response: &str) -> Option<(String, String)> {
        let mut headers_hash: HashMap<&str, &str> = HashMap::new();
        let mut parts = response.split("\r\n\r\n");
        if let (Some(headers), Some(body)) = (parts.next(), parts.next()) {
            // let header_string = headers.to_string();
            // let mut head_split = header_string.split_once("\r\n");
            // println!("Headers: {:?}", head_split);
            // if let Some(header_line) = head_split {
            //     let status_line = header_line.0.to_string();
            //     println!("Status line: {}", status_line);
            //     let rest_headers = header_line.1;
            //     for line in rest_headers.lines() {
            //         let mut parts = line.splitn(2, ": ");
            //         if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            //             headers_hash.insert(key, value);
            //        }
            //     }
            // }
            // println!("Header key and values:\n{:?}", headers_hash);
            (Some((headers.to_string(), body.to_string())))
        } else {
            None
        }
    }

    pub fn send_request(&mut self) -> Result<HttpResponse, Error> {
        // consist of headers

        let _ = self.set_headers().unwrap();
        let mut headers_hash = HashMap::new();
        //let mut body = Vec::new();
        let mut buffer = String::new();
        let bytes_read = self.stream.read_to_string(&mut buffer).unwrap();
        let res = &buffer[..bytes_read];
        let mut stat_line = String::new();
        if let Some((headers, body)) = self.get_response(res) {
            //println!("Headers:\n{}\n\nBody:\n{}", headers, body);
            let mut status_line = None;
            for line in headers.as_str().lines() {
                if status_line.is_none() {
                    status_line = Some(line.to_string());
                    if let Some(s_line) = &status_line {
                        stat_line.push_str(&s_line);
                    }
                    continue;
                }
                if line.is_empty() {
                    break;
                }
                let mut parts = line.splitn(2, ": ");
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    headers_hash.insert(key.to_string(), value.to_string());
                }
            }
            return Ok(HttpResponse {
                status_line: stat_line,
                headers: headers_hash,
                body: body,
            });
        }
        Err(Error::ResponseParseError)

        //println!("String Data: {:?}", res);
    }
}

// impl HttpResponse {
//     pub fn new(stream: &mut TcpStream) {
//         let mut headers: HashMap<String, String> = HashMap::new();
//         let mut body: Option<vec<u8>> = None;
//         let mut buf: Vec<u8> = vec![];

//         match stream.read(buf) {

//         }
//     }
// }

fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:7800").unwrap();

    let url = "https://example.com";
    if let Ok(p_url) = URL::from(url) {
        //let query = dns::resolve::Query::new();
        //let ip_addr = Query::send_query(p_url.host).unwrap();
        //let constructed_url = format!("{}", ip_addr);
        //println!("Ip Address: {:?}", constructed_url);
        if let Ok(mut conn) = ClientConnection::new(&p_url.host) {
            let response = conn.send_request().unwrap();
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        handle_client(s, &response);
                    }
                    Err(e) => {
                        println!("Error connection: {}", e);
                    }
                }
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, response: &HttpResponse) {
    let mut buffer = [0; 1024];
    while match stream.read(&mut buffer) {
        Ok(s) => {
            let status_line = &response.status_line;
            stream.write(status_line.as_bytes()).unwrap();
            return;
        }
        Err(e) => {
            println!("Error: {}", e);
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}
