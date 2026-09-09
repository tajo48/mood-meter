use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const INDEX: &str = include_str!("../index.html");

fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3002);

    let listener = match TcpListener::bind((host.as_str(), port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("mood_meter: failed to bind {host}:{port}: {err}");
            return Err(err);
        }
    };
    eprintln!("mood_meter listening on http://{host}:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle(stream));
            }
            Err(err) => eprintln!("accept failed: {err}"),
        }
    }

    Ok(())
}

fn handle(mut stream: TcpStream) {
    let mut request = Vec::new();
    let mut buf = [0u8; 4096];

    // Requests here are tiny GETs; read until the end of the headers.
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return,
        };
        request.extend_from_slice(&buf[..n]);
        if request.len() > 65_536 || request.windows(4).any(|w| w == b"\r\n\r\n".as_slice()) {
            break;
        }
    }

    let head = String::from_utf8_lossy(&request);
    let mut parts = head.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    let (status, content_type, body) = match (method, path) {
        ("GET", "/") | ("GET", "/index.html") | ("HEAD", "/") | ("HEAD", "/index.html") => {
            ("200 OK", "text/html; charset=utf-8", INDEX)
        }
        _ => ("404 Not Found", "text/plain; charset=utf-8", "not found\n"),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    if method != "HEAD" {
        let _ = stream.write_all(body.as_bytes());
    }
    let _ = stream.flush();
}
