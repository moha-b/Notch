use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

fn serve_reply(response: String) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/usage", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut headers = Vec::new();
        let mut byte = [0];
        while !headers.ends_with(b"\r\n\r\n") {
            assert!(headers.len() < 16384);
            stream.read_exact(&mut byte).unwrap();
            headers.push(byte[0]);
        }
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(headers).unwrap()
    });
    (url, server)
}

#[test]
fn http_statuses_keep_vendor_errors_out_of_readings_and_messages() {
    for status in [200, 401, 403, 429, 503] {
        let body = r#"{"used":42,"detail":"private-vendor-response"}"#;
        let (url, server) = serve_reply(format!("HTTP/1.1 {status} Fixture\r\nRetry-After: 900\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
        let reading = get(request(&url, "fixture-token"));
        match status {
            200 => assert_eq!(reading.unwrap()["used"], 42),
            401 | 403 => assert!(matches!(reading, Err(Failure::NeedsAuth))),
            429 => assert!(matches!(reading, Err(Failure::RateLimited(900)))),
            _ => assert!(
                matches!(reading, Err(Failure::Unavailable(ref note)) if !note.contains("private-vendor-response"))
            ),
        }
        server.join().unwrap();
    }
}

#[test]
fn redirects_neither_forward_credentials_nor_accept_a_json_reading() {
    for authorization in ["bearer", "cookie"] {
        let destination = TcpListener::bind("127.0.0.1:0").unwrap();
        destination.set_nonblocking(true).unwrap();
        let (url, server) = serve_reply(format!("HTTP/1.1 302 Found\r\nLocation: http://{}/capture\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}", destination.local_addr().unwrap()));
        let outbound = if authorization == "bearer" {
            request(&url, "fixture-token")
        } else {
            json_request(&url).set("Cookie", "session=fixture-cookie")
        };
        assert!(get(outbound).is_err());
        let headers = server.join().unwrap();
        assert!(headers.contains(if authorization == "bearer" {
            "fixture-token"
        } else {
            "fixture-cookie"
        }));
        assert!(
            matches!(destination.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}
