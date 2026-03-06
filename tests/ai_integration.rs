use mockito::Server;
use reqwest::Client;
use serial_test::serial;
use speechd_ng::backends::ollama::OllamaBackend;
use speechd_ng::backends::openai::OpenAIBackend;
use speechd_ng::backends::BrainBackend;
use speechd_ng::error::SpeechdError;

#[tokio::test]
#[serial]
async fn test_ollama_backend_prompt_success() {
    let mut server = Server::new_async().await;
    let url = server.url();

    let mock = server
        .mock("POST", "/api/generate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"response": "Hello from Mock Ollama"}"#)
        .create_async()
        .await;

    let backend = OllamaBackend::new(url, "test-model".into(), Client::new());
    let result = backend.prompt("sys", "ctx", "user").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello from Mock Ollama");
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_ollama_backend_prompt_error_status() {
    let mut server = Server::new_async().await;
    let url = server.url();

    let _mock = server
        .mock("POST", "/api/generate")
        .with_status(500)
        .create_async()
        .await;

    let backend = OllamaBackend::new(url, "test-model".into(), Client::new());
    let result = backend.prompt("sys", "ctx", "user").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SpeechdError::AiHttp { .. }));
    // Backward-compatible display
    assert_eq!(err.to_string(), "AI Error: HTTP 500 Internal Server Error");
}

#[tokio::test]
#[serial]
async fn test_openai_backend_prompt_success() {
    let mut server = Server::new_async().await;
    let url = server.url();

    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "choices": [
                {
                    "message": {
                        "content": "Hello from Mock OpenAI"
                    }
                }
            ]
        }"#,
        )
        .create_async()
        .await;

    let backend = OpenAIBackend::new(url, "test-model".into(), Client::new());
    let result = backend.prompt("sys", "ctx", "user").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello from Mock OpenAI");
    mock.assert_async().await;
}

#[tokio::test]
#[serial]
async fn test_ollama_backend_stream() {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Ollama stream is a series of JSON objects, typically separated by newlines
    let body = "{\"response\": \"Part 1\"}\n{\"response\": \"Part 2\"}\n";

    let _mock = server
        .mock("POST", "/api/generate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let backend = OllamaBackend::new(url, "test-model".into(), Client::new());
    let mut rx = backend.stream("sys", "ctx", "user", None).await;

    let mut response = String::new();
    while let Some(result) = rx.recv().await {
        if let Ok(token) = result {
            response.push_str(&token);
        }
    }

    assert_eq!(response, "Part 1Part 2");
}

#[tokio::test]
#[serial]
async fn test_openai_backend_stream() {
    let mut server = Server::new_async().await;
    let url = server.url();

    // OpenAI stream uses SSE format "data: <json>"
    let body = "data: {\"choices\": [{\"delta\": {\"content\": \"Hello\"}}]}\n\ndata: {\"choices\": [{\"delta\": {\"content\": \" World\"}}]}\n\ndata: [DONE]\n\n";

    let _mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(body)
        .create_async()
        .await;

    let backend = OpenAIBackend::new(url, "test-model".into(), Client::new());
    let mut rx = backend.stream("sys", "ctx", "user", None).await;

    let mut response = String::new();
    while let Some(result) = rx.recv().await {
        if let Ok(token) = result {
            response.push_str(&token);
        }
    }

    assert_eq!(response, "Hello World");
}

