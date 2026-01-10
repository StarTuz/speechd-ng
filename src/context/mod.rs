mod wayland;
mod x11;

use std::env;

pub struct EnvironmentalContext {
    pub active_window: String,
    pub active_app: String,
}

impl EnvironmentalContext {
    pub fn get_current() -> Self {
        let session_type = env::var("XDG_SESSION_TYPE")
            .unwrap_or_default()
            .to_lowercase();

        let ctx = if session_type.contains("wayland") {
            // Try Wayland native methods first
            wayland::get_wayland_context()
                // Fallback to X11 (XWayland) if native fails (e.g. running an X app in Wayland)
                .or_else(|| x11::get_x11_context())
        } else {
            // X11 only
            x11::get_x11_context()
        };

        let (window, app) =
            ctx.unwrap_or(("Unknown Window".to_string(), "Unknown App".to_string()));

        Self {
            active_window: window,
            active_app: app,
        }
    }

    pub fn to_prompt_fragment(&self) -> String {
        format!(
            "\n[ENVIRONMENTAL CONTEXT]\nActive Window: {}\nActive Application: {}\n",
            self.active_window, self.active_app
        )
    }
}
