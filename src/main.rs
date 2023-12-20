use clap::Parser;
use nom::FindSubstring;
use std::{
    // Error handling
    error::Error,
    // File system related
    fs,
    // IO related
    io::{BufRead, BufReader, BufWriter, Write},
    // Networking related
    net::{TcpListener, TcpStream},
    path::PathBuf,
};

#[derive(Parser)]
#[clap(
    name = "HTTP Server",
    version = "1.0",
    author = "ross@dedsol.xyz",
    about = "A Simple HTTP Server"
)]
struct Args {
    #[clap(long, short)]
    directory: PathBuf,
}

enum StatusLine {
    Ok(Option<String>, ContentType),
    NotFound,
}

enum ContentType {
    TextPlain,
    ApplicationOctetStream,
}

trait Message {
    fn get_message(&self) -> Vec<u8>;
}

impl Message for StatusLine {
    fn get_message(&self) -> Vec<u8> {
        match self {
            StatusLine::Ok(None, _) => b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),
            StatusLine::Ok(Some(body), content_type) => {
                let content_type_str = match content_type {
                    ContentType::TextPlain => "text/plain",
                    ContentType::ApplicationOctetStream => "application/octet-stream",
                };
                let content_length = body.len();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                    content_type_str, content_length, body
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
            std::thread::spawn(move || {
                let _ = handle_connection(stream);
            });
        }
    }

    Ok(())
}

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let read_stream = stream.try_clone()?;
    let write_stream = stream.try_clone()?;
    let buf_reader = BufReader::new(read_stream);
    let mut buf_writer = BufWriter::new(write_stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);

    let response = if let Some(request_line) = http_request.get(0) {
        match resolve_path(request_line) {
            Some("/") => StatusLine::Ok(None, ContentType::TextPlain),
            Some(path) => path_to_status_line(path, &http_request),
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

fn path_to_status_line(path: &str, request_data: &Vec<String>) -> StatusLine {
    if let Some(s) = path.strip_prefix("/echo/") {
        return StatusLine::Ok(Some(s.to_string()), ContentType::TextPlain);
    }

    if let Some(file_path) = path.strip_prefix("/files/") {
        return handle_file_path(file_path, request_data);
    }

    if path.find_substring("/user-agent").is_some() {
        return handle_user_agent(request_data);
    }

    StatusLine::NotFound
}

fn handle_file_path(file_path: &str, request_data: &Vec<String>) -> StatusLine {
    let args = Args::parse();
    let mut full_path = args.directory.clone();
    full_path.push(file_path);

    let request_type = request_data
        .iter()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(0);

    if request_type == Some("GET") {
        fs::read_to_string(full_path).map_or(StatusLine::NotFound, |file_contents| {
            StatusLine::Ok(Some(file_contents), ContentType::ApplicationOctetStream)
        })
    } else {
        StatusLine::NotFound
    }
}

fn handle_user_agent(request_data: &Vec<String>) -> StatusLine {
    request_data
        .iter()
        .find(|header| header.contains("User-Agent:"))
        .map_or(StatusLine::NotFound, |user_agent| {
            StatusLine::Ok(
                user_agent
                    .strip_prefix("User-Agent: ")
                    .map(|s| s.to_string()),
                ContentType::TextPlain,
            )
        })
}

fn resolve_path(request: &str) -> Option<&str> {
    let request_header = request.lines().next()?;
    request_header.split_whitespace().nth(1)
}
