#![allow(clippy::unwrap_used)]
use super::*;

fn host(id: char) -> SavedHost {
    SavedHost {
        id: id.to_string().repeat(32),
        label: format!("Host {id}"),
        target: format!("user@host-{id}"),
        session: "default".into(),
        enabled: true,
    }
}

#[test]
fn renaming_changes_only_the_label_and_rejects_invalid_updates_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("endpoints.json");
    let original = host('a');
    store_host(&path, &original).unwrap();
    store_selection(&path, Some(&original.id)).unwrap();
    let selection = fs::read(path.with_file_name("endpoint-selection.json")).unwrap();
    assert!(rename(&path, &original.id, "iPad host").unwrap());
    let mut expected = original.clone();
    expected.label = "iPad host".into();
    assert_eq!(
        load_with_selection(&path).unwrap(),
        (vec![expected], Some(original.id.clone()))
    );
    assert_eq!(
        fs::read(path.with_file_name("endpoint-selection.json")).unwrap(),
        selection
    );
    let before = fs::read(&path).unwrap();
    for label in ["", "\n", &"x".repeat(129)] {
        let error = rename(&path, &original.id, label).unwrap_err();
        assert!(
            matches!(error, Error::Storage { source, .. } if matches!(*source, Error::ProfileLabel))
        );
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn removing_a_device_preserves_other_devices_and_clears_stale_selection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("endpoints.json");
    let removed = host('a');
    let kept = host('b');
    store_host(&path, &removed).unwrap();
    store_host(&path, &kept).unwrap();
    store_selection(&path, Some(&removed.id)).unwrap();
    assert!(remove(&path, &removed.id).unwrap());
    assert_eq!(load_with_selection(&path).unwrap(), (vec![kept], None));
    let before = fs::read(&path).unwrap();
    assert!(!remove(&path, &removed.id).unwrap());
    assert!(!rename(&path, &removed.id, "absent").unwrap());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(matches!(remove(&path, "invalid"), Err(Error::ProfileId)));
}
