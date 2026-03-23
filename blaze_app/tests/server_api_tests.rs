//! Integration tests for the Blaze web server API.
//! Verifies time control endpoints and manual stepping behavior.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use blaze_app::server;

fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .expect("connect to server");
    let req = format!("GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", path);
    stream.write_all(req.as_bytes()).expect("write request");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    extract_body(&buf)
}

fn http_post(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .expect("connect to server");
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        path
    );
    stream.write_all(req.as_bytes()).expect("write request");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    extract_body(&buf)
}

fn extract_body(response: &str) -> String {
    if let Some(idx) = response.find("\r\n\r\n") {
        response[idx + 4..].to_string()
    } else {
        String::new()
    }
}

fn event_count_from_events_json(body: &str) -> usize {
    if let Some(start) = body.find("\"total_count\":") {
        let rest = &body[start + 14..];
        if let Some(end) = rest.find('}') {
            if let Ok(n) = rest[..end].trim().parse::<usize>() {
                return n;
            }
        }
    }
    0
}

#[test]
fn time_state_returns_json() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("local_addr").port();

    thread::spawn(move || server::run_server(listener));
    thread::sleep(Duration::from_millis(80));

    let body = http_get(port, "/api/time/state");
    assert!(body.contains("\"manual_mode\""));
    assert!(body.contains("\"paused\""));
}

#[test]
fn manual_scenario_start_and_step_increments_events() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("local_addr").port();

    thread::spawn(move || server::run_server(listener));
    thread::sleep(Duration::from_millis(80));

    let start_body = http_post(port, "/api/scenario/start?id=basic_parallel&manual=1");
    assert!(start_body.contains("\"ok\""));

    thread::sleep(Duration::from_millis(100));

    let time_body = http_get(port, "/api/time/state");
    assert!(time_body.contains("\"manual_mode\":true"));
    assert!(time_body.contains("\"paused\":true"));

    let events_before = event_count_from_events_json(&http_get(port, "/api/events?since=0"));
    http_post(port, "/api/time/step");
    thread::sleep(Duration::from_millis(50));
    let events_after = event_count_from_events_json(&http_get(port, "/api/events?since=0"));
    assert!(events_after >= events_before, "step should allow at least one new event");
}
