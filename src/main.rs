use std::{
    error::Error,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
};

enum ResponseKind {
    Ok(Option<String>),
    NotFound,
}

trait Message {
    fn get_message(&self) -> Vec<u8>;
}

impl Message for ResponseKind {
    fn get_message(&self) -> Vec<u8> {
        match self {
            ResponseKind::Ok(None) => b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
            ResponseKind::Ok(Some(body)) => {
                let content_length = body.len();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    content_length, body
                )
                .into_bytes()
            }
            ResponseKind::NotFound => b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec(),
        }
    }
}

fn main() {
    if let Err(e) = run_server() {
        eprintln!("error: {}", e);
    }
}

fn run_server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;

    println!("Server up!");

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let _ = handle_connection(stream);
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let read_stream = stream.try_clone()?;
    let write_stream = stream.try_clone()?;
    let buf_reader = BufReader::new(read_stream);
    let mut buf_writer = BufWriter::new(write_stream);
    let http_request: Vec<String> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);

    let response = match resolve_path(&http_request[0]) {
        Some("/") => ResponseKind::Ok(None),
        Some(path) => {
            if path.starts_with("/echo/") {
                ResponseKind::Ok(path.strip_prefix("/echo/").map(|s| s.to_string()))
            } else {
                ResponseKind::NotFound
            }
        }
        _ => ResponseKind::NotFound,
    };

    buf_writer.write_all(&response.get_message())?;
    stream.flush()?;
    Ok(())
}

fn resolve_path(request: &str) -> Option<&str> {
    let request_header = request.lines().next()?;
    request_header.split_whitespace().nth(1)
}
