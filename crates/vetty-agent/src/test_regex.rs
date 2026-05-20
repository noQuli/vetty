use regex::Regex;

fn main() {
    let line_re = Regex::new(r#"^\s*(?:(\d+)\s+)?([0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.\d+)?|\d+(?:\.\d+)?)\s+([a-zA-Z_][a-zA-Z0-9_]*)\((.*)\)\s+=\s+(-?\d+)"#).unwrap();
    let text = r#"23689 16:34:02.000000 execve("/usr/bin/ls", ["ls"], 0x7ffd524388e0 /* 59 vars */) = 0 <0.000300>"#;
    if let Some(caps) = line_re.captures(text) {
        println!("MATCH! pid={}, time={}, sys={}, args={}, ret={}", 
            caps.get(1).map_or("", |m| m.as_str()),
            caps.get(2).map_or("", |m| m.as_str()),
            caps.get(3).map_or("", |m| m.as_str()),
            caps.get(4).map_or("", |m| m.as_str()),
            caps.get(5).map_or("", |m| m.as_str()));
    } else {
        println!("NO MATCH");
    }
}
