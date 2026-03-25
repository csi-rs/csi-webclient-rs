use crate::core::messages::{ApiRequest, ApiResponseEvent, CoreEvent, HttpMethod};
use serde::Deserialize;
use serde_json::Value;

/// Best-effort parser for the common webserver API envelope.
#[derive(Debug, Deserialize)]
struct GenericApiResponse {
    #[allow(dead_code)]
    success: Option<bool>,
    message: Option<String>,
    data: Option<Value>,
}

/// Execute one API request and convert the result into a [`CoreEvent`].
///
/// This function never panics for expected network/protocol errors. Instead,
/// failures are converted into `CoreEvent::ApiResponse` with `success = false`.
pub async fn execute_api_request(req: ApiRequest) -> CoreEvent {
    let client = reqwest::Client::new();
    let url = format!("{}{}", req.base_url, req.path);

    let request_builder = match req.method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
    };

    let request_builder = if let Some(body) = req.body {
        request_builder.json(&body)
    } else {
        request_builder
    };

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let text = match response.text().await {
                Ok(t) => t,
                Err(err) => {
                    return CoreEvent::ApiResponse(ApiResponseEvent {
                        label: req.label,
                        success: false,
                        status,
                        message: format!("Failed reading response body: {err}"),
                        data: None,
                    });
                }
            };

            if text.trim().is_empty() {
                return CoreEvent::ApiResponse(ApiResponseEvent {
                    label: req.label,
                    success: (200..300).contains(&status),
                    status,
                    message: if (200..300).contains(&status) {
                        "Request completed".to_owned()
                    } else {
                        "Request failed".to_owned()
                    },
                    data: None,
                });
            }

            let parsed_value = serde_json::from_str::<Value>(&text).ok();
            let parsed_envelope = serde_json::from_str::<GenericApiResponse>(&text).ok();
            let success = (200..300).contains(&status);

            let message = parsed_envelope
                .as_ref()
                .and_then(|p| p.message.clone())
                .unwrap_or_else(|| {
                    if success {
                        "Request completed".to_owned()
                    } else {
                        text.chars().take(300).collect()
                    }
                });

            let data = parsed_envelope
                .as_ref()
                .and_then(|p| p.data.clone())
                .or(parsed_value);

            CoreEvent::ApiResponse(ApiResponseEvent {
                label: req.label,
                success,
                status,
                message,
                data,
            })
        }
        Err(err) => CoreEvent::ApiResponse(ApiResponseEvent {
            label: req.label,
            success: false,
            status: 0,
            message: format!("Network error: {err}"),
            data: None,
        }),
    }
}
