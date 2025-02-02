use std::{
    net::IpAddr,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    sync::Arc,
    time::{Duration, Instant},
};
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tokio::runtime::Runtime;
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};

#[cfg(target_os = "macos")]
mod login_item;

// Configuration constants
const VALUE_HISTORY: usize = 10;
const PING_FAILED: u64 = u64::MAX;
const NO_DATA: u64 = 0;
const PING_PAYLOAD_SIZE: usize = 64;
// const PING_TARGET: &str = "8.8.8.8";  // Google DNS
const PING_TARGET: &str = "34.91.238.70";

struct LatencyHistory {
    values: [AtomicU64; VALUE_HISTORY],
    current_index: AtomicUsize,
}

impl LatencyHistory {
    fn new() -> Self {
        Self {
            values: array_init::array_init(|_| AtomicU64::new(0)),
            current_index: AtomicUsize::new(0),
        }
    }

    fn add(&self, value: u64) {
        let index = self.current_index.load(Ordering::Relaxed);
        self.values[index].store(value, Ordering::Relaxed);
        self.current_index
            .store((index + 1) % VALUE_HISTORY, Ordering::Relaxed);
    }

    fn latest(&self) -> u64 {
        let index = self.current_index.load(Ordering::Relaxed);
        let prev_index = if index == 0 {
            VALUE_HISTORY - 1
        } else {
            index - 1
        };
        self.values[prev_index].load(Ordering::Relaxed)
    }

    fn all_values(&self) -> Vec<u64> {
        let current = self.current_index.load(Ordering::Relaxed);
        let mut result = Vec::with_capacity(VALUE_HISTORY);
        for i in 0..VALUE_HISTORY {
            let index = (current + VALUE_HISTORY - i - 1) % VALUE_HISTORY;
            result.push(self.values[index].load(Ordering::Relaxed));
        }
        result
    }
}

// Icon generation
struct IconGenerator;

impl IconGenerator {
    fn create_x() -> Icon {
        let mut icon_data = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                let dx = x as f32 - 15.5;
                let dy = y as f32 - 15.5;

                let on_line = (dx - dy).abs() < 2.0 || (dx + dy).abs() < 2.0;
                icon_data.extend_from_slice(if on_line {
                    &[255, 0, 0, 255] // Red X
                } else {
                    &[0, 0, 0, 0] // Transparent
                });
            }
        }
        Icon::from_rgba(icon_data, 32, 32).expect("Failed to create icon")
    }

    fn create_circle(r: u8, g: u8, b: u8) -> Icon {
        let mut icon_data = Vec::with_capacity(32 * 32 * 4);
        let color = [r, g, b, 255];
        for y in 0..32 {
            for x in 0..32 {
                let dx = x as f32 - 15.5;
                let dy = y as f32 - 15.5;
                let distance = (dx * dx + dy * dy).sqrt();

                icon_data.extend_from_slice(if distance < 13.0 {
                    &color // Colored circle
                } else {
                    &[0, 0, 0, 0] // Transparent
                });
            }
        }
        Icon::from_rgba(icon_data, 32, 32).expect("Failed to create icon")
    }
}

// Latency monitoring and storage
#[derive(Clone)]
struct ThresholdConfig {
    yellow: Arc<AtomicU64>,
    red: Arc<AtomicU64>,
}

impl ThresholdConfig {
    fn new(yellow: u64, red: u64) -> Self {
        Self {
            yellow: Arc::new(AtomicU64::new(yellow)),
            red: Arc::new(AtomicU64::new(red)),
        }
    }

    fn set_thresholds(&self, yellow: u64, red: u64) {
        self.yellow.store(yellow, Ordering::Relaxed);
        self.red.store(red, Ordering::Relaxed);
    }

    fn get_thresholds(&self) -> (u64, u64) {
        (
            self.yellow.load(Ordering::Relaxed),
            self.red.load(Ordering::Relaxed),
        )
    }
}

#[derive(Clone)]
struct LatencyMonitor {
    history: Arc<LatencyHistory>,
    thresholds: ThresholdConfig,
    plane_mode: Arc<AtomicBool>,
    runtime: Arc<Runtime>,
}

impl LatencyMonitor {
    fn new(runtime: Runtime) -> Self {
        Self {
            history: Arc::new(LatencyHistory::new()),
            thresholds: ThresholdConfig::new(30, 100), // Default thresholds
            plane_mode: Arc::new(AtomicBool::new(false)),
            runtime: Arc::new(runtime),
        }
    }

    fn toggle_plane_mode(&self) -> bool {
        let new_state = !self.plane_mode.load(Ordering::Relaxed);
        self.plane_mode.store(new_state, Ordering::Relaxed);

        // Update thresholds based on mode
        if new_state {
            self.thresholds.set_thresholds(600, 1000); // Plane mode thresholds
        } else {
            self.thresholds.set_thresholds(30, 100); // Normal thresholds
        }
        new_state
    }

    fn start_monitoring(self) {
        std::thread::spawn(move || {
            self.runtime.block_on(async {
                self.run_ping_loop().await;
            });
        });
    }

