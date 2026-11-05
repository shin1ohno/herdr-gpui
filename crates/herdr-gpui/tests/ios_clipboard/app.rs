//! NSItemProvider pasteboard promises need an attributed foreground UIKit app.
//! Matches the platform's existing uikit_smoke scene lifecycle.
#![allow(
    unsafe_code,
    reason = "test-only objc2 UIKit delegates and run-loop callback"
)]

use block2::RcBlock;
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::{Allocated, Retained},
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSRunLoop, NSString};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIScene, UISceneConfiguration, UISceneConnectionOptions,
    UISceneDelegate, UISceneSession, UIViewController, UIWindow, UIWindowScene,
    UIWindowSceneDelegate,
};
use std::{
    cell::OnceCell,
    sync::atomic::{AtomicBool, Ordering},
};

static STARTED: AtomicBool = AtomicBool::new(false);

define_class!(
    // SAFETY: NSObject subclass without extra initialization requirements.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HerdrClipboardTestAppDelegate"]
    struct AppDelegate;
    unsafe impl NSObjectProtocol for AppDelegate {}
    unsafe impl UIApplicationDelegate for AppDelegate {
        #[unsafe(method_id(application:configurationForConnectingSceneSession:options:))]
        fn configure(
            &self,
            _: &UIApplication,
            session: &UISceneSession,
            _: &UISceneConnectionOptions,
        ) -> Retained<UISceneConfiguration> {
            let config = UISceneConfiguration::configurationWithName_sessionRole(
                Some(&NSString::from_str("Clipboard tests")),
                &session.role(),
                self.mtm(),
            );
            // SAFETY: this NSObject subclass adopts UIWindowSceneDelegate.
            unsafe { config.setDelegateClass(Some(SceneDelegate::class())) };
            config
        }
    }
);

#[derive(Default)]
struct SceneState {
    window: OnceCell<Retained<UIWindow>>,
}

define_class!(
    // SAFETY: ivars are initialized before NSObject init; native signatures match.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "HerdrClipboardTestSceneDelegate"]
    #[ivars = SceneState]
    struct SceneDelegate;
    impl SceneDelegate {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(SceneState::default());
            unsafe { msg_send![super(this), init] }
        }
    }
    unsafe impl NSObjectProtocol for SceneDelegate {}
    unsafe impl UISceneDelegate for SceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn connect(&self, scene: &UIScene, _: &UISceneSession, _: &UISceneConnectionOptions) {
            let Some(scene) = scene.downcast_ref::<UIWindowScene>() else { return };
            let controller = UIViewController::new(self.mtm());
            let window = UIWindow::initWithWindowScene(UIWindow::alloc(self.mtm()), scene);
            window.setRootViewController(Some(&controller));
            window.makeKeyAndVisible();
            let _ = self.ivars().window.set(window);
        }
        #[unsafe(method(sceneDidBecomeActive:))]
        fn active(&self, _: &UIScene) {
            if STARTED.swap(true, Ordering::AcqRel) { return; }
            // A run-loop block, not a main dispatch-queue block: the nested
            // test pump must still be able to service queued native loads.
            let block = RcBlock::new(|| {
                let Some(main) = MainThreadMarker::new() else {
                    eprintln!("clipboard test run loop requires the main thread");
                    std::process::exit(2);
                };
                eprintln!("clipboard host application state: {:?}",
                    UIApplication::sharedApplication(main).applicationState());
                let passed = std::panic::catch_unwind(super::run_tests).is_ok();
                let result = if passed { "passed\n" } else { "failed\n" };
                let wrote = std::env::var_os("HERDR_TEST_CLIPBOARD_RESULT")
                    .is_some_and(|path| std::fs::write(path, result).is_ok());
                std::process::exit(if passed && wrote { 0 } else { 1 });
            });
            // SAFETY: captures no state and only executes on this run loop.
            unsafe { NSRunLoop::mainRunLoop().performBlock(&block) };
        }
    }
    unsafe impl UIWindowSceneDelegate for SceneDelegate {
        #[unsafe(method_id(window))]
        fn window(&self) -> Option<Retained<UIWindow>> { self.ivars().window.get().cloned() }
    }
);

pub(super) fn run() {
    let Some(main) = MainThreadMarker::new() else {
        std::process::exit(2);
    };
    let delegate = NSString::from_class(AppDelegate::class());
    UIApplication::main(None, Some(&delegate), main);
}
