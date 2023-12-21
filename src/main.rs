use clap::Parser;
use nom::FindSubstring;
use std::{
    // Error handling
    error::Error,
    // File system related
    fs,
    // IO related
    io::{BufRead, BufReader, BufWriter, Read, Write},
    // Networking related
    net::{TcpListener, TcpStream},
    path::PathBuf,
};

const GET: &str = "GET";
const POST: &str = "POST";
const USER_AGENT: &str = "User-Agent:";

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
    Created(ContentType),
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
        let (status_code, body, content_type) = match self {
            StatusLine::Ok(body, content_type) => {
                let status_code = "200 OK";
                (status_code, body.clone(), content_type)
            }
            StatusLine::Created(content_type) => {
                let status_code = "201 Created";
                (status_code, None, content_type)
            }
            StatusLine::NotFound => {
                let status_code = "404 Not Found";
                return format!("HTTP/1.1 {}\r\n\r\n", status_code).into_bytes();
            }
        };

        let content_type_str = match content_type {
            ContentType::TextPlain => "text/plain",
            ContentType::ApplicationOctetStream => "application/octet-stream",
        };

        match body {
            Some(body) => {
                let content_length = body.len();
                format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                    status_code, content_type_str, content_length, body
                )
                .into_bytes()
            }
            None => format!("HTTP/1.1 {}\r\nContent-Type: {}\r\n\r\n", status_code, content_type_str).into_bytes(),
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
    let (mut buf_reader, mut buf_writer) = setup_streams(stream)?;
    let mut http_request = read_request(&mut buf_reader)?;
    process_request_body(&mut buf_reader, &mut http_request)?;
    let response = generate_response(&http_request)?;
    send_response(&mut buf_writer, &response)?;
    Ok(())
}

fn setup_streams(stream: TcpStream) -> Result<(BufReader<TcpStream>, BufWriter<TcpStream>), Box<dyn Error>> {
    let read_stream = stream.try_clone()?;
    let write_stream = stream.try_clone()?;
    let buf_reader = BufReader::new(read_stream);
    let buf_writer = BufWriter::new(write_stream);
    Ok((buf_reader, buf_writer))
}

fn read_request(buf_reader: &mut BufReader<TcpStream>) -> Result<Vec<String>, Box<dyn Error>> {
    let http_request: Vec<_> = buf_reader
        .by_ref()
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    Ok(http_request)
}

fn process_request_body(buf_reader: &mut BufReader<TcpStream>, http_request: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
    let content_length = find_content_length(http_request);
    let mut buffer = vec![0; content_length];
    buf_reader.read_exact(&mut buffer)?;
    let body = String::from_utf8_lossy(&buffer);
    http_request.push(body.to_string());
    Ok(())
}

fn generate_response(http_request: &[String]) -> Result<StatusLine, Box<dyn Error>> {
    if let Some(request_line) = http_request.get(0) {
        match resolve_path(request_line) {
            Some("/") => Ok(StatusLine::Ok(None, ContentType::TextPlain)),
            Some(path) => Ok(path_to_status_line(path, http_request)),
            _ => Ok(StatusLine::NotFound),
        }
    } else {
        eprintln!("Received empty request");
        Err("Empty request received".into())
    }
}

fn send_response(buf_writer: &mut BufWriter<TcpStream>, response: &StatusLine) -> Result<(), Box<dyn Error>> {
    buf_writer.write_all(&response.get_message())?;
    buf_writer.flush()?;
    Ok(())
}

fn path_to_status_line(path: &str, request_data: &[String]) -> StatusLine {
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

fn handle_file_path(file_path: &str, request_data: &[String]) -> StatusLine {
    let args = Args::parse();
    let full_path = args.directory.join(file_path);

    let request_type = request_data
        .iter()
        .next()
        .and_then(|s| s.split_whitespace().next());

    match request_type {
        Some(GET) => fs::read_to_string(&full_path)
            .map(|file_contents| StatusLine::Ok(Some(file_contents), ContentType::ApplicationOctetStream))
            .unwrap_or(StatusLine::NotFound),
        Some(POST) => {
            fs::write(&full_path, request_data.last().unwrap_or(&String::new())).unwrap();
            StatusLine::Created(ContentType::TextPlain)
        }
        _ => StatusLine::NotFound,
    }
}

fn handle_user_agent(request_data: &[String]) -> StatusLine {
    request_data
        .iter()
        .find(|header| header.contains(USER_AGENT))
        .map_or(StatusLine::NotFound, |user_agent| {
            StatusLine::Ok(
                user_agent
                    .strip_prefix(USER_AGENT)
                    .map(|s| s.to_string()),
                ContentType::TextPlain,
            )
        })
}

fn resolve_path(request: &str) -> Option<&str> {
    let request_header = request.lines().next()?;
    request_header.split_whitespace().nth(1)
}

fn find_content_length(headers: &[String]) -> usize {
    headers
        .iter()
        .find_map(|header| {
            if header.starts_with("Content-Length:") {
                header.split_whitespace().nth(1)?.parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}
