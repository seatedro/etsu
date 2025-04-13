use crate::error::Result;
use rdev::{listen, Event, EventType};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
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

/// Structure to control the input listener thread
pub struct InputListener {
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    stop_sender: mpsc::Sender<()>,
}

impl InputListener {
    pub fn stop(&self) {
        info!("Stopping input listener...");
        // Signal the thread to stop
        self.running.store(false, Ordering::SeqCst);

        // Send stop signal through channel
        if let Err(e) = self.stop_sender.send(()) {
            error!("Failed to send stop signal: {}", e);
        }
    }

    pub fn join(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            match handle.join() {
                Ok(()) => info!("Input listener thread joined successfully"),
                Err(_) => error!("Failed to join input listener thread"),
            }
        }
    }
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
pub async fn listen_for_input(tx: Sender<InputEvent>) -> Result<InputListener> {
    info!("Initializing rdev input listener...");

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    // Create a channel to signal the thread to stop
    let (stop_sender, stop_receiver) = mpsc::channel::<()>();

    let thread_handle = thread::spawn(move || {
        info!("Input listener thread started");

        let mut terminate_thread = false;

        // Spawn a thread to listen for stop signals
        let r_clone = Arc::clone(&running_clone);
        let stop_thread = thread::spawn(move || {
            if let Ok(_) = stop_receiver.recv() {
                info!("Stop signal received in input listener");
                // Process will terminate itself as soon as possible
                // Force a dummy event to break out of the listen loop
                let _ = rdev::simulate(&rdev::EventType::KeyRelease(rdev::Key::Space));
            }
        });

        // Create a callback that processes events and checks for stop signal
        let callback = move |event: Event| {
            // Check if we should terminate before processing
            if !running_clone.load(Ordering::SeqCst) {
                terminate_thread = true;
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
        match (terminate_thread, listen_result) {
            (true, _) => info!("Input listener stopped by request"),
            (false, Ok(_)) => info!("Input listener stopped normally"),
            (false, Err(err)) => error!("Input listener error: {:?}", err),
        }

        info!("Input listener thread exiting");
    });

    let listener = InputListener {
        running,
        thread_handle: Some(thread_handle),
        stop_sender,
    };

    info!("Input listener task spawned");
    Ok(listener)
}

