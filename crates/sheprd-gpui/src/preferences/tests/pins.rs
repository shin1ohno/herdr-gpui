use super::*;

#[core::prelude::v1::test]
fn agent_pins_roundtrip_through_the_existing_preferences_worker() {
    let directory = TestDirectory::new();
    let path = directory.0.join("pins.json");
    fs::write(&path, r#"{"agent_sort_manual":"grouped"}"#).unwrap();
    let mut preferences = Preferences::start(Ok(path.clone()));
    let mut chrome = await_loaded(&mut preferences);
    assert_eq!(chrome.agent_sort, Some(AgentSort::Grouped));
    assert_eq!(chrome.agent_pins, Default::default());
    chrome
        .agent_pins
        .toggle(crate::preferences::pins::AgentPin::new(
            "local", "boot", "p0",
        ));
    chrome
        .agent_pins
        .toggle(crate::preferences::pins::AgentPin::new(
            "remote", "boot", "p0",
        ));
    preferences.save(chrome.clone());
    let worker = preferences.worker.take().unwrap();
    drop(preferences);
    worker.join().unwrap();
    let mut restarted = Preferences::start(Ok(path));
    assert_eq!(await_loaded(&mut restarted), chrome);
}
