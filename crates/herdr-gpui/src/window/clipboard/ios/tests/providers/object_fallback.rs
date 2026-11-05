//! A native provider whose object callback has no value but data is usable.
use super::*;
use objc2::{
    ClassType, DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Allocated,
    runtime::{AnyClass, ProtocolObject},
};
use objc2_foundation::{NSItemProviderReading, NSObjectProtocol};

define_class!(
    // SAFETY: a native NSItemProvider subclass with main-thread metadata and
    // synchronized counters. The override's class/block/progress ABI matches
    // Foundation's documented generic object-loading selector.
    #[unsafe(super(NSItemProvider))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Arc<AtomicUsize>]
    struct NilTextProvider;
    impl NilTextProvider {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(Arc::new(AtomicUsize::new(0)));
            unsafe { msg_send![super(this), init] }
        }
        #[unsafe(method(canLoadObjectOfClass:))]
        fn can_load(&self, class: &AnyClass) -> bool {
            ptr::eq(class, NSString::class())
        }
        #[unsafe(method_id(loadObjectOfClass:completionHandler:))]
        fn load_object(&self, _: &AnyClass,
            reply: &DynBlock<dyn Fn(*mut ProtocolObject<dyn NSItemProviderReading>, *mut NSError)>
        ) -> Retained<NSProgress> {
            self.ivars().fetch_add(1, Ordering::AcqRel);
            reply.call((ptr::null_mut(), ptr::null_mut()));
            NSProgress::progressWithTotalUnitCount(1)
        }
    }
    unsafe impl NSObjectProtocol for NilTextProvider {}
);

pub(crate) fn nil_text_object_uses_same_provider_data() {
    let main = MainThreadMarker::new().unwrap();
    // SAFETY: allocated main-thread instance invokes its native init above.
    let provider: Retained<NilTextProvider> =
        unsafe { msg_send![NilTextProvider::alloc(main), init] };
    let text = register(
        &provider,
        "public.utf8-plain-text",
        Response::Data(b"same provider text".to_vec()),
    );
    let image = register(
        &provider,
        "public.png",
        Response::Data(b"image must not be loaded".to_vec()),
    );
    let read = capture(&native_providers(&[&provider]), false, main).unwrap();
    assert_eq!(provider.ivars().load(Ordering::Acquire), 0);
    assert_eq!(text.calls.load(Ordering::Acquire), 0);
    assert_eq!(image.calls.load(Ordering::Acquire), 0);
    let item = run_read(read, || false, || {}).unwrap().unwrap();
    assert_eq!(item.text(), Some("same provider text".into()));
    assert_eq!(provider.ivars().load(Ordering::Acquire), 1);
    assert_eq!(text.calls.load(Ordering::Acquire), 1);
    assert_eq!(image.calls.load(Ordering::Acquire), 0);
}
