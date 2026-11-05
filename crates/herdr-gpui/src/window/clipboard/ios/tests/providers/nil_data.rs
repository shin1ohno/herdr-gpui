//! A native callback that returns no data without Foundation inventing an error.
use super::*;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, rc::Allocated};
use objc2_foundation::NSObjectProtocol;

define_class!(
    // SAFETY: main-thread NSItemProvider subclass; native method ABI matches
    // Foundation and borrowed block arguments remain live through each call.
    #[unsafe(super(NSItemProvider))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Arc<State>]
    struct NilPngProvider;
    impl NilPngProvider {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(Arc::new(State::default()));
            unsafe { msg_send![super(this), init] }
        }
        #[unsafe(method_id(loadDataRepresentationForTypeIdentifier:completionHandler:))]
        fn load_data(&self, kind: &NSString,
            reply: &DynBlock<dyn Fn(*mut NSData, *mut NSError)>
        ) -> Retained<NSProgress> {
            if kind.to_string() != "public.png" {
                return unsafe { msg_send![super(self),
                    loadDataRepresentationForTypeIdentifier: kind, completionHandler: reply] };
            }
            self.ivars().calls.fetch_add(1, Ordering::AcqRel);
            reply.call((ptr::null_mut(), ptr::null_mut()));
            NSProgress::progressWithTotalUnitCount(1)
        }
    }
    unsafe impl NSObjectProtocol for NilPngProvider {}
);

pub(super) fn nil_png_provider(main: MainThreadMarker) -> (Retained<NSItemProvider>, Arc<State>) {
    // SAFETY: allocated main-thread instance invokes its native init above.
    let provider: Retained<NilPngProvider> =
        unsafe { msg_send![NilPngProvider::alloc(main), init] };
    register(
        &provider,
        "public.png",
        Response::Data(b"metadata only".to_vec()),
    );
    let state = provider.ivars().clone();
    (provider.into_super(), state)
}
