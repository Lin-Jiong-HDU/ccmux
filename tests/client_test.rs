use ccmux::client::Client;

#[test]
fn test_client_creation() {
    let client = Client::new();
    assert!(client.is_ok());
}

#[test]
fn test_socket_path() {
    let client = Client::new().unwrap();
    // Socket path should end with ccmux.sock
    assert!(client
        .socket_path()
        .to_string_lossy()
        .ends_with("ccmux.sock"));
}
