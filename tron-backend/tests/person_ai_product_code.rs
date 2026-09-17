use crab_serverapi::{AuthorizedClientApi, AuthorizedClientContext};
use reqwest::Method;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn sdk_plain_authorized_and_gateway_requests_send_tron_product_code() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock Person-AI server");
    let address = listener.local_addr().expect("read mock server address");
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().await.expect("accept SDK request");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = stream.read(&mut chunk).await.expect("read request");
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
            assert!(request.contains("x-product-code: tron"), "{request}");
            stream
                .write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("write response");
        }
    });

    let api = AuthorizedClientApi::new(format!("http://{address}")).expect("create SDK client");
    let context = AuthorizedClientContext::new()
        .with_token("token")
        .with_client_id("client")
        .with_public_key("public-key");
    let requests = [
        api.new_api_request(Method::GET, "/plain"),
        api.new_authorized_request(Method::GET, "/authorized", &context)
            .expect("build authorized request"),
        api.new_gateway_request(Method::GET, "/gateway", &context)
            .expect("build gateway request"),
    ];
    for request in requests {
        api.http_client()
            .execute(request.build().expect("build request"))
            .await
            .expect("send SDK request");
    }
    server.await.expect("mock server completes");
}
