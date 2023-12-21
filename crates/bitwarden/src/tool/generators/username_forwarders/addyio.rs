use reqwest::{header::CONTENT_TYPE, StatusCode};

use crate::error::Result;
pub async fn generate(
    http: &reqwest::Client,
    api_token: String,
    domain: String,
    base_url: String,
    website: Option<String>,
) -> Result<String> {
    let description = super::format_description(&website);

    #[derive(serde::Serialize)]
    struct Request {
        domain: String,
        description: String,
    }

    let response = http
        .post(format!("{base_url}/api/v1/aliases"))
        .header(CONTENT_TYPE, "application/json")
        .bearer_auth(api_token)
        .header("X-Requested-With", "XMLHttpRequest")
        .json(&Request {
            domain,
            description,
        })
        .send()
        .await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err("Invalid addy.io API token.".into());
    }

    // Throw any other errors
    response.error_for_status_ref()?;

    #[derive(serde::Deserialize)]
    struct ResponseData {
        email: String,
    }
    #[derive(serde::Deserialize)]
    struct Response {
        data: ResponseData,
    }
    let response: Response = response.json().await?;

    Ok(response.data.email)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[tokio::test]
    async fn test_mock_server() {
        use wiremock::{matchers, Mock, ResponseTemplate};

        let (server, _client) = crate::util::start_mock(vec![
            // Mock the request to the addy.io API, and verify that the correct request is made
            Mock::given(matchers::path("/api/v1/aliases"))
                .and(matchers::method("POST"))
                .and(matchers::header("Content-Type", "application/json"))
                .and(matchers::header("Authorization", "Bearer MY_TOKEN"))
                .and(matchers::body_json(json!({
                    "domain": "myemail.com",
                    "description": "Website: example.com. Generated by Bitwarden."
                })))
                .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                    "data": {
                        "id": "50c9e585-e7f5-41c4-9016-9014c15454bc",
                        "user_id": "ca0a4e09-c266-4f6f-845c-958db5090f09",
                        "local_part": "50c9e585-e7f5-41c4-9016-9014c15454bc",
                        "domain": "myemail.com",
                        "email": "50c9e585-e7f5-41c4-9016-9014c15454bc@myemail.com",
                        "active": true
                    }
                })))
                .expect(1),
            // Mock an invalid API token request
            Mock::given(matchers::path("/api/v1/aliases"))
                .and(matchers::method("POST"))
                .and(matchers::header("Content-Type", "application/json"))
                .and(matchers::header("Authorization", "Bearer MY_FAKE_TOKEN"))
                .and(matchers::body_json(json!({
                    "domain": "myemail.com",
                    "description": "Website: example.com. Generated by Bitwarden."
                })))
                .respond_with(ResponseTemplate::new(401))
                .expect(1),
            // Mock an invalid domain
            Mock::given(matchers::path("/api/v1/aliases"))
                .and(matchers::method("POST"))
                .and(matchers::header("Content-Type", "application/json"))
                .and(matchers::header("Authorization", "Bearer MY_TOKEN"))
                .and(matchers::body_json(json!({
                    "domain": "gmail.com",
                    "description": "Website: example.com. Generated by Bitwarden."
                })))
                .respond_with(ResponseTemplate::new(403))
                .expect(1),
        ])
        .await;

        let address = super::generate(
            &reqwest::Client::new(),
            "MY_TOKEN".into(),
            "myemail.com".into(),
            format!("http://{}", server.address()),
            Some("example.com".into()),
        )
        .await
        .unwrap();

        let fake_token_error = super::generate(
            &reqwest::Client::new(),
            "MY_FAKE_TOKEN".into(),
            "myemail.com".into(),
            format!("http://{}", server.address()),
            Some("example.com".into()),
        )
        .await
        .unwrap_err();

        assert!(fake_token_error
            .to_string()
            .contains("Invalid addy.io API token."));

        let fake_domain_error = super::generate(
            &reqwest::Client::new(),
            "MY_TOKEN".into(),
            "gmail.com".into(),
            format!("http://{}", server.address()),
            Some("example.com".into()),
        )
        .await
        .unwrap_err();

        assert!(fake_domain_error.to_string().contains("403 Forbidden"));

        server.verify().await;
        assert_eq!(address, "50c9e585-e7f5-41c4-9016-9014c15454bc@myemail.com");
    }
}