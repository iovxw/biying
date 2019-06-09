use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use ksni::{self, menu, tray};
use qmetaobject::*;

lazy_static! {
    static ref TRAY_RX: Mutex<Option<mpsc::Receiver<Cmd>>> = Mutex::default();
    static ref TRAY_TX: Mutex<Option<mpsc::Sender<Cmd>>> = Mutex::default();
    static ref ON_OPEN: Mutex<Option<Box<dyn Fn() -> bool + Send>>> = Mutex::default();
    static ref ON_QUIT: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::default();
}

#[derive(Debug)]
pub enum Cmd {
    Open,
    Quit,
}

pub fn wait() -> Cmd {
    let rx = TRAY_RX.lock().unwrap();
    let rx = rx.as_ref().expect("");
    rx.recv().unwrap()
}

#[derive(QObject, Default)]
pub struct TrayProxy {
    base: qt_base_class!(trait QObject),
    pub connect_to_backend: qt_method!(fn (&mut self)),
    pub open: qt_signal!(),
    pub quit: qt_signal!(),
}

impl TrayProxy {
    fn connect_to_backend(&mut self) {
        let qptr = QPointer::from(&*self);
        let on_open_callback = queued_callback(move |ret: mpsc::Sender<bool>| {
            if let Some(ptr) = qptr.as_ref() {
                ptr.open();
            }
            ret.send(qptr.as_ref().is_some()).unwrap();
        });
        *ON_OPEN.lock().unwrap() = Some(Box::new(move || {
            let (tx, rx) = mpsc::channel();
            (on_open_callback)(tx);
            rx.recv().unwrap()
        }));
        let qptr = QPointer::from(&*self);
        let on_quit_callback = queued_callback(move |_| {
            if let Some(ptr) = qptr.as_ref() {
                ptr.quit();
            }
        });
        *ON_QUIT.lock().unwrap() = Some(Box::new(move || (on_quit_callback)(())));
    }
}

impl Drop for TrayProxy {
    fn drop(&mut self) {
        *ON_OPEN.lock().unwrap() = None;
        *ON_QUIT.lock().unwrap() = None;
    }
}

pub struct Tray;

impl ksni::Tray for Tray {
    type Err = std::convert::Infallible;
    fn tray_properties() -> tray::Properties {
        tray::Properties {
            icon_name: "livewallpaper-indicator".to_owned(),
            ..Default::default()
        }
    }
    fn menu() -> Vec<menu::MenuItem> {
        use menu::*;
        vec![
            StandardItem {
                label: "Open".into(),
                activate: Box::new(open),
                ..Default::default()
            }
            .into(),
            MenuItem::Sepatator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(quit),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn run_tray_in_background() {
    let mut rx = TRAY_RX.lock().unwrap();
    if rx.is_none() {
        let (sender, recver) = mpsc::channel();
        *rx = Some(recver);
        *TRAY_TX.lock().unwrap() = Some(sender);
        thread::spawn(move || {
            ksni::run(Tray);
        });
    }
}

fn open() {
    let arrived = ON_OPEN
        .lock()
        .unwrap()
        .as_ref()
        .map(|callback| (callback)())
        .unwrap_or(false);
    if !arrived {
        TRAY_TX
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(Cmd::Open)
            .unwrap()
    }
}

fn quit() {
    if let Some(callback) = &*ON_QUIT.lock().unwrap() {
        (callback)();
    }
    TRAY_TX
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .send(Cmd::Quit)
        .unwrap();
}
