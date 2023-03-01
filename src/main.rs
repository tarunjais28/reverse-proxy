use std::{
    collections::HashMap,
    io::prelude::*,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let cache: Arc<Mutex<HashMap<String, (Vec<u8>, Instant)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        let cache_clone = Arc::clone(&cache);

        thread::spawn(move || {
            let mut stream = stream.unwrap();
            let mut buffer = [0; 1024];
            stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..]);

            let url = match get_url(&request) {
                Some(url) => url,
                None => return,
            };

            let cache = cache_clone.lock().unwrap();

            if let Some((response, timestamp)) = cache.get(&url) {
                if timestamp.elapsed() < Duration::from_secs(30) {
                    stream.write(response).unwrap();
                    stream.flush().unwrap();
                    return;
                }
            }

            drop(cache);

            let mut origin_resp = get_origin_response(&url);

            let mut response = Vec::new();
            origin_resp.read_to_end(&mut response).unwrap();

            let timestamp = Instant::now();
            let cache_item = (response.clone(), timestamp.clone());

            cache_clone.lock().unwrap().insert(url.clone(), cache_item);

            stream.write(&response).unwrap();
            stream.flush().unwrap();
        });
    }
}

fn get_url(request: &str) -> Option<String> {
    let lines: Vec<&str> = request.lines().collect();
    let mut url = None;
    for line in lines {
        if line.starts_with("GET") {
            url = line.split_whitespace().nth(1);
            break;
        }
    }
    url.map(|s| s.to_string())
}

fn get_origin_response(url: &str) -> TcpStream {
    let host = url.split("/").nth(2).unwrap();
    let mut origin = TcpStream::connect((host, 80)).unwrap();
    let request = format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", url, host);
    origin.write(request.as_bytes()).unwrap();
    origin.flush().unwrap();
    origin
}
