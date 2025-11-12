use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use keyless_core::error::KeylessError;
use keyless_desktop_lib::services::models::{
    EventPublisher, ModelBackend, ModelsService, ModelsServiceImpl,
};
use keyless_models::download::DownloadEvent;

#[derive(Default)]
struct TestPublisher {
    events: Mutex<Vec<(String, Value)>>,
}

impl EventPublisher for TestPublisher {
    fn emit_json(&self, name: &str, payload: Value) {
        if let Ok(mut guard) = self.events.lock() {
            guard.push((name.to_string(), payload));
        }
    }
}

impl TestPublisher {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn recorded(&self) -> Vec<(String, Value)> {
        match self.events.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }
}

#[derive(Clone)]
enum MockEvent {
    Started,
    Progress {
        bytes: u64,
        total: Option<u64>,
        mbps: f64,
        eta_s: f64,
    },
    DoneOk,
}

struct MockBackend {
    remote: Vec<String>,
    local: Arc<Mutex<Vec<String>>>,
    event_sequence: Arc<Vec<MockEvent>>,
}

impl MockBackend {
    fn new(remote: Vec<String>, local: Vec<String>, events: Vec<MockEvent>) -> Arc<Self> {
        Arc::new(Self {
            remote,
            local: Arc::new(Mutex::new(local)),
            event_sequence: Arc::new(events),
        })
    }
}

impl ModelBackend for MockBackend {
    fn list_remote_models(&self) -> Result<Vec<String>, String> {
        Ok(self.remote.clone())
    }

    fn discover_local_models(&self) -> Vec<String> {
        match self.local.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }

    fn spawn_download(
        &self,
        model_id: String,
        tx: SyncSender<DownloadEvent>,
        cancel: Arc<AtomicBool>,
    ) {
        let events = Arc::clone(&self.event_sequence);
        let local = Arc::clone(&self.local);
        thread::spawn(move || {
            for event in events.iter() {
                if cancel.load(Ordering::Relaxed) {
                    let _ = tx.send(DownloadEvent::Done(Err(KeylessError::Other(
                        "cancelled".into(),
                    ))));
                    return;
                }
                match event.clone() {
                    MockEvent::Started => {
                        let _ = tx.send(DownloadEvent::Started {
                            model: model_id.clone(),
                        });
                    }
                    MockEvent::Progress {
                        bytes,
                        total,
                        mbps,
                        eta_s,
                    } => {
                        let _ = tx.send(DownloadEvent::Progress {
                            bytes,
                            total,
                            mbps,
                            eta_s,
                        });
                    }
                    MockEvent::DoneOk => {
                        if let Ok(mut guard) = local.lock() {
                            guard.push(model_id.clone());
                        }
                        let _ = tx.send(DownloadEvent::Done(Ok(())));
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
            // Check cancel flag one more time after all events processed
            // This allows cancellation to work even if events are exhausted
            for _ in 0..20 {
                if cancel.load(Ordering::Relaxed) {
                    let _ = tx.send(DownloadEvent::Done(Err(KeylessError::Other(
                        "cancelled".into(),
                    ))));
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
    }
}

fn wait_for_events() {
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn start_download_updates_status_and_emits_events() {
    let publisher = TestPublisher::new();
    let backend = MockBackend::new(
        vec!["openai/whisper-tiny".to_string()],
        vec![],
        vec![
            MockEvent::Started,
            MockEvent::Progress {
                bytes: 50,
                total: Some(100),
                mbps: 1.2,
                eta_s: 5.0,
            },
            MockEvent::DoneOk,
        ],
    );
    let service = ModelsServiceImpl::with_backend(publisher.clone(), backend.clone());

    match service.start_download("openai/whisper-tiny") {
        Ok(_) => {}
        Err(e) => panic!("unexpected error: {e}"),
    }
    wait_for_events();

    // status reflects completed download
    let status = match service.status("openai/whisper-tiny") {
        Ok(s) => s,
        Err(e) => panic!("unexpected error: {e}"),
    };
    assert!(status.downloaded);
    assert!(!status.downloading);
    assert!(status.error.is_none());

    // list_models uses backend and download status
    let models = match service.list_models() {
        Ok(m) => m,
        Err(e) => panic!("unexpected error: {e}"),
    };
    assert_eq!(models.len(), 1);
    assert!(models[0].downloaded);

    let recorded = publisher.recorded();
    let event_names: Vec<String> = recorded.iter().map(|(name, _)| name.clone()).collect();
    assert!(event_names.contains(&"download_started".to_string()));
    assert!(event_names.contains(&"download_progress".to_string()));
    assert!(event_names.contains(&"download_complete".to_string()));
}

#[test]
fn cancel_download_marks_entry_and_emits_event() {
    let publisher = TestPublisher::new();
    let backend = MockBackend::new(
        vec!["openai/whisper-small".to_string()],
        vec![],
        vec![MockEvent::Started],
    );
    let service = ModelsServiceImpl::with_backend(publisher.clone(), backend.clone());

    match service.start_download("openai/whisper-small") {
        Ok(_) => {}
        Err(e) => panic!("unexpected error: {e}"),
    }
    wait_for_events();
    match service.cancel_download("openai/whisper-small") {
        Ok(_) => {}
        Err(e) => panic!("unexpected error: {e}"),
    }
    wait_for_events();

    let status = match service.status("openai/whisper-small") {
        Ok(s) => s,
        Err(e) => panic!("unexpected error: {e}"),
    };
    assert!(!status.downloading);
    assert_eq!(status.error.as_deref(), Some("unknown error: cancelled"));

    let events = publisher.recorded();
    assert!(events.iter().any(|(name, _)| name == "download_cancelled"));
}
