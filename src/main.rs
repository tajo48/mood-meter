use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const INDEX: &str = include_str!("../index.html");

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 3002;

const USAGE: &str = "\
mood-meter — serves the mood meter

USAGE: mood-meter [OPTIONS] [PORT]

ARGS:
    PORT                Port to listen on (default: 3002)

OPTIONS:
    -p, --port <PORT>   Port to listen on
    -H, --host <HOST>   Address to bind (default: 0.0.0.0, all interfaces)
    -h, --help          Print this help

HOST/PORT environment variables provide the defaults; arguments win.";

fn main() -> std::io::Result<()> {
    let (host, port) = parse_args();

    let listener = match TcpListener::bind((host.as_str(), port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("mood_meter: failed to bind {host}:{port}: {err}");
            return Err(err);
        }
    };
    // 0.0.0.0 listens on every interface; show the LAN IP as the URL to use.
    let display_host = if host == "0.0.0.0" {
        local_ip().unwrap_or(host)
    } else {
        host
    };
    eprintln!("mood_meter listening on http://{display_host}:{port}");

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

fn local_ip() -> Option<String> {
    // "Connect" a UDP socket to a public address (no packets sent) to learn
    // which local IP faces the outside world.
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    Some(sock.local_addr().ok()?.ip().to_string())
}

fn parse_args() -> (String, u16) {
    let mut host = std::env::var("HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let mut port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "-p" | "--port" => match args.next() {
                Some(v) => port = parse_port(&v),
                None => fail("missing value for --port"),
            },
            "-H" | "--host" => match args.next() {
                Some(v) => host = v,
                None => fail("missing value for --host"),
            },
            other => {
                if let Some(v) = other.strip_prefix("--port=") {
                    port = parse_port(v);
                } else if let Some(v) = other.strip_prefix("--host=") {
                    host = v.to_string();
                } else {
                    port = parse_port(other);
                }
            }
        }
    }

    (host, port)
}

fn parse_port(s: &str) -> u16 {
    s.parse()
        .unwrap_or_else(|_| fail(&format!("invalid port: {s:?}")))
}

fn fail(msg: &str) -> ! {
    eprintln!("mood_meter: {msg}\n\n{USAGE}");
    std::process::exit(2);
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
