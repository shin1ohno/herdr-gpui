use super::*;

fn callback() -> Callback {
    Callback {
        events: Rc::new(RefCell::new(VecDeque::new())),
        id: 7,
        operation: "test",
    }
}

#[test]
fn callback_errors_do_not_read_undefined_result_pointers() {
    let callback = callback();
    let context = std::ptr::from_ref(&callback).cast_mut().cast();
    browse_reply(
        std::ptr::null_mut(),
        0,
        0,
        -65570,
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        context,
    );
    resolve_reply(
        std::ptr::null_mut(),
        0,
        0,
        -65570,
        std::ptr::null(),
        std::ptr::null(),
        0,
        0,
        std::ptr::null(),
        context,
    );
    address_reply(
        std::ptr::null_mut(),
        0,
        0,
        -65570,
        std::ptr::null(),
        std::ptr::null(),
        0,
        context,
    );
    assert!(
        callback
            .events
            .borrow()
            .iter()
            .all(|event| matches!(event, Event::Failed { id: 7, error } if error.denied()))
    );
}

#[test]
fn callback_queue_cannot_grow_past_its_limit() {
    let callback = callback();
    for _ in 0..MAX_EVENTS * 2 {
        callback.push(Event::Address {
            id: 7,
            address: Ipv4Addr::LOCALHOST.into(),
            add: true,
        });
    }
    assert_eq!(callback.events.borrow().len(), MAX_EVENTS);
    callback.failed(-65570);
    assert_eq!(callback.events.borrow().len(), MAX_EVENTS);
    assert!(
        callback
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Failed { error, .. } if error.denied()))
    );
}

#[test]
fn resolve_decodes_network_order_port_and_copies_host() {
    let callback = callback();
    let context = std::ptr::from_ref(&callback).cast_mut().cast();
    resolve_reply(
        std::ptr::null_mut(),
        0,
        4,
        0,
        c"Office\\032Mac._ssh._tcp.local.".as_ptr(),
        c"studio.local.".as_ptr(),
        2222_u16.to_be(),
        0,
        std::ptr::null(),
        context,
    );
    assert!(
        matches!(callback.events.borrow().front(), Some(Event::Resolved { id: 7, host, port: 2222, interface: 4 }) if host.as_bytes() == b"studio.local.")
    );
}

#[test]
fn browse_copies_only_the_declared_service_and_keeps_interface() {
    let callback = callback();
    let context = std::ptr::from_ref(&callback).cast_mut().cast();
    for kind in [c"_ssh._tcp.", c"_http._tcp."] {
        browse_reply(
            std::ptr::null_mut(),
            ADD,
            4,
            0,
            c"Office Mac".as_ptr(),
            kind.as_ptr(),
            c"local.".as_ptr(),
            context,
        );
    }
    let events = callback.events.borrow();
    assert!(
        matches!(events.front(), Some(Event::Service { service, add: true }) if service.interface == 4 && service.name() == "Office Mac._ssh._tcp.local.")
    );
    assert!(events.get(1).is_none());
}

#[test]
fn darwin_address_layouts_decode_both_families_and_reject_short_or_unknown() {
    let v4 = AddressV4 {
        len: size_of::<AddressV4>() as u8,
        family: 2,
        port: 0,
        address: [192, 0, 2, 3],
        zero: [0; 8],
    };
    let v6 = AddressV6 {
        len: size_of::<AddressV6>() as u8,
        family: 30,
        port: 0,
        flow: 0,
        address: Ipv6Addr::LOCALHOST.octets(),
        scope: 0,
    };
    let unknown = SocketAddress {
        len: 16,
        family: 1,
        data: [0; 14],
    };
    let short = SocketAddress {
        len: 2,
        family: 2,
        data: [0; 14],
    };
    // SAFETY: each pointer has its advertised native layout and lifetime.
    unsafe {
        assert_eq!(
            ip_address(std::ptr::from_ref(&v4).cast()),
            Some(Ipv4Addr::new(192, 0, 2, 3).into())
        );
        assert_eq!(
            ip_address(std::ptr::from_ref(&v6).cast()),
            Some(Ipv6Addr::LOCALHOST.into())
        );
        assert_eq!(ip_address(std::ptr::from_ref(&unknown)), None);
        assert_eq!(ip_address(std::ptr::from_ref(&short)), None);
        assert_eq!(ip_address(std::ptr::null()), None);
    }
}
