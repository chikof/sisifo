use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
};

const ADDR: &str = "0.0.0.0:6640";

fn main() {
    let listener = TcpListener::bind(ADDR).expect("failed to bind to address");
    println!("Listening on http://{ADDR}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    if let Err(e) = handle_connection(stream) {
                        eprintln!("connection error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
}

/// Reads the request line, routes it, and writes the HTTP response.
fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    drain_headers(&mut reader)?;

    let response = route(&request_line);
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Returns the appropriate HTTP response for the given request line.
fn route(request_line: &str) -> &'static str {
    let mut parts = request_line.split_whitespace();
    let _method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    match path {
        "/health" => http_200(r#"{"status":"ok"}"#),
        _ => HTTP_404,
    }
}

/// Reads and discards all remaining HTTP headers until the blank line.
fn drain_headers(reader: &mut BufReader<TcpStream>) -> std::io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line)?;

        if line == "\r\n" || line.is_empty() {
            break;
        }
    }
    Ok(())
}

/// Builds a minimal HTTP 200 response with a JSON body.
fn http_200(body: &'static str) -> &'static str {
    match body {
        r#"{"status":"ok"}"# => concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: 15\r\n",
            "Connection: close\r\n",
            "\r\n",
            r#"{"status":"ok"}"#,
        ),
        _ => HTTP_404,
    }
}

const HTTP_404: &str = concat!(
    "HTTP/1.1 404 Not Found\r\n",
    "Content-Type: text/plain\r\n",
    "Content-Length: 9\r\n",
    "Connection: close\r\n",
    "\r\n",
    "Not Found",
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_should_return_200_for_health_path() {
        let response = route("GET /health HTTP/1.1");
        assert!(response.starts_with("HTTP/1.1 200 OK"));
    }

    #[test]
    fn route_should_return_json_body_for_health_path() {
        let response = route("GET /health HTTP/1.1");
        assert!(response.contains(r#"{"status":"ok"}"#));
    }

    #[test]
    fn route_should_return_404_for_unknown_path() {
        let response = route("GET /unknown HTTP/1.1");
        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
    }

    #[test]
    fn route_should_return_404_for_root_path() {
        let response = route("GET / HTTP/1.1");
        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
    }

    #[test]
    fn route_should_return_404_for_empty_request_line() {
        let response = route("");
        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
    }
}
