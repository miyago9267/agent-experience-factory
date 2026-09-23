use regex::Regex;
use serde_json::{json, Value};
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::sync::OnceLock;
use std::time::Duration;

const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const MODEL: &str = "jev-latest";
const MAX_SUMMARY_BYTES: usize = 1200;
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionDecision {
    Keep,
    Discard,
    Review,
}

impl RetentionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Discard => "discard",
            Self::Review => "review",
        }
    }
}

#[derive(Debug)]
pub struct Assessment {
    pub decision: RetentionDecision,
    pub choice: String,
    pub confidence: f64,
    pub model: String,
    pub probabilities: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JevError {
    MissingKey,
    UnsafeCandidate,
    InvalidThreshold,
    RequestTooLarge,
    Transport,
    InvalidResponse,
}

impl JevError {
    pub fn message(self) -> &'static str {
        match self {
            Self::MissingKey => "Jev API key is unavailable",
            Self::UnsafeCandidate => "candidate did not pass the local privacy filter",
            Self::InvalidThreshold => "Jev confidence threshold must be between 0 and 1",
            Self::RequestTooLarge => "Jev request exceeded the local size limit",
            Self::Transport => "Jev request failed or timed out",
            Self::InvalidResponse => "Jev returned an invalid decision",
        }
    }
}

pub fn evaluate(
    kinds: &[String],
    summary: &str,
    occurrences: usize,
    api_key: Option<&str>,
    min_confidence: f64,
) -> Result<Assessment, JevError> {
    let api_key = api_key
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or(JevError::MissingKey)?;
    if !min_confidence.is_finite() || !(0.0..=1.0).contains(&min_confidence) {
        return Err(JevError::InvalidThreshold);
    }
    let request = build_request(kinds, summary, occurrences)?;
    let body = serde_json::to_vec(&request).map_err(|_| JevError::InvalidResponse)?;
    if body.len() > MAX_REQUEST_BYTES {
        return Err(JevError::RequestTooLarge);
    }

    let response_bytes = send_request(ENDPOINT, api_key, &body)?;
    let response: Value =
        serde_json::from_slice(&response_bytes).map_err(|_| JevError::InvalidResponse)?;
    parse_assessment(&response, min_confidence)
}

fn send_request(endpoint: &str, api_key: &str, body: &[u8]) -> Result<Vec<u8>, JevError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(1500))
        .redirects(0)
        .build();
    let response = agent
        .post(endpoint)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_bytes(body)
        .map_err(|_| JevError::Transport)?;
    let mut response_bytes = Vec::new();
    response
        .into_reader()
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut response_bytes)
        .map_err(|_| JevError::InvalidResponse)?;
    if response_bytes.len() > MAX_RESPONSE_BYTES {
        return Err(JevError::InvalidResponse);
    }
    Ok(response_bytes)
}

fn build_request(kinds: &[String], summary: &str, occurrences: usize) -> Result<Value, JevError> {
    if summary.trim().is_empty()
        || summary.trim().len() > MAX_SUMMARY_BYTES
        || summary.chars().any(char::is_control)
        || contains_sensitive_content(summary)
    {
        return Err(JevError::UnsafeCandidate);
    }

    Ok(json!({
        "model": MODEL,
        "state": {
            "summary": summary.trim(),
            "kinds": kinds,
            "occurrences": occurrences,
        },
        "questions": {
            "retention": {
                "type": "choice",
                "instructions": "Decide whether this short, already summarized work experience is worth keeping in a portable experience library for future sessions. Do not judge factual correctness; judge only durable reuse value.",
                "criteria": {
                    "keep": "A reusable method, confirmed preference, recurring pitfall, or lesson likely to improve future work. It may remain limited to its recorded project scope.",
                    "discard": "Transient status, one-time detail, redundant note, or information unlikely to improve future work.",
                    "review": "The summary is ambiguous, conflicting, unsupported, sensitive, or needs a human to choose its proper scope.",
                }
            }
        }
    }))
}

