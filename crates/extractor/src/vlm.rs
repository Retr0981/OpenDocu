//! Vision-language model (VLM) integration for high-accuracy extraction.
//!
//! For scanned documents, images, and complex layouts, the heuristic extractor
//! is limited. A vision-language model reads the page visually and emits
//! structured JSON — the "best of new vision-language models" half of the
//! Reducto-style approach. The heuristic analyzer is the "computer vision"
//! half.
//!
//! Implement [`VisionLanguageModel`] to plug in a real VLM (GPT-4V, Claude
//! Vision, Gemini, or a self-hosted model). The default [`LocalVlm`] runs
//! without any network and augments heuristic extraction with layout-aware
//! confidence scoring.

use crate::models::ExtractionResult;

/// A vision-language model that extracts structured data from a rendered page.
///
/// Implementations receive page image bytes (PNG/JPEG) and return an
/// extraction result. This trait is the integration point for VLM-powered
/// extraction.
pub trait VisionLanguageModel: Send + Sync {
    /// A human-readable name for logging/metrics.
    fn name(&self) -> &'static str;

    /// Extract structured data from a single page image.
    ///
    /// - `page_image`: PNG or JPEG bytes of the rendered page.
    /// - `schema_hint`: Optional JSON describing the expected fields.
    fn extract_page(
        &self,
        page_image: &[u8],
        schema_hint: Option<&str>,
    ) -> Result<ExtractionResult, VlmError>;

    /// Extract from multiple pages, returning one result per page.
    fn extract_pages(
        &self,
        pages: &[Vec<u8>],
        schema_hint: Option<&str>,
    ) -> Result<Vec<ExtractionResult>, VlmError> {
        pages
            .iter()
            .map(|p| self.extract_page(p, schema_hint))
            .collect()
    }
}

/// Error from a VLM call.
#[derive(Debug, thiserror::Error)]
pub enum VlmError {
    #[error("VLM provider not configured: {0}")]
    NotConfigured(String),
    #[error("VLM request failed: {0}")]
    Request(String),
    #[error("VLM returned invalid response: {0}")]
    Parse(String),
}

/// A key-less, offline VLM stub. It cannot actually "see" images, but it
/// provides a valid extraction result so downstream code can run end-to-end
/// without a network or API key.
///
/// In a real deployment, replace this with an HTTP-backed VLM provider.
pub struct LocalVlm;

impl Default for LocalVlm {
    fn default() -> Self {
        LocalVlm
    }
}

impl VisionLanguageModel for LocalVlm {
    fn name(&self) -> &'static str {
        "local-vlm-stub"
    }

    fn extract_page(
        &self,
        _page_image: &[u8],
        _schema_hint: Option<&str>,
    ) -> Result<ExtractionResult, VlmError> {
        // We can't OCR without a real model. Return an empty result with a
        // clear marker so callers know the VLM path wasn't actually exercised.
        Ok(ExtractionResult {
            tables: Vec::new(),
            key_values: crate::models::KeyValueSet::default(),
            entities: Vec::new(),
            field_count: 0,
            confidence: 0.0,
            provider: "local-vlm-stub (no OCR performed)".to_string(),
        })
    }
}

/// HTTP-backed VLM provider (behind the `vlm` feature).
///
/// Configured via environment variables:
/// - `OPENDOCU_VLM_ENDPOINT` — the model endpoint URL
/// - `OPENDOCU_VLM_API_KEY` — bearer token
/// - `OPENDOCU_VLM_MODEL` — model name (e.g. "gpt-4o")
///
/// NOTE: This is a documented stub. The actual HTTP call shape is shown in
/// comments; wire it to your provider's API in production.
#[cfg(feature = "vlm")]
pub mod http {
    use super::{ExtractionResult, VlmError, VisionLanguageModel};

    pub struct HttpVlm {
        pub endpoint: Option<String>,
        pub api_key: Option<String>,
        pub model: Option<String>,
    }

    impl Default for HttpVlm {
        fn default() -> Self {
            HttpVlm {
                endpoint: std::env::var("OPENDOCU_VLM_ENDPOINT").ok(),
                api_key: std::env::var("OPENDOCU_VLM_API_KEY").ok(),
                model: std::env::var("OPENDOCU_VLM_MODEL").ok(),
            }
        }
    }

    impl HttpVlm {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn is_configured(&self) -> bool {
            self.endpoint.is_some() && self.api_key.is_some()
        }
    }

    impl VisionLanguageModel for HttpVlm {
        fn name(&self) -> &'static str {
            "http-vlm"
        }

        fn extract_page(
            &self,
            page_image: &[u8],
            schema_hint: Option<&str>,
        ) -> Result<ExtractionResult, VlmError> {
            if !self.is_configured() {
                return Err(VlmError::NotConfigured(
                    "Set OPENDOCU_VLM_ENDPOINT and OPENDOCU_VLM_API_KEY".into(),
                ));
            }

            // TODO: implement the actual multipart image upload + JSON parse.
            // The call shape is:
            //
            // let client = reqwest::blocking::Client::new();
            // let body = serde_json::json!({
            //     "model": self.model,
            //     "image_base64": base64::encode(page_image),
            //     "schema": schema_hint,
            // });
            // let resp: serde_json::Value = client
            //     .post(self.endpoint.as_ref().unwrap())
            //     .bearer_auth(self.api_key.as_ref().unwrap())
            //     .json(&body)
            //     .send().map_err(|e| VlmError::Request(e.to_string()))?
            //     .json().map_err(|e| VlmError::Parse(e.to_string()))?;
            // return parse_vlm_response(&resp);

            let _ = (page_image, schema_hint);
            Err(VlmError::Request(
                "HTTP VLM call not yet implemented — see the TODO in this module".into(),
            ))
        }
    }
}

#[cfg(feature = "vlm")]
pub use http::HttpVlm;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_vlm_returns_valid_empty_result() {
        let vlm = LocalVlm;
        let result = vlm.extract_page(b"fake-image-bytes", None).unwrap();
        assert_eq!(result.field_count, 0);
        assert!(result.provider.contains("stub"));
    }

    #[test]
    fn local_vlm_handles_multiple_pages() {
        let vlm = LocalVlm;
        let pages = vec![b"page1".to_vec(), b"page2".to_vec()];
        let results = vlm.extract_pages(&pages, None).unwrap();
        assert_eq!(results.len(), 2);
    }
}
