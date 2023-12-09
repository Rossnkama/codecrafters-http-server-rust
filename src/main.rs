use std::{
    error::Error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

enum ResponseKind {
    Ok,
    NotFound,
}

trait Message {
    fn get_message(&self) -> &[u8];
}

impl Message for ResponseKind {
    fn get_message(&self) -> &[u8] {
        match self {
            ResponseKind::Ok => b"HTTP/1.1 200 OK\r\n\r\n",
            ResponseKind::NotFound => b"HTTP/1.1 404 Not Found\r\n\r\n",
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
            handle_connection(stream)?;
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 512];
    stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer);
    println!("Request: {} \n\n *** *** *** *** \n", request);

    let response = match resolve_path(&request) {
        Some("/") => ResponseKind::Ok,
        _ => ResponseKind::NotFound,
    };

    stream.write(response.get_message())?;
    stream.flush()?;
    Ok(())
}

fn resolve_path(request: &str) -> Option<&str> {
    let request_header = request.lines().next()?;
    request_header.split_whitespace().nth(1)
}
