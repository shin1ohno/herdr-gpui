use super::*;

#[test]
fn layout_is_opt_in_and_restores_nested_guards() {
    assert!(active());
    assert!(!layout());
    {
        let _ipad = ipad_layout();
        assert!(layout());
        {
            let _nested = ipad_layout();
            assert!(layout());
        }
        assert!(layout());
        {
            let _desktop = desktop();
            assert!(!active());
            assert!(!layout());
            let _nested = ipad_layout();
            assert!(!layout());
        }
        assert!(active());
        assert!(layout());
    }
    assert!(!layout());
    assert!(active());
}

#[test]
fn layout_opt_in_is_local_to_the_test_thread() {
    let _ipad = ipad_layout();
    assert!(layout());
    assert!(
        std::thread::spawn(|| !layout())
            .join()
            .is_ok_and(|value| value)
    );
}
