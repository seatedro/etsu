use crate::error::Result;
use rdev::{listen, Event, EventType};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use tokio::sync::mpsc::Sender;
use tracing::{error, info};

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    KeyPress,
    MouseClick,
    MouseMove(i32, i32),
    Scroll(i32),
}

/// Convert rdev events to our internal event representation
fn convert_event(event: &Event) -> Option<InputEvent> {
    match event.event_type {
        EventType::KeyPress(_) => Some(InputEvent::KeyPress),
        EventType::ButtonPress(button) => {
            if button == rdev::Button::Left || button == rdev::Button::Right {
                Some(InputEvent::MouseClick)
            } else {
                None
            }
        }
        EventType::MouseMove { x, y } => Some(InputEvent::MouseMove(x as i32, y as i32)),
        EventType::Wheel { delta_y, .. } => {
            // Convert to absolute value for scroll steps
            let scroll_amount = if delta_y != 0 {
                delta_y.abs() as i32
            } else {
                0
            };
            if scroll_amount > 0 {
                Some(InputEvent::Scroll(scroll_amount))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Spawns a blocking task to run the input listener.
/// Returns a control structure that can be used to stop the listener.
pub async fn listen_for_input(tx: Sender<InputEvent>) -> Result<()> {
    info!("Initializing rdev input listener...");

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    thread::spawn(move || {
        info!("Input listener thread started");

        // Create a callback that processes events and checks for stop signal
        let callback = move |event: Event| {
            // Check if we should terminate before processing
            if !running_clone.load(Ordering::SeqCst) {
                return;
            }

            // Convert and send the event
            if let Some(input_event) = convert_event(&event) {
                if tx.try_send(input_event).is_err() {
                    // Channel might be full, just continue
                }
            }
        };

        // Start listening for events
        let listen_result = listen(callback);

        // If the thread is supposed to terminate, it's a success
        match listen_result {
            Ok(()) => info!("Input listener stopped normally"),
            Err(err) => error!("Input listener error: {:?}", err),
        }

        info!("Input listener thread exiting");
    });

    info!("Input listener task spawned");
    Ok(())
}
