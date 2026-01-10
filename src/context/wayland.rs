use std::env;
use std::process::Command;

pub fn get_wayland_context() -> Option<(String, String)> {
    // Detect desktop environment/compositor
    let desktop = env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_uppercase();
    let session_desktop = env::var("GDMSESSION").unwrap_or_default().to_uppercase();

    if desktop.contains("SWAY") {
        return get_sway_context();
    } else if desktop.contains("HYPRLAND") {
        return get_hyprland_context();
    } else if desktop.contains("GNOME") || session_desktop.contains("GNOME") {
        return get_gnome_context();
    } else if desktop.contains("KDE") {
        return get_kde_context();
    }

    None
}

fn get_sway_context() -> Option<(String, String)> {
    let output = Command::new("swaymsg")
        .args(&["-t", "get_tree"])
        .output()
        .ok()?;

    // Parse JSON output manually or with regex to avoid heavy deps just for this if simple
    // But we probably have serde_json available.
    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    // Recursive search for "focused": true
    fn find_focused(node: &serde_json::Value) -> Option<(String, String)> {
        if node["focused"].as_bool() == Some(true) {
            let name = node["name"].as_str().unwrap_or("Unknown").to_string();
            let app_id = node["app_id"]
                .as_str()
                .or(node["window_properties"]["class"].as_str())
                .unwrap_or("Unknown")
                .to_string();
            return Some((name, app_id));
        }

        if let Some(nodes) = node["nodes"].as_array() {
            for child in nodes {
                if let Some(res) = find_focused(child) {
                    return Some(res);
                }
            }
        }
        // Floating nodes in Sway
        if let Some(nodes) = node["floating_nodes"].as_array() {
            for child in nodes {
                if let Some(res) = find_focused(child) {
                    return Some(res);
                }
            }
        }
        None
    }

    find_focused(&json)
}

fn get_hyprland_context() -> Option<(String, String)> {
    let output = Command::new("hyprctl")
        .args(&["activewindow", "-j"])
        .output()
        .ok()?;

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    let title = json["title"].as_str().unwrap_or("Unknown").to_string();
    let class = json["class"].as_str().unwrap_or("Unknown").to_string();

    Some((title, class))
}

fn get_gnome_context() -> Option<(String, String)> {
    // Using gdbus/shell eval - note: unsafe and often disabled
    let output = Command::new("gdbus")
        .args(&[
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/gnome/Shell",
            "--method",
            "org.gnome.Shell.Eval",
            "global.display.focus_window ? global.display.focus_window.get_title() : 'No Focus'",
        ])
        .output()
        .ok()?;

    let out_str = String::from_utf8_lossy(&output.stdout); // (true, "Window Title")
                                                           // Very rough parsing
    if out_str.contains("(true,") {
        let title = out_str.split('"').nth(1).unwrap_or("Unknown").to_string();
        return Some((title, "GNOME App".to_string()));
    }

    None
}

fn get_kde_context() -> Option<(String, String)> {
    // Try kdotool if available
    if Command::new("which")
        .arg("kdotool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        let title_output = Command::new("kdotool")
            .arg("getactivewindow")
            .arg("getwindowname")
            .output()
            .ok()?;
        let title = String::from_utf8_lossy(&title_output.stdout)
            .trim()
            .to_string();

        // kdotool doesn't easily get class, but maybe using getwindowclassname?
        // Let's assume title is enough or try to get class.
        // There is 'getwindowclassname' in some versions or 'getwindowpid' -> /proc.

        return Some((title, "KDE App".to_string()));
    }

    // Fallback: Check if we can use qdbus/scripting?
    // For now, return None to trigger X11 fallback (XWayland)
    None
}
