use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use mr_hedgehog::api::server;

#[test]
fn test_ipc_server_lifecycle() {
    // 1. Start server in background thread
    let port = 4599; // Use non-standard port for test
    thread::spawn(move || {
        if let Err(e) = server::start_server(port) {
            eprintln!("Server failed: {}", e);
        }
    });

    // Give server a moment to start
    thread::sleep(Duration::from_millis(500));

    // 2. Connect client
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .expect("Failed to connect to server");
    
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    // 3. Send PONG
    let ping_cmd = r#"{"command": "PING"}"#;
    stream.write_all(ping_cmd.as_bytes()).unwrap();
    stream.write_all(b"\n").unwrap();

    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    
    println!("Response: {}", response);
    // Expect: {"status":"success","data":"PONG"}
    assert!(response.contains("PONG"));
    assert!(response.contains("success"));

    // 4. Send ANALYZE (mock/invalid path to check error handling)
    // We don't want to run full SCIP analysis in this unit test as it requires external tools.
    // We just verify protocol mechanics.
    let analyze_cmd = r#"{"command": "ANALYZE", "params": {"path": "/invalid/path/test", "engine": "scip"}}"#;
    stream.write_all(analyze_cmd.as_bytes()).unwrap();
    stream.write_all(b"\n").unwrap();

    response.clear();
    reader.read_line(&mut response).unwrap();
    println!("Response: {}", response);
    
    // Should return error because path doesn't exist
    assert!(response.contains("error"));
    assert!(response.contains("Workspace path not found"));

    // 5. Send SHUTDOWN
    // Note: SHUTDOWN triggers process exit, which kills the test process if running in same process space!
    // However, cargo test harness runs tests in threads. If server calls std::process::exit(0),
    // it will exit the ENTIRE test runner.
    // So for this test, we might skip sending SHUTDOWN to avoid aborting other tests,
    // or we assume this test runs in isolation.
    // Better to NOT send shutdown here to be safe, just close connection.
    // server.rs `handle_connection` loop breaks on connection close.
}
