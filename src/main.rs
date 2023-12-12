use std::{
    error::Error,
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
};

enum StatusLine {
    Ok(Option<String>),
    NotFound,
}

trait Message {
    fn get_message(&self) -> Vec<u8>;
}

impl Message for StatusLine {
    fn get_message(&self) -> Vec<u8> {
        match self {
            StatusLine::Ok(None) => b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
            StatusLine::Ok(Some(body)) => {
                let content_length = body.len();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    content_length, body
                )
                .into_bytes()
            }
            StatusLine::NotFound => b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec(),
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

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let read_stream = stream.try_clone()?;
    let write_stream = stream.try_clone()?;
    let buf_reader = BufReader::new(read_stream);
    let mut buf_writer = BufWriter::new(write_stream);

    let http_request: Result<Vec<String>, _> = buf_reader
        .lines()
        .collect();

    let http_request = match http_request {
        Ok(request) => request,
        Err(e) => {
            eprintln!("Failed to read request: {}", e);
            return Ok(());
        }
    };

    println!("Request: {:#?}", http_request);

    let response = if let Some(request_line) = http_request.get(0) {
        match resolve_path(request_line) {
            Some("/") => StatusLine::Ok(None),
            Some(path) => path_to_statusline(path),
            _ => StatusLine::NotFound,
        }
    } else {
        eprintln!("Received empty request");
        return Ok(());
    };

    buf_writer.write_all(&response.get_message())?;
    buf_writer.flush()?;
    Ok(())
}

fn path_to_statusline(path: &str) -> StatusLine {
    match path.strip_prefix("/echo/") {
        Some(s) => StatusLine::Ok(Some(s.to_string())),
        None => match path.strip_prefix("/file/") {
            Some(file_path) => fs::read_to_string(file_path)
                .map_or(StatusLine::NotFound, |file_contents| StatusLine::Ok(Some(file_contents))),
            None => StatusLine::NotFound,
        },
    }
}

fn resolve_path(request: &str) -> Option<&str> {
    let request_header = request.lines().next()?;
    request_header.split_whitespace().nth(1)
}