fn contains_sensitive_content(value: &str) -> bool {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            r"(?i)\b(?:sk-[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16})\b",
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
            r"(?i)\b(?:bearer|basic)\s+[A-Za-z0-9._~+/-]{8,}={0,2}",
            r"(?i)\b(?:api[_-]?key|auth[_-]?token|access[_-]?token|secret|password|passwd|pwd)\b\s*[:=：]\s*\S+",
            r"(?i)https?://[^\s<>]+|www\.[^\s<>]+",
            r"(?:/Users/|/private/|/home/|~/|[A-Za-z]:\\|(?:^|\s)\.\.?/)[^\s,;:]+",
            r"(?i)\b[\w.-]+\.(?:rs|swift|py|ts|tsx|js|jsx|sh|json|toml|ya?ml|lock|md|c|h|cpp|go|rb|java|kt)\b",
            r"\b(?:\d{1,3}\.){3}\d{1,3}\b",
            r"(?i)\b[\w.+-]+@[\w.-]+\.[a-z]{2,}\b",
            r"[`{}]",
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).expect("fixed Jev privacy pattern must compile"))
        .collect()
    });
    patterns.iter().any(|pattern| pattern.is_match(value))
}

fn parse_assessment(response: &Value, min_confidence: f64) -> Result<Assessment, JevError> {
    if !min_confidence.is_finite() || !(0.0..=1.0).contains(&min_confidence) {
        return Err(JevError::InvalidThreshold);
    }
    let model = response
        .get("model")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 80)
        .ok_or(JevError::InvalidResponse)?
        .to_string();
    let answer = response
        .get("answers")
        .and_then(|answers| answers.get("retention"))
        .ok_or(JevError::InvalidResponse)?;
    if answer.get("type").and_then(Value::as_str) != Some("choice") {
        return Err(JevError::InvalidResponse);
    }
    let choice = answer
        .get("choice")
        .and_then(Value::as_str)
        .ok_or(JevError::InvalidResponse)?;
    let raw_confidence = answer
        .get("confidence")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .ok_or(JevError::InvalidResponse)?;
    let probabilities = answer
        .get("probabilities")
        .and_then(Value::as_object)
        .ok_or(JevError::InvalidResponse)?;
    let labels = ["keep", "discard", "review"];
    let mut values = [0.0; 3];
    let mut probability_sum = 0.0;
    for (index, label) in labels.iter().enumerate() {
        let probability = probabilities
            .get(*label)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .ok_or(JevError::InvalidResponse)?;
        values[index] = probability;
        probability_sum += probability;
    }
    if (probability_sum - 1.0).abs() > 0.02 {
        return Err(JevError::InvalidResponse);
    }
    let raw_decision = match choice {
        "keep" => RetentionDecision::Keep,
        "discard" => RetentionDecision::Discard,
        "review" => RetentionDecision::Review,
        _ => return Err(JevError::InvalidResponse),
    };
    let choice_index = labels
        .iter()
        .position(|label| *label == choice)
        .ok_or(JevError::InvalidResponse)?;
    if values
        .iter()
        .enumerate()
        .any(|(index, value)| *value > values[choice_index] + 0.001 && index != choice_index)
    {
        return Err(JevError::InvalidResponse);
    }
    let decision = if raw_confidence < min_confidence {
        RetentionDecision::Review
    } else {
        raw_decision
    };
    Ok(Assessment {
        decision,
        choice: choice.to_string(),
        confidence: raw_confidence,
        model,
        probabilities: values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_contains_only_minimized_experience_fields() {
        let kinds = vec!["correction".to_string()];
        let request = build_request(&kinds, "先確認來源與實際生效位置", 2).unwrap();
        let state = request.get("state").unwrap();

        assert_eq!(state["summary"], "先確認來源與實際生效位置");
        assert_eq!(state["occurrences"], 2);
        assert_eq!(state["kinds"][0], "correction");
        assert!(state.get("scope").is_none());
        assert!(state.get("evidence").is_none());
        assert!(state.get("task_id").is_none());
        assert!(state.get("runtime").is_none());
        assert!(state.get("hypothesis").is_none());
    }

    #[test]
    fn privacy_gate_rejects_paths_credentials_urls_and_code() {
        for unsafe_summary in [
            "檢查 /tmp/private/project",
            "api_key=do-not-send-this",
            "部署網址 https://internal.example.test",
            "執行 `kubectl get secret`",
        ] {
            assert!(
                build_request(&[], unsafe_summary, 1).is_err(),
                "{unsafe_summary}"
            );
        }
    }

    #[test]
    fn jev_choice_is_bounded_and_low_confidence_routes_to_review() {
        let keep = json!({
            "model": "jev-test",
            "answers": { "retention": {
                "type": "choice", "choice": "keep", "confidence": 0.91,
                "probabilities": { "keep": 0.91, "discard": 0.04, "review": 0.05 }
            }}
        });
        assert_eq!(
            parse_assessment(&keep, 0.75).unwrap().decision,
            RetentionDecision::Keep
        );

        let uncertain = json!({
            "model": "jev-test",
            "answers": { "retention": {
                "type": "choice", "choice": "keep", "confidence": 0.62,
                "probabilities": { "keep": 0.62, "discard": 0.20, "review": 0.18 }
            }}
        });
        assert_eq!(
            parse_assessment(&uncertain, 0.75).unwrap().decision,
            RetentionDecision::Review
        );
    }

    #[test]
    fn jev_response_rejects_unknown_choices_and_invalid_probabilities() {
        let unknown = json!({
            "model": "jev-test",
            "answers": { "retention": {
                "type": "choice", "choice": "maybe", "confidence": 0.9,
                "probabilities": { "keep": 0.1, "discard": 0.1, "review": 0.8 }
            }}
        });
        assert!(parse_assessment(&unknown, 0.75).is_err());

        let malformed = json!({
            "model": "jev-test",
            "answers": { "retention": {
                "type": "choice", "choice": "keep", "confidence": 0.9,
                "probabilities": { "keep": 0.9, "discard": 0.9, "review": 0.1 }
            }}
        });
        assert!(parse_assessment(&malformed, 0.75).is_err());

        let mismatched = json!({
            "model": "jev-test",
            "answers": { "retention": {
                "type": "choice", "choice": "keep", "confidence": 0.9,
                "probabilities": { "keep": 0.1, "discard": 0.8, "review": 0.1 }
            }}
        });
        assert!(parse_assessment(&mismatched, 0.75).is_err());
    }

    #[test]
    fn jev_response_without_a_decision_is_rejected() {
        let no_choice = json!({
            "model": "jev-test",
            "answers": { "retention": {
                "type": "choice", "confidence": 0.9,
                "probabilities": { "keep": 0.9, "discard": 0.05, "review": 0.05 }
            }}
        });
        assert_eq!(
            parse_assessment(&no_choice, 0.75).unwrap_err(),
            JevError::InvalidResponse
        );
    }

    #[test]
    fn request_transport_uses_typed_payload() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 4096];
            let mut expected_length = None;
            loop {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if expected_length.is_none() {
                    if let Some(header_end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..header_end]);
                        expected_length = headers.lines().find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        });
                    }
                }
                if let (Some(header_end), Some(length)) = (
                    request.windows(4).position(|v| v == b"\r\n\r\n"),
                    expected_length,
                ) {
                    if request.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            let response_body = r#"{"model":"jev-test","answers":{"retention":{"type":"choice","choice":"keep","confidence":0.9,"probabilities":{"keep":0.9,"discard":0.05,"review":0.05}}}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            )
            .unwrap();
            request
        });

        let body = serde_json::to_vec(
            &build_request(&["correction".to_string()], "safe reusable lesson", 2).unwrap(),
        )
        .unwrap();
        let response = send_request(&format!("http://{address}"), "unit-test-key", &body).unwrap();
        let captured = String::from_utf8(server.join().unwrap()).unwrap();
        assert!(captured
            .to_ascii_lowercase()
            .contains("authorization: bearer unit-test-key"));
        assert!(captured.contains("safe reusable lesson"));
        assert!(!captured.contains("\"scope\":"));
        assert_eq!(
            parse_assessment(&serde_json::from_slice(&response).unwrap(), 0.75)
                .unwrap()
                .decision,
            RetentionDecision::Keep
        );
    }

    #[test]
    fn unavailable_service_is_reported_as_transport_failure() {
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });

        let result = send_request(&format!("http://{address}"), "test-only-key", b"{}");
        server.join().unwrap();
        assert!(matches!(result, Err(JevError::Transport)));
    }
}
