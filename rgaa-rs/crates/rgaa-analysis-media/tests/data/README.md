# Test fixture

`jfk.wav` — the public 16 kHz mono speech sample shipped with
[whisper.cpp](https://github.com/ggml-org/whisper.cpp) (`samples/jfk.wav`,
352078 bytes, SHA-256
`59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`).

Used only by the model-gated tests in `asr.rs`, `vad.rs` and `lib.rs`: those
tests verify that a real speech signal produces speech segments and a
transcription when the local models are present. They are skipped without
`RUN_MODEL_TESTS=1`, and fail loudly when `RGAA_MODEL_TESTS_REQUIRED=1` is set
in CI (see `src/testing.rs`). No model is downloaded by the plugin or by tests.
