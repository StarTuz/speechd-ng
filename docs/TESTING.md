# Testing Guide

`speechd-ng` uses a comprehensive test harness to verify component interactions without requiring live audio hardware or external API connections.

## Running Tests

To run all tests, including integration tests:

```bash
cargo test
```

## Integration Tests

Integration tests are located in `tests/integration_test.rs`. They verify that the core components (`Ear`, `Cortex`, `Services`, `AudioEngine`) can be instantiated and wired together correctly.

### Components

#### MockAudioOutput

The `AudioEngine` is mocked using `mockall`. This allows you to verify that components call `speak` or other audio methods without actually playing sound.

```rust
let mut mock_engine = MockAudioOutput::new();
mock_engine
    .expect_speak()
    .with(mockall::predicate::eq("Expected text"), mockall::predicate::eq(None::<String>))
    .times(1)
    .returning(|_, _| ());
```

#### Cortex Mocks

`Cortex` provides two mock constructors:

- `Cortex::new_dummy()`: A silent dummy that consumes messages but returns nothing. Use this for structural wiring tests.
- `Cortex::new_testing()`: A reactive mock that intercepts specific messages (like `QueryStream`) and returns "Testing response.". Use this for functional flow tests where a component expects a reply from the LLM.

#### Ear Mocks

`Ear` provides `Ear::new_dummy()` which initializes the ear without loading the heavy Vosk model. This enables tests to run in CI environments where model files are absent.

## Adding New Tests

When adding new tests:

1. Use `MockAudioOutput` for any component requires audio output.
2. Use `Cortex::new_testing()` if your component interacts with the LLM pipeline.
3. Use `Ear::new_dummy()` if your component needs an Ear instance but doesn't test the actual transcription logic.

### Vision Engine (The Eye)

The `TheEye` component involves heavy model weights (~2GB). To verify its integration without a full model load:

1. **Verify Binary Dependencies**: Run `speechd-control describe`. If capture tools (`grim`, `spectacle`, etc.) are missing, it will return a specific "Binary not found" error.
2. **Lazy Loading**: The vision model is loaded only on the first call to `DescribeScreen`. Initial startup should remain fast regardless of model presence.
3. **Manual Verification**:

   ```bash
   # Test full pipeline (requires Moondream weights)
   speechd-control describe "What is on the screen?"
   ```

### Example: Proactive Events

See `test_proactive_event_trigger` in `tests/integration_test.rs` for an example of testing the `ProactiveManager` -> `Cortex` -> `AudioEngine` pipeline.

## Stress Tests (The Council's Mandate)

For V1.0 validation, the following high-load stress tests are available in `tests/stress_tests.rs`:

1. **Adversarial Image Shapes (Aris)**:
   - Command: `cargo test vision::tests::test_adversarial_image_shapes -- --nocapture`
   - Purpose: Verifies that the Vision engine gracefully handles 1x1, 0x0, and massively inverted aspect ratio images without panicking.

2. **Chronicler Flooding (Viper)**:
   - Command: `cargo test --test stress_tests test_chronicler_flooding -- --nocapture`
   - Purpose: Inserts 500+ memory items with embeddings to verify `sled` database integrity and BERT model throughput.

3. **Concurrency Collision (Sprint)**:
   - Command: `./scripts/concurrency_test.sh`
   - Purpose: Simulates a "perfect storm" of simultaneous TTS streaming, Vision inference, and ASR listening to check for deadlock or audio stutter.
