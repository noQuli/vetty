use regex::Regex;

fn main() {
    let line_re = Regex::new(r#"^\s*(?:(\d+)\s+)?([0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.\d+)?|\d+(?:\.\d+)?)\s+([a-zA-Z_][a-zA-Z0-9_]*)\((.*)\)\s+=\s+(-?\d+)"#).unwrap();
    let texts = [
        r#"23689 16:34:02.000000 execve("/usr/bin/ls", ["ls"], 0x7ffd524388e0 /* 59 vars */) = 0 <0.000300>"#,
        r#"144 14:15:33.123 openat(AT_FDCWD, "/etc/ld.so.cache", O_RDONLY|O_CLOEXEC) = 3"#,
        r#"145 14:15:33.456 read(3, "...", 832) = 832"#,
        r#"146 14:15:33.789 close(3) = 0"#,
        r#"20000 00:00:00.000000 write(1, "hello\n", 6) = 6"#,
        r#"20000 00:00:00.000000 connect(3, {sa_family=AF_INET, sin_port=htons(80), sin_addr=inet_addr("1.1.1.1")}, 16) = -1 EINPROGRESS"#,
    ];
    for text in texts.iter() {
        if let Some(caps) = line_re.captures(text) {
            println!("MATCH! sys={}, ret={}", 
                caps.get(3).map_or("", |m| m.as_str()),
                caps.get(5).map_or("", |m| m.as_str()));
        } else {
            println!("NO MATCH: {}", text);
        }
    }
}
