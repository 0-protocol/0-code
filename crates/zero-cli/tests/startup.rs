use std::time::Instant;

#[test]
fn test_startup_under_50ms() {
    let start = Instant::now();
    let _config = zero_core::EngineConfig::default();
    let mut registry = zero_tools::ToolRegistry::new();
    zero_tools::register_core_tools(&mut registry);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 50,
        "Startup took {}ms, target <50ms",
        elapsed.as_millis()
    );
}
