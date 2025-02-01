use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem}, Icon};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use rand::Rng;
use std::{
    time::{Duration, Instant},
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
    net::IpAddr,
};
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tokio::runtime::Runtime;

fn create_colored_circle_icon(r: u8, g: u8, b: u8) -> Icon {
    let mut icon_data = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            // Calculate distance from center
            let dx = x as f32 - 15.5;
            let dy = y as f32 - 15.5;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // If within 13 pixels from center, draw colored circle
            if distance < 13.0 {
                icon_data.extend_from_slice(&[r, g, b, 255]);
            } else {
                // Transparent
                icon_data.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(icon_data, 32, 32).expect("Failed to create icon")
}

fn main() {
    let latency = Arc::new(AtomicU64::new(0));
    let latency_clone = latency.clone();

    // Create tokio runtime for async ping operations
    let rt = Runtime::new().unwrap();
    
    // Spawn background thread for pinging
    std::thread::spawn(move || {
        rt.block_on(async {
            let client = Client::new(&Config::default()).unwrap();
            let addr: IpAddr = "8.8.8.8".parse().unwrap();
            let mut pinger = client.pinger(addr, PingIdentifier(111)).await;
            let payload = vec![0; 64];

            let mut ping_sequence = 0;
            loop {
                if let Ok(response) = pinger.ping(PingSequence(ping_sequence), &payload).await {
                    let duration = response.1;
                    latency_clone.store(duration.as_millis() as u64, Ordering::Relaxed);
                }
                ping_sequence += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    });

    let event_loop = EventLoopBuilder::new().build();
    
    let tray_menu = Menu::new();
    // Add latency menu item that will be updated
    let latency_item = MenuItem::new("Latency: --ms", false, None);
    tray_menu.append(&latency_item);
    tray_menu.append(&MenuItem::new("Hello", true, None));

    // Create initial grey icon
    let grey_icon = create_colored_circle_icon(128, 128, 128);

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Ping Monitor")
        .with_icon(grey_icon)
        .build()
        .unwrap();

    let mut last_update = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(last_update + Duration::from_secs(1));

        if last_update.elapsed() >= Duration::from_secs(1) {
            // Read current latency
            let current_latency = latency.load(Ordering::Relaxed);
            
            // Update menu item text
            let menu_text = if current_latency == u64::MAX {
                "Latency: Failed!".to_string()
            } else {
                format!("Latency: {}ms", current_latency)
            };
            latency_item.set_text(&menu_text);
            
            // Color based on latency:
            // Green: < 50ms
            // Yellow: 50-100ms
            // Red: > 100ms or failed (u64::MAX)
            let (r, g, b) = if current_latency == u64::MAX {
                (255, 0, 0) // Red for failure
            } else if current_latency < 50 {
                (0, 255, 0) // Green for good
            } else if current_latency < 100 {
                (255, 255, 0) // Yellow for medium
            } else {
                (255, 0, 0) // Red for high latency
            };

            // Create and set new icon
            let new_icon = create_colored_circle_icon(r, g, b);
            tray_icon.set_icon(Some(new_icon)).unwrap();

            // Update tooltip
            let tooltip = if current_latency == u64::MAX {
                "Ping failed!".to_string()
            } else {
                format!("Latency: {}ms", current_latency)
            };
            tray_icon.set_tooltip(Some(&tooltip)).unwrap();

            last_update = Instant::now();
        }
    });
}
