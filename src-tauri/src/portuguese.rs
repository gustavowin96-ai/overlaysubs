//! Offline PT-BR worker. A bounded FIFO never blocks the ASR thread.
use crate::engine::Plamo;
use overlaysubs_core::pfml::TranslateRequest;
use overlaysubs_core::session::{Caption, Translator};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};

pub const MODEL: &str = "qwen3-1.7b-ptbr-q4_k_m";
pub struct Job {
    pub caption: Caption,
    pub speech_start: u64,
    pub audio_end: u64,
}

pub fn spawn(path: PathBuf, cancelled: Arc<AtomicBool>, mut emit: impl FnMut(Job) + Send + 'static,
    mut error: impl FnMut(String) + Send + 'static) -> (mpsc::SyncSender<Job>, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::sync_channel::<Job>(8);
    let handle = std::thread::spawn(move || {
        let mut model = None;
        for mut job in rx {
            if cancelled.load(Ordering::Relaxed) { break; }
            if job.caption.lang != "en" {
                error("O modelo português requer fala em inglês.".into());
                continue;
            }
            if model.is_none() {
                match Plamo::load_portuguese(&path, 2) {
                    Ok(m) => model = Some(m),
                    Err(e) => { error(format!("Falha ao carregar modelo português: {e}")); continue; }
                }
            }
            let req = TranslateRequest { text: &job.caption.source, source_lang: Some("en"), target_lang: "pt-BR", ..Default::default() };
            match model.as_mut().unwrap().translate(&req, &mut |_| {}) {
                Ok(text) if !text.trim().is_empty() => {
                    job.caption.translation = text;
                    if !cancelled.load(Ordering::Relaxed) { emit(job); }
                }
                Ok(_) => error("O tradutor retornou uma legenda vazia.".into()),
                Err(e) => error(format!("Falha na tradução: {e}")),
            }
        }
    });
    (tx, handle)
}
