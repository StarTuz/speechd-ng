use crate::cortex::Cortex;
use crate::engine::AudioOutput;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::time::sleep;
use zbus::{Connection, MatchRule};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProactiveEvent {
    BatteryLow,
    HighCpu,
    SystemIdle,
    Timer { message: String },
    Notification { title: String, body: String },
}

#[derive(Debug, Clone)]
struct Timer {
    deadline: Instant,
    message: String,
}

#[derive(Clone)]
pub struct ProactiveManager {
    cortex: Cortex,
    engine: Arc<dyn AudioOutput + Send + Sync>,
    last_speech: Arc<Mutex<Instant>>,
    timers: Arc<Mutex<Vec<Timer>>>,
}

impl ProactiveManager {
    pub fn new(cortex: Cortex, engine: Arc<dyn AudioOutput + Send + Sync>) -> Self {
        Self {
            cortex,
            engine,
            last_speech: Arc::new(Mutex::new(Instant::now())),
            timers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_timer(&self, duration: Duration, message: String) {
        println!("Proactive: Timer set for {:?}: '{}'", duration, message);
        let mut timers = self.timers.lock().unwrap();
        timers.push(Timer {
            deadline: Instant::now() + duration,
            message,
        });
    }

    pub fn reset_rate_limit(&self) {
        let mut last = self.last_speech.lock().unwrap();
        *last = Instant::now() - Duration::from_secs(60);
    }

    pub async fn start_timers(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            loop {
                // Sleep first to avoid tight loop on start
                sleep(Duration::from_secs(1)).await;

                let mut expired_timers;
                {
                    let mut timers = manager.timers.lock().unwrap();
                    let now = Instant::now();
                    // Drain expired timers
                    let (expired, remaining): (Vec<Timer>, Vec<Timer>) =
                        timers.drain(..).partition(|t| t.deadline <= now);
                    *timers = remaining;
                    expired_timers = expired;
                }

                for timer in expired_timers {
                    manager
                        .trigger_event(ProactiveEvent::Timer {
                            message: timer.message,
                        })
                        .await;
                }
            }
        });
    }

    pub async fn start_system_monitor(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            // Use new_all for simplicity and compatibility
            let mut sys = System::new_all();

            // Initial refresh to get baseline
            sys.refresh_all();
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

            loop {
                // Refresh stats
                sys.refresh_all();

                let cpu_usage = sys.global_cpu_usage();
                let ram_used = sys.used_memory();
                let ram_total = sys.total_memory();
                let ram_percent = (ram_used as f64 / ram_total as f64) * 100.0;

                // Heuristic: High CPU > 90%
                if cpu_usage > 90.0 {
                    manager.trigger_event(ProactiveEvent::HighCpu).await;
                }

                // Heuristic: High RAM > 95% (Map to 'HighCpu' message or add new event later)
                // For now, let's just log it or maybe trigger HighCpu as a generic "System Load" warning
                if ram_percent > 95.0 {
                    // manager.trigger_event(ProactiveEvent::HighMemory).await; // Need to add to enum
                    println!("Proactive: High Memory Usage: {:.1}%", ram_percent);
                }

                // Check for system idle (simple check, just placeholders for now)

                // Sleep
                sleep(Duration::from_secs(60)).await;
            }
        });
    }

    pub async fn start_notification_monitor(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let connection = match Connection::session().await {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("Proactive: Failed to connect to session bus: {}", e);
                    return;
                }
            };

            // Monitor Notify calls
            let rule_str = "type='method_call',interface='org.freedesktop.Notifications',member='Notify',eavesdrop='true'";
            let rule: zbus::MatchRule = match rule_str.try_into() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Proactive: Invalid match rule: {}", e);
                    return;
                }
            };

            // Manually call AddMatch on DBus driver
            let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await.unwrap();
            if let Err(e) = dbus_proxy.add_match_rule(rule).await {
                eprintln!("Proactive: Failed to add match rule: {}", e);
            }

            let mut stream = zbus::MessageStream::from(&connection);
            while let Some(msg_result) = stream.next().await {
                if let Ok(msg) = msg_result {
                    let header = msg.header();
                    if header.interface().map(|i| i.as_str())
                        == Some("org.freedesktop.Notifications")
                        && header.member().map(|m| m.as_str()) == Some("Notify")
                    {
                        // Signature: susssasa{sv}i
                        type NotifyArgs = (
                            String,
                            u32,
                            String,
                            String,
                            String,
                            Vec<String>,
                            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
                            i32,
                        );

                        if let Ok((_app, _id, _icon, summary, body, _actions, _hints, _expire)) =
                            msg.body().deserialize::<NotifyArgs>()
                        {
                            manager
                                .trigger_event(ProactiveEvent::Notification {
                                    title: summary,
                                    body,
                                })
                                .await;
                        }
                    }
                }
            }
        });
    }

    pub async fn trigger_event(&self, event: ProactiveEvent) {
        println!("Proactive Event Triggered: {:?}", event);

        // Rate limiting logic
        {
            let mut last = self.last_speech.lock().unwrap();
            if last.elapsed() < Duration::from_secs(30) {
                // Too soon to speak again automatically
                println!("Proactive event suppressed due to rate limiting.");
                return;
            }
            *last = Instant::now();
        }

        // Generate prompt based on event type
        let prompt = match event {
            ProactiveEvent::BatteryLow => {
                "The system battery is critically low (5%). Warn the user immediately.".to_string()
            }
            ProactiveEvent::HighCpu => {
                "System CPU usage is abnormally high (>90%). Advise the user.".to_string()
            }
            ProactiveEvent::SystemIdle => {
                "The system has been idle for 10 minutes. Ask if the user is still there."
                    .to_string()
            }
            ProactiveEvent::Timer { message } => {
                format!("Timer expired: {}. Alert the user.", message)
            }
            ProactiveEvent::Notification { title, body } => format!(
                "Notification received: '{}' - '{}'. Read it out if important.",
                title, body
            ),
        };

        if prompt.is_empty() {
            return;
        }

        println!("Proactive Streaming Query: {}", prompt);

        // Streaming LLM Request (Predictive Synthesis)
        // This is the core of Phase 6: reduce TTFB by streaming into buffer
        let mut rx = self.cortex.query_stream(prompt, None).await;
        let mut sentence_buffer = String::new();

        while let Some(token) = rx.recv().await {
            sentence_buffer.push_str(&token);

            // Check for sentence delimiters
            // Simple heuristic, can be improved with regex or NLP library later
            if sentence_buffer.ends_with('.')
                || sentence_buffer.ends_with('?')
                || sentence_buffer.ends_with('!')
                || sentence_buffer.ends_with('\n')
            {
                let text_to_speak = sentence_buffer.trim().to_string();
                if !text_to_speak.is_empty() {
                    println!("Predictive Synthesis (Speaking Chunk): {}", text_to_speak);
                    self.engine.speak(&text_to_speak, None);
                }
                sentence_buffer.clear();
            }
        }

        // Flush remaining buffer
        let remaining = sentence_buffer.trim().to_string();
        if !remaining.is_empty() {
            self.engine.speak(&remaining, None);
        }
    }
}
