//! Minimal bindings to the iOS SDK's dns_sd.h. References and callback
//! contexts stay on one worker; only `process` can invoke callbacks. The
//! socket is borrowed, never closed independently of its DNSServiceRef.
#![allow(
    unsafe_code,
    reason = "Apple DNS-SD has a C callback API without a workspace safe binding" // i18n: data
)]

use super::{Error, MAX_SERVICES};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fd::BorrowedFd,
};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    ffi::{CStr, CString, c_char, c_void},
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr::NonNull,
    rc::Rc,
    time::Duration,
};

const ADD: u32 = 0x2;
const SHARE_CONNECTION: u32 = 0x4000;
const MAX_EVENTS: usize = MAX_SERVICES * 4;
const MAX_DNS_NAME: usize = 1005;
type Raw = *mut c_void;
type Queue = Rc<RefCell<VecDeque<Event>>>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Service {
    interface: u32,
    name: CString,
    kind: CString,
    domain: CString,
}

impl Service {
    pub(super) fn name(&self) -> String {
        format!("{}.{}", self.name.to_string_lossy(), super::super::SERVICE)
    }
}

pub(super) enum Event {
    Service {
        service: Service,
        add: bool,
    },
    Resolved {
        id: usize,
        host: CString,
        port: u16,
        interface: u32,
    },
    Address {
        id: usize,
        address: IpAddr,
        add: bool,
    },
    Failed {
        id: usize,
        error: Error,
    },
}

struct Callback {
    events: Queue,
    id: usize,
    operation: &'static str,
}

impl Callback {
    fn push(&self, event: Event) {
        let mut events = self.events.borrow_mut();
        // Refusal must stay visible even when an announcement flood filled
        // the bounded queue. Prefer its failure over the latest data event.
        if events.len() == MAX_EVENTS && matches!(event, Event::Failed { .. }) {
            events.pop_back();
        }
        if events.len() < MAX_EVENTS {
            events.push_back(event);
        }
    }

    fn failed(&self, status: i32) -> bool {
        if status == 0 {
            return false;
        }
        self.push(Event::Failed {
            id: self.id,
            error: Error::status(self.operation, status),
        });
        true
    }
}

struct Reference(NonNull<c_void>);

impl Drop for Reference {
    fn drop(&mut self) {
        // SAFETY: this reference is owned once, used only on this worker,
        // and never deallocated while DNSServiceProcessResult is running.
        unsafe { DNSServiceRefDeallocate(self.0.as_ptr()) };
    }
}

struct Operation {
    // Field order matters: cancel before releasing the callback context.
    _reference: Reference,
    _callback: Box<Callback>,
}

impl Operation {
    fn start(
        parent: &Reference,
        events: &Queue,
        id: usize,
        operation: &'static str,
        start: impl FnOnce(&mut Raw, *mut c_void) -> i32,
    ) -> Result<Self, Error> {
        let mut callback = Box::new(Callback {
            events: events.clone(),
            id,
            operation,
        });
        let context = (&mut *callback as *mut Callback).cast();
        let mut reference = parent.0.as_ptr();
        let status = start(&mut reference, context);
        check(operation, status)?;
        // On success a shared operation has its own child reference; on
        // failure the input parent must never be deallocated as a child.
        let reference = NonNull::new(reference).ok_or_else(|| Error::status(operation, -65541))?;
        Ok(Self {
            _reference: Reference(reference),
            _callback: callback,
        })
    }
}

pub(super) struct Browser {
    // Children and their contexts must be destroyed before the shared
    // connection. Dropping the parent first would invalidate child handles.
    operations: HashMap<usize, Operation>,
    connection: Reference,
    events: Queue,
    next_id: usize,
}

impl Browser {
    pub(super) fn new() -> Result<Self, Error> {
        let mut connection = std::ptr::null_mut();
        // SAFETY: valid out-pointer, no callback or caller context.
        // i18n: data
        check("opening Bonjour", unsafe {
            DNSServiceCreateConnection(&mut connection)
        })?;
        let connection = Reference(
            NonNull::new(connection).ok_or_else(|| Error::status("opening Bonjour", -65541))?, // i18n: data
        );
        let events = Rc::new(RefCell::new(VecDeque::new()));
        let browse = Operation::start(
            &connection,
            &events,
            0,
            "browsing SSH services", // i18n: data
            |reference, context| {
                // SAFETY: out-pointer begins as the live shared connection;
                // strings are static, callback has dns_sd.h's exact signature,
                // and its boxed context outlives the returned child reference.
                unsafe {
                    DNSServiceBrowse(
                        reference,
                        SHARE_CONNECTION,
                        0,
                        c"_ssh._tcp".as_ptr(),
                        c"local.".as_ptr(),
                        browse_reply,
                        context,
                    )
                }
            },
        )?;
        Ok(Self {
            operations: HashMap::from([(0, browse)]),
            connection,
            events,
            next_id: 1,
        })
    }

