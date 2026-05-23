use chrono::{DateTime, Duration, NaiveTime, TimeZone, Timelike, Utc};
use regex::Regex;
use vetty_common::http::parse_http_message;
use vetty_common::{EventType, SandboxEvent};

pub struct StraceParser {
    line_re: Regex,
    port_re: Regex,
    ipv4_re: Regex,
    ipv6_re: Regex,
}

impl StraceParser {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            line_re: Regex::new(
                r#"^\s*(?:(\d+)\s+)?([0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.\d+)?|\d+(?:\.\d+)?)\s+([a-zA-Z_][a-zA-Z0-9_]*)\((.*)\)(?:\s+=\s+(-?\d+)|.*<unfinished \.\.\.>)"#,
            )?,
            port_re: Regex::new(r#"(?:sin|sin6)_port=htons\((\d+)\)"#)?,
            ipv4_re: Regex::new(r#"inet_addr\("([^"]+)"\)"#)?,
            ipv6_re: Regex::new(r#"inet_pton\(AF_INET6,\s*"([^"]+)""#)?,
        })
    }

    pub fn parse_line(&self, line: &str) -> Option<SandboxEvent> {
        if line.contains("resumed>") || line.starts_with("--- ") || line.starts_with("+++ ") {
            return None;
        }

        let caps = self.line_re.captures(line)?;
        let pid: u32 = caps.get(1)?.as_str().parse().ok()?;
        let timestamp = parse_timestamp(caps.get(2)?.as_str())?;
        let syscall_name = caps.get(3)?.as_str().to_string();
        let args = caps.get(4)?.as_str();
        if should_drop_syscall(&syscall_name, args) {
            return None;
        }

        // Return value might be missing if unfinished
        let return_value: Option<i64> = caps.get(5).and_then(|m| m.as_str().parse().ok());

        let event_type = classify_syscall(&syscall_name, args);
        let mut event = SandboxEvent {
            timestamp,
            pid,
            event_type,
            syscall_name: Some(syscall_name.clone()),
            path: None,
            hostname: None,
            port: None,
            flags: None,
            return_value,
            http_method: None,
            http_url: None,
            http_status: None,
            http_headers: None,
            http_body: None,
            http_message: None,
            raw: Some(line.to_string()),
        };

        match event.event_type {
            EventType::FileAccess => {
                event.path = extract_first_quoted(args);
                event.flags = args.split(',').nth(1).map(|s| s.trim().to_string());
            }
            EventType::NetworkConnect => {
                event.port = self
                    .port_re
                    .captures(args)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<u16>().ok());
                event.hostname = self
                    .ipv4_re
                    .captures(args)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .or_else(|| {
                        self.ipv6_re
                            .captures(args)
                            .and_then(|c| c.get(1))
                            .map(|m| m.as_str().to_string())
                    });
            }
            EventType::ProcessSpawn => {
                event.path = extract_first_quoted(args);
            }
            EventType::HttpRequest => {
                let payload = extract_first_quoted(args);
                if let Some(payload) = payload {
                    let parsed = parse_http_message(&payload);
                    event.http_method = parsed.method;
                    event.http_url = parsed.url;
                    event.hostname = parsed.hostname;
                    event.http_status = parsed.status;
                    event.http_headers = parsed.headers;
                    event.http_body = parsed.body;
                    event.http_message = Some(payload);
                }
            }
            EventType::HttpResponse => {
                let payload = extract_first_quoted(args);
                if let Some(payload) = payload {
                    let parsed = parse_http_message(&payload);
                    event.http_status = parsed.status;
                    event.http_headers = parsed.headers;
                    event.http_body = parsed.body;
                    event.http_message = Some(payload);
                }
            }
            _ => {}
        }

        Some(event)
    }
}

fn extract_first_quoted(input: &str) -> Option<String> {
    extract_first_quoted_raw(input)
}

fn classify_syscall(syscall: &str, args: &str) -> EventType {
    if looks_like_http_request(syscall, args) {
        return EventType::HttpRequest;
    }

    if looks_like_http_response(syscall, args) {
        return EventType::HttpResponse;
    }

    match syscall {
        "open" | "openat" | "stat" | "lstat" | "access" | "readlink" | "unlink" | "rename"
        | "mkdir" | "rmdir" | "chmod" | "chown" => EventType::FileAccess,
        "connect" | "bind" | "accept" | "socket" | "sendto" | "recvfrom" | "sendmsg"
        | "recvmsg" => EventType::NetworkConnect,
        "execve" | "fork" | "clone" | "vfork" => EventType::ProcessSpawn,
        _ => EventType::Syscall,
    }
}

fn should_drop_syscall(syscall: &str, args: &str) -> bool {
    matches!(syscall, "read" | "readv")
        || (matches!(syscall, "write" | "writev") && !looks_like_http_request(syscall, args))
}

fn looks_like_http_request(syscall: &str, args: &str) -> bool {
    matches!(syscall, "sendto" | "sendmsg" | "write" | "writev") && {
        let payload = extract_first_quoted_raw(args).unwrap_or_default();
        matches!(
            payload.as_str(),
            s if s.starts_with("GET ")
                || s.starts_with("POST ")
                || s.starts_with("PUT ")
                || s.starts_with("PATCH ")
                || s.starts_with("DELETE ")
                || s.starts_with("HEAD ")
                || s.starts_with("OPTIONS ")
        )
    }
}

