use serde_json::{Map, Value};

#[derive(Debug, Default, Clone)]
pub struct ParsedHttpMessage {
    pub method: Option<String>,
    pub url: Option<String>,
    pub hostname: Option<String>,
    pub status: Option<u16>,
    pub headers: Option<Value>,
    pub body: Option<String>,
}

pub fn parse_http_message(message: &str) -> ParsedHttpMessage {
    let (start_line, headers_text, body) = split_http_message(message);
    let headers = parse_http_headers(headers_text);

    if start_line.starts_with("HTTP/") {
        ParsedHttpMessage {
            status: parse_http_response_status(start_line),
            headers,
            body: Some(body.to_string()),
            ..Default::default()
        }
    } else {
        let mut parts = start_line.split_whitespace();
        let method = parts.next().map(|s| s.to_string());
        let target = parts.next().map(|s| s.to_string());
        let host = header_value(headers_text, "host");

        let hostname = target
            .as_deref()
            .and_then(host_from_absolute_url)
            .or_else(|| host.clone().map(strip_host_port));

        let url = target.as_ref().map(|target| {
            if target.starts_with("http://") || target.starts_with("https://") {
                target.clone()
            } else if let Some(host) = host.as_ref() {
                format!("http://{host}{target}")
            } else {
                target.clone()
            }
        });

        ParsedHttpMessage {
            method,
            url,
            hostname,
            headers,
            body: Some(body.to_string()),
            ..Default::default()
        }
    }
}

fn split_http_message(message: &str) -> (&str, &str, &str) {
    if let Some((head, body)) = message.split_once("\r\n\r\n") {
        if let Some((start_line, headers)) = head.split_once("\r\n") {
            return (start_line, headers, body);
        }
        return (head, "", body);
    }

    if let Some((head, body)) = message.split_once("\n\n") {
        if let Some((start_line, headers)) = head.split_once('\n') {
            return (start_line, headers, body);
        }
        return (head, "", body);
    }

    if let Some((start_line, headers)) = message.split_once('\n') {
        return (start_line, headers, "");
    }

    (message, "", "")
}

fn parse_http_response_status(start_line: &str) -> Option<u16> {
    start_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
}

fn parse_http_headers(headers_text: &str) -> Option<Value> {
    let headers = headers_text
        .lines()
        .take_while(|line| !line.is_empty())
        .filter_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            Some((header_name.trim().to_string(), Value::String(value.trim().to_string())))
        })
        .collect::<Map<String, Value>>();

    if headers.is_empty() {
        None
    } else {
        Some(Value::Object(headers))
    }
}

fn header_value(headers_text: &str, name: &str) -> Option<String> {
    headers_text.lines().find_map(|line| {
        let (header_name, value) = line.split_once(':')?;
        if header_name.eq_ignore_ascii_case(name) {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn host_from_absolute_url(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://")?.1;
    let authority = after_scheme.split(['/', '?', '#']).next()?;
    Some(strip_host_port(authority.to_string()))
}

fn strip_host_port(host: String) -> String {
    if let Some((without_port, port)) = host.rsplit_once(':') {
        if port.chars().all(|ch| ch.is_ascii_digit()) {
            return without_port.to_string();
        }
    }
    host
}