    pub(super) fn resolve(&mut self, service: &Service) -> Result<usize, Error> {
        let id = self.next_id;
        self.next_id += 1;
        let resolve = Operation::start(
            &self.connection,
            &self.events,
            id,
            "resolving an SSH service", // i18n: data
            |reference, context| {
                // SAFETY: strings are live CStrings for the call, and DNS-SD
                // copies their values. The callback context is owned by resolve.
                unsafe {
                    DNSServiceResolve(
                        reference,
                        SHARE_CONNECTION,
                        service.interface,
                        service.name.as_ptr(),
                        service.kind.as_ptr(),
                        service.domain.as_ptr(),
                        resolve_reply,
                        context,
                    )
                }
            },
        )?;
        self.operations.insert(id, resolve);
        Ok(id)
    }

    pub(super) fn addresses(
        &mut self,
        id: usize,
        interface: u32,
        host: &CStr,
    ) -> Result<(), Error> {
        self.cancel(id);
        let query = Operation::start(
            &self.connection,
            &self.events,
            id,
            "resolving SSH addresses", // i18n: data
            |reference, context| {
                // SAFETY: host is live and terminated; protocol bits 1|2 request
                // IPv4 and IPv6. The operation owns its callback context.
                unsafe {
                    DNSServiceGetAddrInfo(
                        reference,
                        SHARE_CONNECTION,
                        interface,
                        3,
                        host.as_ptr(),
                        address_reply,
                        context,
                    )
                }
            },
        )?;
        self.operations.insert(id, query);
        Ok(())
    }

    pub(super) fn cancel(&mut self, id: usize) {
        self.operations.remove(&id);
    }

    pub(super) fn next(&self) -> Option<Event> {
        self.events.borrow_mut().pop_front()
    }

    pub(super) fn process(&mut self, timeout: Duration) -> Result<(), Error> {
        // SAFETY: the reference is live, used on this worker only. DNS-SD
        // owns the fd until the connection is deallocated, after this borrow.
        let fd = unsafe { DNSServiceRefSockFD(self.connection.0.as_ptr()) };
        if fd < 0 {
            return Err(Error::status("reading Bonjour", -65541)); // i18n: data
        }
        // SAFETY: the fd was checked and its owner outlives this function.
        let fd = unsafe { BorrowedFd::borrow_raw(fd) };
        let mut fds = [PollFd::new(&fd, PollFlags::IN)];
        let timeout = Timespec::try_from(timeout)
            .map_err(|_| Error::Poll(io::Error::from(io::ErrorKind::InvalidInput)))?;
        match poll(&mut fds, Some(&timeout)) {
            Err(rustix::io::Errno::INTR) => return Ok(()),
            Err(error) => return Err(Error::Poll(error.into())),
            Ok(_) => {}
        }
        let flags = fds[0].revents();
        if flags.contains(PollFlags::IN) {
            // SAFETY: poll reported readable data, all child callback boxes
            // are live, no queue borrow is held, and no other thread can
            // process or deallocate this connection during callbacks.
            // i18n: data
            check("reading Bonjour", unsafe {
                DNSServiceProcessResult(self.connection.0.as_ptr())
            })?;
        } else if flags.intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL) {
            return Err(Error::Poll(io::Error::from(
                io::ErrorKind::ConnectionAborted,
            )));
        }
        Ok(())
    }
}

fn check(operation: &'static str, status: i32) -> Result<(), Error> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error::status(operation, status))
    }
}

/// DNS-SD guarantees terminated DNS names; bound copies of their maximum
/// escaped presentation form and reject malformed UTF-8 host names later.
unsafe fn dns_name(name: *const c_char) -> Option<CString> {
    if name.is_null() {
        return None;
    }
    // SAFETY: the callback's successful result guarantees a terminated name
    // valid until callback return. No reference to it escapes this copy.
    let name = unsafe { CStr::from_ptr(name) };
    (name.to_bytes().len() <= MAX_DNS_NAME).then(|| name.to_owned())
}

extern "C" fn browse_reply(
    _: Raw,
    flags: u32,
    interface: u32,
    status: i32,
    name: *const c_char,
    kind: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    // SAFETY: Operation holds this boxed context throughout processing.
    let callback = unsafe { &*context.cast::<Callback>() };
    if callback.failed(status) {
        return;
    }
    // SAFETY: after a success these are the three valid dns_sd.h names.
    let service = unsafe { dns_name(name).zip(dns_name(kind)).zip(dns_name(domain)) };
    if let Some(((name, kind), domain)) = service
        && kind.as_bytes() == b"_ssh._tcp."
        && domain.as_bytes() == b"local."
    {
        callback.push(Event::Service {
            service: Service {
                interface,
                name,
                kind,
                domain,
            },
            add: flags & ADD != 0,
        });
    }
}

