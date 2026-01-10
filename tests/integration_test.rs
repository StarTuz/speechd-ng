use speechd_ng::backends::Voice;
use speechd_ng::cortex::Cortex;
use speechd_ng::ear::Ear;
use speechd_ng::engine::AudioOutput;
use std::sync::Arc;

mockall::mock! {
    pub AudioOutput {}
    #[async_trait::async_trait]
    impl AudioOutput for AudioOutput {
        fn speak(&self, text: &str, voice: Option<String>);
        async fn speak_blocking(&self, text: &str, voice: Option<String>);
        async fn list_voices(&self) -> Vec<Voice>;
        async fn list_downloadable_voices(&self) -> Vec<Voice>;
        async fn download_voice(&self, voice_id: String) -> std::io::Result<()>;
        async fn play_audio(&self, url: &str) -> Result<(), String>;
        async fn stop_audio(&self) -> bool;
        async fn set_volume(&self, volume: f32) -> bool;
        async fn get_playback_status(&self) -> (bool, String);
        fn speak_channel(&self, text: &str, voice: Option<String>, channel: &str);
        async fn play_audio_channel(&self, url: &str, channel: &str) -> Result<(), String>;
    }
}

#[tokio::test]
async fn test_components_setup_with_mock() {
    let mut mock_engine = MockAudioOutput::new();

    // Just setting up an expectation to prove we can access the mock methods
    mock_engine
        .expect_speak()
        .with(
            mockall::predicate::eq("Hello"),
            mockall::predicate::eq(None::<String>),
        )
        .times(0) // We won't actually call it here to avoid runtime issues
        .returning(|_, _| ());

    let engine: Arc<dyn speechd_ng::engine::AudioOutput + Send + Sync> = Arc::new(mock_engine);
    let ear = Ear::new_dummy();
    let cortex = Cortex::new_dummy();

    // Verify we can call the method with the trait object
    // Verify wiring is correct (no type mismatches etc) - runtime behavior stubbed
    let _ = (ear, engine, cortex);
}

#[tokio::test]
async fn test_proactive_event_trigger() {
    let mut mock_engine = MockAudioOutput::new();

    // We expect the proactive manager to eventually call speak
    // The "Testing response." comes from Cortex::new_testing()
    mock_engine
        .expect_speak()
        .with(
            mockall::predicate::eq("Testing response."),
            mockall::predicate::eq(None::<String>),
        )
        .times(1)
        .returning(|_, _| ());

    let engine: Arc<dyn AudioOutput + Send + Sync> = Arc::new(mock_engine);
    let cortex = Cortex::new_testing();

    let manager = speechd_ng::proactive::ProactiveManager::new(cortex, engine);
    manager.reset_rate_limit();

    // Trigger an event
    manager
        .trigger_event(speechd_ng::proactive::ProactiveEvent::SystemIdle)
        .await;

    // Mock expectation is verified on drop
}
