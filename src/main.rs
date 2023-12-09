use std::{
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
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    println!("Server up!");

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                handle_connection(_stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    let response: ResponseKind;
    stream.read(&mut buffer).unwrap();
    let request = String::from_utf8_lossy(&buffer);
    println!("Request: {} \n\n *** *** *** *** \n", request);

    let request_header = resolve_path(&request);
    
    if request_header[1] == "/" {
        response = ResponseKind::Ok
    } else {
        response = ResponseKind::NotFound
    }
    
    stream.write(response.get_message()).unwrap();
    stream.flush().unwrap();
}

fn resolve_path(request: &str) -> Vec<&str> {
    let request_header = request.lines().next().unwrap_or("");
    request_header.split_whitespace().collect()
}