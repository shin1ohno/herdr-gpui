use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

#[derive(Clone)]
pub(super) struct Control {
    pub before_read: Arc<AtomicBool>,
    pub after_read: Arc<AtomicBool>,
    pub write: Arc<AtomicUsize>,
    release: mpsc::Sender<()>,
    reached: Arc<std::sync::Mutex<mpsc::Receiver<()>>>,
}

impl Control {
    pub fn wait(&self) {
        self.reached
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
    pub fn release(&self) {
        self.release.send(()).unwrap();
    }
    pub fn park_read(&self) {
        self.before_read.store(true, Ordering::Release);
        self.wait();
    }
}

pub(super) struct Guarded {
    socket: Socket,
    control: Control,
    reached: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Guarded {
    pub fn new(socket: Socket) -> (Self, Control) {
        let (signal, reached) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let control = Control {
            before_read: Arc::new(AtomicBool::new(false)),
            after_read: Arc::new(AtomicBool::new(false)),
            write: Arc::new(AtomicUsize::new(0)),
            release,
            reached: Arc::new(std::sync::Mutex::new(reached)),
        };
        (
            Self {
                socket,
                control: control.clone(),
                reached: signal,
                release: resume,
            },
            control,
        )
    }
    fn park(&self) {
        self.reached.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

impl Read for Guarded {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.control.before_read.swap(false, Ordering::AcqRel) {
            self.park();
        }
        let result = self.socket.read(bytes);
        if matches!(result, Ok(n) if n > 0) && self.control.after_read.swap(false, Ordering::AcqRel)
        {
            self.park();
        }
        result
    }
}

impl Write for Guarded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.control.write.swap(0, Ordering::AcqRel) {
            1 => {
                let n = self.socket.write(&bytes[..1])?;
                self.park();
                Ok(n)
            }
            2 => {
                self.socket.write_all(bytes)?;
                self.park();
                Ok(bytes.len())
            }
            3 => Err(std::io::ErrorKind::Interrupted.into()),
            4 => Ok(0),
            5 => self.socket.write(&bytes[..1]),
            _ => self.socket.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}

pub(super) fn attached() -> (crate::Core, crate::Attachment, MockCore, Control) {
    let (client, server) = pair();
    let (guarded, control) = Guarded::new(client);
    let peer = std::thread::spawn(move || {
        MockCore::accept(server, &hello_reply("0.1.13", &every_method()))
    });
    let core = crate::Core::over(guarded).unwrap();
    let (mut peer, _) = peer.join().unwrap();
    let call = std::thread::spawn(move || {
        let attachment = core
            .attach(&crate::wire::Attach {
                terminal: "t7",
                ..Default::default()
            })
            .unwrap();
        (core, attachment)
    });
    let request = peer.request();
    peer.answer(&request, json!({"channel":5,"terminal":terminal("t7")}));
    let (core, attachment) = call.join().unwrap();
    (core, attachment, peer, control)
}

pub(super) fn until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(Instant::now() < deadline, "state did not settle");
        std::thread::yield_now();
    }
}

pub(super) fn eof(socket: &mut Socket) -> Vec<u8> {
    let mut bytes = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "no EOF");
        let mut chunk = [0; 4096];
        match socket.read(&mut chunk) {
            Ok(0) => return bytes,
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => panic!("{e}"),
        }
    }
}
