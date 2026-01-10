use std::process::Command;

pub fn get_x11_context() -> Option<(String, String)> {
    // 1. Get active window ID
    let root_output = Command::new("xprop")
        .args(&["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .ok()?;

    let root_str = String::from_utf8_lossy(&root_output.stdout);
    let window_id = root_str
        .split("window id # ")
        .nth(1)
        .and_then(|s| s.trim().split_whitespace().next());

    if let Some(id) = window_id {
        // 2. Get window name and class
        let window_output = Command::new("xprop")
            .args(&["-id", id, "_NET_WM_NAME", "WM_CLASS"])
            .output()
            .ok()?;

        let window_str = String::from_utf8_lossy(&window_output.stdout);

        let mut name = "Unknown Window".to_string();
        let mut class = "Unknown App".to_string();

        for line in window_str.lines() {
            if line.contains("_NET_WM_NAME") {
                if let Some(n) = line
                    .split(" = \"")
                    .nth(1)
                    .and_then(|s| s.strip_suffix('\"'))
                {
                    name = n.to_string();
                }
            } else if line.contains("WM_CLASS") {
                if let Some(c) = line.split(", \"").nth(1).and_then(|s| s.strip_suffix('\"')) {
                    class = c.to_string();
                } else if let Some(c) = line
                    .split(" = \"")
                    .nth(1)
                    .and_then(|s| s.strip_suffix('\"'))
                {
                    class = c.to_string();
                }
            }
        }
        Some((name, class))
    } else {
        None
    }
}
