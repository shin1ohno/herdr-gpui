use super::*;
use crate::controls::COMMANDS;

#[test]
fn catalog_commands_need_an_explicit_ipad_classification() {
    let allowed: std::collections::HashSet<_> = ALLOWED.iter().copied().collect();
    assert_eq!(allowed.len(), ALLOWED.len(), "duplicate allow-list entry");
    for command in ALLOWED {
        assert!(COMMANDS.iter().any(|info| info.command == *command));
    }
    for info in COMMANDS {
        assert_eq!(
            available(info.command, true),
            allowed.contains(&info.command)
        );
        assert!(available(info.command, false));
    }
    // A new catalog entry has no implicit permission, even in a known family.
    let unclassified = sheprd_commands::CommandInfo {
        command: Command::TabNumber(250),
        name: "unclassified",
        message: "unclassified",
    };
    let offered: Vec<_> = COMMANDS
        .iter()
        .chain([&unclassified])
        .filter(|info| available(info.command, true))
        .map(|info| info.command)
        .collect();
    assert!(!offered.contains(&unclassified.command));
    assert!(offered.contains(&Command::Tab));
}
