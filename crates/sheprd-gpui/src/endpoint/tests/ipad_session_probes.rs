use super::*;
use sheprd_backend::Generation;

#[gpui::test]
fn ipad_session_probes_only_ask_herdr1_hosts(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, _| {
        for (id, generation) in [
            ("ssh:selected", Generation::Herdr1),
            ("ssh:other", Generation::Herdr1),
            ("ssh:core", Generation::Herdr2),
        ] {
            let mut endpoint = Endpoint::new(
                id.into(),
                id.into(),
                ConnectTarget::Ssh {
                    target: "user@sessions.invalid".into(),
                    session: "default".into(),
                },
                true,
            );
            endpoint.set_backend_generation(generation);
            view.endpoints.push(endpoint);
        }
        view.selected_endpoint = 1;
        let targets = view.probe_targets();
        let ids: Vec<_> = targets.iter().map(|(id, _)| id.as_str()).collect();
        // Only the unfolded device is asked; on iPad a Herdr 2 host never
        // unfolds, so it is never a target.
        if cfg!(target_os = "ios") {
            assert_eq!(ids, ["ssh:selected", "ssh:other"]);
        } else {
            assert_eq!(ids, ["ssh:selected", "ssh:other", "ssh:core"]);
        }
    });
}