fn looks_like_http_response(syscall: &str, args: &str) -> bool {
    matches!(syscall, "recvfrom" | "recvmsg") && {
        let payload = extract_first_quoted_raw(args).unwrap_or_default();
        payload.starts_with("HTTP/")
    }
}

fn extract_first_quoted_raw(input: &str) -> Option<String> {
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            let mut buf = String::new();
            let mut escaped = false;
            for next in chars.by_ref() {
                if escaped {
                    match next {
                        'n' => buf.push('\n'),
                        'r' => buf.push('\r'),
                        't' => buf.push('\t'),
                        '\\' => buf.push('\\'),
                        '"' => buf.push('"'),
                        '0' => buf.push('\0'),
                        other => buf.push(other),
                    }
                    escaped = false;
                    continue;
                }
                match next {
                    '\\' => escaped = true,
                    '"' => return Some(buf),
                    other => buf.push(other),
                }
            }
            break;
        }
    }
    None
}

fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    if raw.contains(':') {
        let time = NaiveTime::parse_from_str(raw, "%H:%M:%S%.f").ok()?;
        let now = Utc::now();
        let midnight = now.date_naive().and_hms_opt(0, 0, 0)?;
        let delta = Duration::seconds(time.num_seconds_from_midnight() as i64)
            + Duration::nanoseconds(time.nanosecond() as i64);
        let naive = midnight.checked_add_signed(delta)?;
        Some(Utc.from_utc_datetime(&naive))
    } else {
        let epoch = raw.parse::<f64>().ok()?;
        let secs = epoch.trunc() as i64;
        let nanos = ((epoch.fract().abs()) * 1_000_000_000f64).round() as u32;
        DateTime::<Utc>::from_timestamp(secs, nanos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_http_request_from_write() {
        let parser = StraceParser::new().unwrap();
        let event = parser
            .parse_line(
                r#"123 12:00:00.000000 write(3, "GET /hello HTTP/1.1\r\nHost: example.com\r\nUser-Agent: curl/8\r\n\r\n", 67) = 67 <0.000012>"#,
            )
            .unwrap();

        assert_eq!(event.event_type, EventType::HttpRequest);
        assert_eq!(event.http_method.as_deref(), Some("GET"));
        assert_eq!(event.http_url.as_deref(), Some("http://example.com/hello"));
        assert_eq!(event.hostname.as_deref(), Some("example.com"));
        assert_eq!(
            event
                .http_headers
                .as_ref()
                .and_then(|value| value.get("Host")),
            Some(&serde_json::Value::String("example.com".to_string()))
        );
        assert_eq!(event.http_body.as_deref(), Some(""));
    }

    #[test]
    fn drops_plain_progress_write() {
        let parser = StraceParser::new().unwrap();
        let event = parser.parse_line(
            r#"293   01:38:44.789336 write(2, "remote: Counting objects:  15% (9/57)\33[K\r", 41) = 41 <0.002322>"#,
        );

        assert!(event.is_none());
    }

    #[test]
    fn parses_absolute_http_request_url() {
        let parser = StraceParser::new().unwrap();
        let event = parser
            .parse_line(
                r#"123 12:00:00.000000 sendto(3, "POST http://example.org:8080/api HTTP/1.1\r\nHost: ignored.test\r\n\r\n", 67, 0, NULL, 0) = 67 <0.000012>"#,
            )
            .unwrap();

        assert_eq!(event.event_type, EventType::HttpRequest);
        assert_eq!(event.http_method.as_deref(), Some("POST"));
        assert_eq!(
            event.http_url.as_deref(),
            Some("http://example.org:8080/api")
        );
        assert_eq!(event.hostname.as_deref(), Some("example.org"));
        assert_eq!(
            event
                .http_headers
                .as_ref()
                .and_then(|value| value.get("Host")),
            Some(&serde_json::Value::String("ignored.test".to_string()))
        );
    }

    #[test]
    fn drops_plain_http_response_from_read() {
        let parser = StraceParser::new().unwrap();
        let event = parser.parse_line(
            r#"123 12:00:00.000000 read(3, "HTTP/1.1 204 No Content\r\nServer: test\r\n\r\n", 8192) = 39 <0.000012>"#,
        );

        assert!(event.is_none());
    }

    #[test]
    fn parses_plain_http_response_from_recvfrom() {
        let parser = StraceParser::new().unwrap();
        let event = parser
            .parse_line(
                r#"123 12:00:00.000000 recvfrom(3, "HTTP/1.1 204 No Content\r\nServer: test\r\n\r\n", 8192, 0, NULL, NULL) = 39 <0.000012>"#,
            )
            .unwrap();

        assert_eq!(event.event_type, EventType::HttpResponse);
        assert_eq!(event.http_status, Some(204));
        assert_eq!(
            event
                .http_headers
                .as_ref()
                .and_then(|value| value.get("Server")),
            Some(&serde_json::Value::String("test".to_string()))
        );
        assert_eq!(event.http_body.as_deref(), Some(""));
    }
}
