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
    mut error: impl FnMut(String) + Send + 'static,
    mut status: impl FnMut(&str) + Send + 'static) -> (mpsc::SyncSender<Job>, std::thread::JoinHandle<()>) {
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
                status("Carregando modelo português…");
                match Plamo::load_portuguese(&path, 2) {
                    Ok(mut m) => { m.cancel = Some(cancelled.clone()); model = Some(m); status("Modelo português pronto."); },
                    Err(e) => { error(format!("Falha ao carregar modelo português: {e}")); break; }
                }
            }
            let req = TranslateRequest { text: &job.caption.source, source_lang: Some("en"), target_lang: "pt-BR", ..Default::default() };
            match model.as_mut().unwrap().translate(&req, &mut |_| {}) {
                Ok(text) if !text.trim().is_empty() => {
                    job.caption.translation = text;
                    if !cancelled.load(Ordering::Relaxed) { emit(job); }
                }
                Ok(_) => error("O tradutor retornou uma legenda vazia.".into()),
                Err(e) => if !cancelled.load(Ordering::Relaxed) { error(format!("Falha na tradução: {e}")); },
            }
        }
    });
    (tx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use overlaysubs_core::session::Stage;
    fn job(lang: &str) -> Job {
        Job { caption: Caption { id: 7, stage: Stage::Final, lang: lang.into(), source: "Hello".into(), translation: String::new() }, speech_start: 0, audio_end: 0 }
    }

    #[test]
    fn unsupported_language_reports_an_error_without_loading_or_emitting() {
        let (errors, rx) = mpsc::channel();
        let (tx, handle) = spawn(PathBuf::from("missing-model.gguf"), Arc::new(AtomicBool::new(false)),
            |_| panic!("must not emit a translation"), move |e| { errors.send(e).unwrap(); }, |_| panic!("must not load a model"));
        tx.send(job("ja")).unwrap();
        drop(tx);
        handle.join().unwrap();
        assert!(rx.recv().unwrap().contains("inglês"));
    }

    #[test]
    fn cancelled_session_discards_jobs_before_model_load() {
        let (tx, handle) = spawn(PathBuf::from("missing-model.gguf"), Arc::new(AtomicBool::new(true)),
            |_| panic!("must not emit"), |_| panic!("must not report stale errors"), |_| panic!("must not load"));
        let _ = tx.send(job("en"));
        drop(tx);
        handle.join().unwrap();
    }
}
