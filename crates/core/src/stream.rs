//! Async streaming reduction via tokio.
//!
//! [`reduce_stream`] emits [`ProgressEvent`]s as it moves through the pipeline,
//! giving callers (CLI, web UI, FFI consumers) real-time visibility.

use std::time::Duration;

use tokio::sync::mpsc;

use opendocu_parser::{detect_format, parse_format};
use opendocu_structures::ProcessingOptions;

use crate::error::Result;
use crate::events::ProgressEvent;
use crate::reduce::reduce_doc;

/// Spawn a reduction on a background task and return a receiver of progress
/// events. The channel closes after `Done` or `Error`.
pub fn reduce_stream(
    input: Vec<u8>,
    opts: ProcessingOptions,
) -> mpsc::Receiver<ProgressEvent> {
    let (tx, rx) = mpsc::channel(16);

    tokio::spawn(async move {
        let total = input.len();
        if tx.send(ProgressEvent::Started { total_bytes: total }).await.is_err() {
            return;
        }

        // Detect format.
        let fmt = detect_format(&input);
        if tx.send(ProgressEvent::Detected { format: format!("{fmt:?}") }).await.is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;

        // Parse.
        let doc = match parse_format(fmt, &input) {
            Ok(d) => d,
            Err(e) => {
                let _ = tx.send(ProgressEvent::Error { message: e.to_string() }).await;
                return;
            }
        };
        let sections = doc.sections.len();
        let words = doc.word_count();
        if tx.send(ProgressEvent::Parsed { sections, words }).await.is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;

        // Reduce (reuse the synchronous orchestrator on this thread).
        let reduced = match reduce_doc(doc, opts) {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(ProgressEvent::Error { message: e.to_string() }).await;
                return;
            }
        };
        let sentences = reduced.metrics.original_sentences;
        let key_points = reduced.key_points.len();
        if tx.send(ProgressEvent::Summarized { sentences, key_points }).await.is_err() {
            return;
        }

        let reduced_json = match serde_json::to_string(&reduced) {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(ProgressEvent::Error { message: e.to_string() }).await;
                return;
            }
        };
        let _ = tx.send(ProgressEvent::Done { reduced_json }).await;
    });

    rx
}

/// Consume a stream into a [`crate::ReducedDocument`], returning the final result.
pub async fn collect_stream(
    mut rx: mpsc::Receiver<ProgressEvent>,
) -> Result<opendocu_structures::ReducedDocument> {
    while let Some(ev) = rx.recv().await {
        match ev {
            ProgressEvent::Done { reduced_json } => {
                let reduced: opendocu_structures::ReducedDocument =
                    serde_json::from_str(&reduced_json)?;
                return Ok(reduced);
            }
            ProgressEvent::Error { message } => {
                return Err(crate::error::CoreError::InvalidOptions(message));
            }
            _ => {}
        }
    }
    Err(crate::error::CoreError::InvalidOptions("stream closed without Done".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn stream_completes_for_simple_input() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let input = b"# Title\n\nFirst sentence here. Second sentence here. Third one too.".to_vec();
            let rx = reduce_stream(input, ProcessingOptions::default());
            let reduced = collect_stream(rx).await.unwrap();
            assert!(reduced.metrics.original_words > 0);
            assert!(reduced.metrics.reduced_words > 0);
        });
    }
}
