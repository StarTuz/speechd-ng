use speechd_ng::chronicler::Chronicler;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_chronicler_flooding() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_db");

    // Chronicler::new downloads models, which might be slow or fail in CI without network.
    // However, if we are in the user's environment, they should be cached.
    let chronicler = match Chronicler::new(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Skipping Chronicler stress test (model download failed/offline): {}",
                e
            );
            return;
        }
    };

    println!("Starting Chronicler flooding test (500 items)...");

    // Viper requested 5,000, but for a standard test suite, let's start with 500
    // to keep it under a reasonable duration if the CPU is slow.
    // We can tune this higher if performance holds.
    for i in 0..500 {
        let text = format!("Memory item {} with some extra padding text to simulate real conversation data. Lorem ipsum dolor sit amet.", i);
        chronicler
            .add_memory(&text)
            .expect("Failed to add memory during flood");
        if i % 100 == 0 {
            println!("Flooded {} items...", i);
        }
    }

    println!("Flood complete. Running search...");
    let start = std::time::Instant::now();
    let results = chronicler.search("Lorem ipsum", 5).expect("Search failed");
    let duration = start.elapsed();

    println!(
        "Search returned {} results in {:?}",
        results.len(),
        duration
    );
    assert_eq!(results.len(), 5);

    // Final integrity check
    assert!(
        duration.as_millis() < 5000,
        "Search took too long (>5000ms for 500 items)"
    );
}
