#![feature(try_trait)]
#![feature(try_blocks)]
#![recursion_limit = "1024"]

use std::fs::{self, File};
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;

use cpp::*;
use cstr::*;
use qmetaobject::*;

mod async_utils;
mod config;
mod implementation;
mod listmodel;
mod systray;

cpp! {{
    #include <malloc.h>
    #include <QtGui/QIcon>
    #include <QtQuick/QtQuick>
    #include <QtCore/QTranslator>
    #include <QtWidgets/QApplication>

    static QTranslator translator;

    struct QmlEngineHolder {
        std::unique_ptr<QApplication> app;
        std::unique_ptr<QQmlApplicationEngine> engine;
        std::unique_ptr<QQuickView> view;
    };
}}

#[cfg(not(debug_assertions))]
qrc! { init_ressource,
     "/" {
         "assets/main.qml",
         "assets/WallpaperFlow.qml",
         "assets/SettingPage.qml",
         "assets/background.png",
         "assets/livewallpaper.svg",
         "assets/emblem-favorite-symbolic.svg",
         "assets/i18n/zh_CN.qm",
     },
}

fn main() {
    #[cfg(not(debug_assertions))]
    init_ressource();
    qml_register_type::<systray::TrayProxy>(cstr!("TrayProxy"), 1, 0, cstr!("TrayProxy"));

    systray::run_tray_in_background();

    let mut engine = create_engine();
    loop {
        let wallpapers = QObjectBox::new(create_wallpapers());
        let wallpapers = wallpapers.pinned();

        engine.set_object_property("wallpapers".into(), wallpapers.clone());
        engine.load_file(
            #[cfg(debug_assertions)]
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/main.qml").into(),
            #[cfg(not(debug_assertions))]
            "qrc:/assets/main.qml".into(),
        );

        engine.exec();

        wallpapers
            .borrow()
            .config
            .borrow()
            .save()
            .expect("Failed to save configs");

        if !wallpapers.borrow().config.borrow().auto_change.enable {
            break;
        }
        let engine_ptr = &mut engine;

        cpp!(unsafe [engine_ptr as "QmlEngineHolder*"] {
            engine_ptr->engine.reset(new QQmlApplicationEngine);
            malloc_trim(0);
        });

        match systray::wait() {
            systray::Cmd::Open => {
                continue;
            }
            systray::Cmd::Quit => break,
        }
    }
}

fn create_engine() -> QmlEngine {
    let mut engine = QmlEngine::new();
    let engine_ptr = &mut engine;

    cpp!(unsafe [engine_ptr as "QmlEngineHolder*"] {
        QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);

        translator.load(QLocale::system(), "", "", ":/assets/i18n");
        QApplication::installTranslator(&translator);

        auto icon = QIcon::fromTheme("livewallpaper", QIcon(":/assets/livewallpaper.svg"));
        engine_ptr->app->setWindowIcon(icon);
    });

    engine
}

fn create_wallpapers() -> implementation::Wallpapers {
    let set_wallpaper_fallback = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/variety/data/scripts/set_wallpaper"
    ));

    let wallpapers = implementation::Wallpapers::new();

    {
        let config = &wallpapers.config.borrow();
        if !config.cache_dir.exists() {
            fs::create_dir_all(&config.cache_dir).expect("create cache_dir");
        }
        let fallback_script_path = config.cache_dir.join("set_wallpaper");
        let mut fallback_script =
            File::create(&fallback_script_path).expect("create set_wallpaper");
        fallback_script
            .write_all(set_wallpaper_fallback)
            .expect("write set_wallpaper");
        fallback_script
            .set_permissions(fs::Permissions::from_mode(0o755))
            .expect("set 755 for set_wallpaper");
        let mut de = config.de.borrow_mut();
        let custom_cmd = &mut de.last_mut().unwrap().cmd;
        if custom_cmd.to_slice().is_empty() {
            *custom_cmd = format!(
                "{} \"$WALLPAPER\" auto",
                &*fallback_script_path.to_string_lossy()
            )
            .into();
        }
    }

    wallpapers
}