extern "C" fn resolve_reply(
    _: Raw,
    _: u32,
    interface: u32,
    status: i32,
    _: *const c_char,
    host: *const c_char,
    port: u16,
    _: u16,
    _: *const u8,
    context: *mut c_void,
) {
    // SAFETY: as in browse_reply; error results leave other arguments undefined.
    let callback = unsafe { &*context.cast::<Callback>() };
    if callback.failed(status) {
        return;
    }
    // SAFETY: host is a valid terminated name for a successful resolve.
    if let Some(host) = unsafe { dns_name(host) } {
        callback.push(Event::Resolved {
            id: callback.id,
            host,
            port: u16::from_be(port),
            interface,
        });
    }
}

// Darwin sockaddr layout from the SDK's sys/socket.h and netinet/in.h.
#[repr(C)]
struct SocketAddress {
    len: u8,
    family: u8,
    data: [u8; 14],
}
#[repr(C)]
struct AddressV4 {
    len: u8,
    family: u8,
    port: u16,
    address: [u8; 4],
    zero: [u8; 8],
}
#[repr(C)]
struct AddressV6 {
    len: u8,
    family: u8,
    port: u16,
    flow: u32,
    address: [u8; 16],
    scope: u32,
}

unsafe fn ip_address(address: *const SocketAddress) -> Option<IpAddr> {
    // SAFETY: caller guarantees null or a valid sockaddr from DNS-SD.
    let address = unsafe { address.as_ref() }?;
    match (address.family, usize::from(address.len)) {
        (2, len) if len >= size_of::<AddressV4>() => {
            // SAFETY: DNS-SD returns an aligned sockaddr_in for AF_INET.
            let address = unsafe { &*(std::ptr::from_ref(address).cast::<AddressV4>()) };
            Some(Ipv4Addr::from(address.address).into())
        }
        (30, len) if len >= size_of::<AddressV6>() => {
            // SAFETY: DNS-SD returns an aligned sockaddr_in6 for AF_INET6.
            let address = unsafe { &*(std::ptr::from_ref(address).cast::<AddressV6>()) };
            Some(Ipv6Addr::from(address.address).into())
        }
        _ => None,
    }
}

extern "C" fn address_reply(
    _: Raw,
    flags: u32,
    _: u32,
    status: i32,
    _: *const c_char,
    address: *const SocketAddress,
    _: u32,
    context: *mut c_void,
) {
    // SAFETY: Operation holds this boxed context throughout processing.
    let callback = unsafe { &*context.cast::<Callback>() };
    if callback.failed(status) {
        return;
    }
    // SAFETY: after success the address is null or a valid SDK sockaddr.
    if let Some(address) = unsafe { ip_address(address) } {
        callback.push(Event::Address {
            id: callback.id,
            address,
            add: flags & ADD != 0,
        });
    }
}

type BrowseReply =
    extern "C" fn(Raw, u32, u32, i32, *const c_char, *const c_char, *const c_char, *mut c_void);
type ResolveReply = extern "C" fn(
    Raw,
    u32,
    u32,
    i32,
    *const c_char,
    *const c_char,
    u16,
    u16,
    *const u8,
    *mut c_void,
);
type AddressReply =
    extern "C" fn(Raw, u32, u32, i32, *const c_char, *const SocketAddress, u32, *mut c_void);

// Apple exports DNS-SD from libSystem; it has no standalone libdns_sd.
#[link(name = "System")]
unsafe extern "C" {
    fn DNSServiceCreateConnection(reference: *mut Raw) -> i32;
    fn DNSServiceBrowse(
        reference: *mut Raw,
        flags: u32,
        interface: u32,
        kind: *const c_char,
        domain: *const c_char,
        callback: BrowseReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceResolve(
        reference: *mut Raw,
        flags: u32,
        interface: u32,
        name: *const c_char,
        kind: *const c_char,
        domain: *const c_char,
        callback: ResolveReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceGetAddrInfo(
        reference: *mut Raw,
        flags: u32,
        interface: u32,
        protocol: u32,
        host: *const c_char,
        callback: AddressReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceRefSockFD(reference: Raw) -> i32;
    fn DNSServiceProcessResult(reference: Raw) -> i32;
    fn DNSServiceRefDeallocate(reference: Raw);
}

#[cfg(test)]
mod tests;
