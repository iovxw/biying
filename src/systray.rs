use std::sync::{mpsc, Mutex};

use image::ImageFormat;
use ksni;
use qmetaobject::*;

lazy_static! {
    /// Send Cmd to TrayProxy
    static ref REMOTE_CMD_SENDER: Mutex<Option<Box<dyn Fn(Cmd) + Send>>> = Mutex::default();
    /// If TrayProxy isn't running, all Cmd will be here
    static ref LOCAL_CMD_RECIVER: Mutex<Option<mpsc::Receiver<Cmd>>> = Mutex::default();
}

#[derive(Debug, Clone, Copy)]
pub enum Cmd {
    Open,
    Quit,
}

pub fn wait() -> Cmd {
    let rx = LOCAL_CMD_RECIVER
        .try_lock()
        .expect("another thread is waiting");
    let rx = rx.as_ref().expect("no tray is running");
    rx.recv().unwrap()
}

#[derive(QObject, Default)]
pub struct TrayProxy {
    base: qt_base_class!(trait QObject),
    pub connect_to_backend: qt_method!(fn(&mut self)),
    pub open: qt_signal!(),
    pub quit: qt_signal!(),
}

impl TrayProxy {
    fn connect_to_backend(&mut self) {
        let qptr = QPointer::from(&*self);
        let on_open_callback = queued_callback(move |cmd| {
            let this = qptr
                .as_ref()
                .unwrap_or_else(|| unreachable!("REMOTE_CMD_SENDER should have beed dropped"));
            match cmd {
                Cmd::Open => this.open(),
                Cmd::Quit => this.quit(),
            }
        });
        *REMOTE_CMD_SENDER.lock().unwrap() = Some(Box::new(on_open_callback));
    }
}

impl Drop for TrayProxy {
    fn drop(&mut self) {
        *REMOTE_CMD_SENDER.lock().unwrap() = None;
    }
}

pub struct Tray {
    cmd_tx: mpsc::Sender<Cmd>,
}

impl Tray {
    fn emit_cmd(&mut self, cmd: Cmd) {
        let tray_proxy_exist = REMOTE_CMD_SENDER
            .lock()
            .unwrap()
            .as_ref()
            .map(|emit| (emit)(cmd))
            .is_some();
        if !tray_proxy_exist {
            self.cmd_tx.send(cmd).unwrap()
        }
    }
}

impl ksni::Tray for Tray {
    fn icon_name(&self) -> String {
        "livewallpaper-indicator".to_owned()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        icons()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Open".into(),
                activate: Box::new(|this: &mut Self| this.emit_cmd(Cmd::Open)),
                ..Default::default()
            }
            .into(),
            MenuItem::Sepatator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| this.emit_cmd(Cmd::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn run_tray_in_background() {
    let mut cmd_rx = LOCAL_CMD_RECIVER.lock().unwrap();
    if cmd_rx.is_none() {
        let (sender, recver) = mpsc::channel();
        *cmd_rx = Some(recver);

        let service = ksni::TrayService::new(Tray { cmd_tx: sender });
        service.spawn();
    }
}

// TODO: const fn
fn icons() -> Vec<ksni::Icon> {
    let images: &[&[u8]] = &[
        // include_bytes!("../assets/tray-icon_16.png"),
        include_bytes!("../assets/tray-icon_24.png"),
        include_bytes!("../assets/tray-icon_32.png"),
        include_bytes!("../assets/tray-icon_48.png"),
        include_bytes!("../assets/tray-icon_64.png"),
    ];
    let mut icons = Vec::with_capacity(images.len());
    for img in images.iter() {
        let img = image::load_from_memory_with_format(img, ImageFormat::Png).expect("");
        let mut img = img.to_rgba8();
        let (width, height) = img.dimensions();
        for pixel in img.pixels_mut() {
            // rgba rotated to argb
            pixel.0.rotate_right(1);
        }
        let icon = ksni::Icon {
            width: width as i32,
            height: height as i32,
            data: img.into_raw(),
        };
        icons.push(icon);
    }
    icons
}
