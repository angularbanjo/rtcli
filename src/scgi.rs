use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;

use crate::error::{Error, Result};

pub fn send_request(url: &str, body: &[u8]) -> Result<Vec<u8>> {
    let content_length = body.len().to_string();
    let headers: Vec<(&str, &str)> = vec![
        ("CONTENT_LENGTH", &content_length),
        ("SCGI", "1"),
        ("REQUEST_METHOD", "POST"),
        ("REQUEST_URI", "/RPC2"),
        ("CONTENT_TYPE", "application/json"),
    ];

    let mut header_bytes = Vec::new();
    for (k, v) in &headers {
        header_bytes.extend_from_slice(k.as_bytes());
        header_bytes.push(0);
        header_bytes.extend_from_slice(v.as_bytes());
        header_bytes.push(0);
    }

    let mut request = Vec::new();
    request.extend_from_slice(header_bytes.len().to_string().as_bytes());
    request.push(b':');
    request.extend_from_slice(&header_bytes);
    request.push(b',');
    request.extend_from_slice(body);

    let mut response = Vec::new();

    if url.contains(':') && !url.starts_with('/') {
        let mut stream = TcpStream::connect(url)?;
        stream.write_all(&request)?;
        stream.flush()?;
        stream.read_to_end(&mut response)?;
    } else {
        let mut stream = UnixStream::connect(url)?;
        stream.write_all(&request)?;
        stream.flush()?;
        stream.read_to_end(&mut response)?;
    }

    let response_str = String::from_utf8_lossy(&response);
    if let Some(pos) = response_str.find("\r\n\r\n") {
        Ok(response[pos + 4..].to_vec())
    } else {
        Err(Error::Scgi("missing HTTP header delimiter in response".into()))
    }
}