    async fn run_ping_loop(&self) {
        let client = Client::new(&Config::default()).unwrap();
        let addr: IpAddr = PING_TARGET.parse().unwrap();
        let mut pinger = client.pinger(addr, PingIdentifier(111)).await;
        let payload = vec![0; PING_PAYLOAD_SIZE];
        let mut ping_sequence = 0;

        loop {
            let latency = match pinger.ping(PingSequence(ping_sequence), &payload).await {
                Ok(response) => response.1.as_millis() as u64,
                Err(_) => PING_FAILED,
            };
            self.history.add(latency);
            ping_sequence += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    fn get_status(&self) -> LatencyStatus {
        let current = self.history.latest();
        let all = self.history.all_values();
        LatencyStatus {
            current,
            history: all,
        }
    }
}

struct LatencyStatus {
    current: u64,
    history: Vec<u64>,
}

struct UiGenerator;

impl UiGenerator {
    fn create_sparkline(values: &[u64]) -> String {
        if values.is_empty() {
            return String::new();
        }

        let valid_values: Vec<_> = values
            .iter()
            .filter(|&&v| v != NO_DATA && v != PING_FAILED)
            .collect();

        if valid_values.is_empty() {
            return "▁▁▁▁▁".to_string();
        }

        let min = **valid_values.iter().min().unwrap_or(&&0);
        let max = **valid_values.iter().max().unwrap_or(&&100);
        let range = (max - min) as f64;

        const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        values
            .iter()
            .rev()
            .map(|&v| match v {
                NO_DATA => BLOCKS[0],
                PING_FAILED => '✖',
                v => {
                    let scaled = if range == 0.0 {
                        4
                    } else {
                        ((v - min) as f64 / range * 7.0).round() as usize
                    };
                    BLOCKS[scaled.min(7)]
                }
            })
            .collect()
    }

    fn get_color_for_latency(latency: u64, thresholds: &ThresholdConfig) -> (u8, u8, u8) {
        let (yellow, red) = thresholds.get_thresholds();
        match latency {
            PING_FAILED => (255, 0, 0),     // Red
            v if v < yellow => (0, 255, 0), // Green
            v if v < red => (255, 255, 0),  // Yellow
            _ => (255, 0, 0),               // Red
        }
    }

    fn format_latency_text(status: &LatencyStatus) -> String {
        if status.current == PING_FAILED {
            return "Latency: Failed!".to_string();
        }

        let history_text = status
            .history
            .iter()
            .take(5)
            .rev()
            .map(|&v| match v {
                NO_DATA => String::from("--"),
                PING_FAILED => String::from("✖"),
                v => v.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("Recent latencies: {} ms", history_text)
    }
}

enum UserEvent {
    MenuEvent(tray_icon::menu::MenuEvent),
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        use crate::login_item::add_to_login_items;
        use cocoa::appkit::{NSApplication, NSApplicationActivationPolicy};
        use cocoa::base::nil;

        unsafe {
            let app = NSApplication::sharedApplication(nil);
            app.setActivationPolicy_(
                NSApplicationActivationPolicy::NSApplicationActivationPolicyProhibited,
            );
        }
        match add_to_login_items() {
            Ok(_) => println!("Successfully added to login items"),
            Err(e) => eprintln!("Failed to add to login items: {}", e),
        }
    }

    let runtime = Runtime::new().unwrap();
    let monitor = LatencyMonitor::new(runtime);
    monitor.clone().start_monitoring();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let tray_menu = Menu::new();
    let latency_item = MenuItem::new("Latency: --ms", false, None);
    let sparkline_item = MenuItem::new("", false, None);
    let mode_toggle = CheckMenuItem::new("✈ Plane Mode", true, false, None);
    let quit_item = MenuItem::new("Quit", true, None);

    tray_menu
        .append_items(&[
            &latency_item,
            &sparkline_item,
            &PredefinedMenuItem::separator(),
            &mode_toggle,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .expect("Should append menu items");

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Ping Monitor")
        .with_icon(IconGenerator::create_circle(128, 128, 128))
        .build()
        .unwrap();

    let mut last_update = Instant::now();
    let monitor_clone = monitor.clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(last_update + Duration::from_secs(1));

        if let Event::UserEvent(UserEvent::MenuEvent(event)) = event {
            if event.id == mode_toggle.id() {
                monitor_clone.toggle_plane_mode();
            } else if event.id == quit_item.id() {
                *control_flow = ControlFlow::Exit;
            }
        }

        if last_update.elapsed() >= Duration::from_secs(1) {
            let status = monitor.get_status();

            // Update UI elements
            latency_item.set_text(UiGenerator::format_latency_text(&status));
            sparkline_item.set_text(UiGenerator::create_sparkline(&status.history));

            // Update icon
            let icon = if status.current == PING_FAILED {
                IconGenerator::create_x()
            } else {
                let color = UiGenerator::get_color_for_latency(status.current, &monitor.thresholds);
                IconGenerator::create_circle(color.0, color.1, color.2)
            };
            tray_icon.set_icon(Some(icon)).unwrap();

            // Update tooltip with current mode and thresholds
            let (yellow, red) = monitor.thresholds.get_thresholds();
            let mode = if monitor.plane_mode.load(Ordering::Relaxed) {
                "Plane Mode"
            } else {
                "Normal Mode"
            };
            let tooltip = if status.current == PING_FAILED {
                format!("Ping failed! ({mode})")
            } else {
                format!(
                    "Latency: {}ms ({mode}, Y: {}ms, R: {}ms)",
                    status.current, yellow, red
                )
            };
            tray_icon.set_tooltip(Some(&tooltip)).unwrap();

            last_update = Instant::now();
        }
    });
}
