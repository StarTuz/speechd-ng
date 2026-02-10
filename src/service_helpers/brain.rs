use crate::config_loader;
use serde_json::json;
use std::sync::Mutex;

/// Get the AI backend status: (is_running, active_model, available_models)
pub async fn get_brain_status(
    model_override: &Mutex<Option<String>>,
) -> (bool, String, Vec<String>) {
    let (backend, model_cfg, url_cfg) = config_loader::read_settings(
        |s| {
            (
                s.ai_backend.clone(),
                if s.ai_backend == "bitnet" {
                    s.bitnet_model.clone()
                } else {
                    s.ollama_model.clone()
                },
                if s.ai_backend == "bitnet" {
                    s.bitnet_url.clone()
                } else {
                    s.ollama_url.clone()
                },
            )
        },
        (
            "bitnet".to_string(),
            "models/bitnet_b1_58-3B".to_string(),
            "http://localhost:8000".to_string(),
        ),
    );

    let model = model_override
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .unwrap_or(model_cfg);

    let url = url_cfg;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    let mut available = Vec::new();
    let (ping_url, model_key) = if backend == "ollama" {
        (format!("{}/api/tags", url), "models")
    } else {
        (format!("{}/v1/models", url), "data")
    };

    let res = client.get(&ping_url).send().await;

    let is_running = match res {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json[model_key].as_array() {
                    for m in models {
                        let name = if backend == "ollama" {
                            m["name"].as_str()
                        } else {
                            m["id"].as_str()
                        };
                        if let Some(n) = name {
                            available.push(n.to_string());
                        }
                    }
                }
            }
            true
        }
        Err(_) => false,
    };

    (is_running, model, available)
}

/// Manage the AI brain service (start/stop/pull/use)
pub async fn manage_brain(
    action: &str,
    param: &str,
    model_override: &Mutex<Option<String>>,
) -> bool {
    let backend = config_loader::read_settings(|s| s.ai_backend.clone(), "bitnet".to_string());
    let service_name = if backend == "bitnet" {
        "bitnet"
    } else {
        "ollama"
    };

    match action {
        "start" => {
            let _ = std::process::Command::new("systemctl")
                .args(["start", service_name])
                .status();
            std::process::Command::new("systemctl")
                .args(["--user", "start", service_name])
                .status()
                .is_ok()
        }
        "stop" => {
            let _ = std::process::Command::new("systemctl")
                .args(["stop", service_name])
                .status();
            std::process::Command::new("systemctl")
                .args(["--user", "stop", service_name])
                .status()
                .is_ok()
        }
        "pull" => {
            if backend == "ollama" {
                let url = config_loader::read_settings(
                    |s| s.ollama_url.clone(),
                    "http://localhost:11434".to_string(),
                );
                let client = reqwest::Client::new();
                let res = client
                    .post(format!("{}/api/pull", url))
                    .json(&json!({"name": param, "stream": false}))
                    .send()
                    .await;
                res.is_ok()
            } else {
                println!(
                    "ManageBrain: 'pull' not supported for backend '{}'",
                    backend
                );
                false
            }
        }
        "use" => set_brain_model(param, model_override),
        _ => false,
    }
}

/// Set the active AI model override.
pub fn set_brain_model(model: &str, model_override: &Mutex<Option<String>>) -> bool {
    if model.is_empty() {
        return false;
    }

    if let Ok(mut override_lock) = model_override.lock() {
        let old = override_lock
            .clone()
            .unwrap_or_else(|| "<default>".to_string());
        *override_lock = Some(model.to_string());
        println!("Switched AI model: {} → {}", old, model);
        true
    } else {
        println!("SetBrainModel: Failed to acquire lock");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_brain_model_empty_rejected() {
        let model_override = Mutex::new(None);
        assert!(!set_brain_model("", &model_override));
    }

    #[test]
    fn test_set_brain_model_success() {
        let model_override = Mutex::new(None);
        assert!(set_brain_model("llama3", &model_override));
        let val = model_override.lock().unwrap();
        assert_eq!(val.as_deref(), Some("llama3"));
    }
}
