#![feature(type_alias_enum_variants)]
#![feature(try_trait)]
#![feature(try_blocks)]
#![feature(result_map_or_else)]
#![feature(vec_remove_item)]
#![recursion_limit = "128"]

use std::fs::{self, File};
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;

use cpp::*;
use qmetaobject::*;

mod config;
mod implementation;
mod listmodel;

cpp! {{
    #include <memory>
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
    init_ressource();

    let set_wallpaper_fallback = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/variety/data/scripts/set_wallpaper"
    ));

    let mut engine = QmlEngine::new();

    let wallpapers = implementation::Wallpapers::new();
    let wallpapers = QObjectBox::new(wallpapers);
    let wallpapers = wallpapers.pinned();

    {
        let config = &wallpapers.borrow().config.borrow();
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
            *custom_cmd = format!("{} \"$WALLPAPER\" auto",&*fallback_script_path.to_string_lossy()).into();
        }
    }

    engine.set_object_property("wallpapers".into(), wallpapers);

    let engine = &mut engine;
    unsafe {
        cpp!([engine as "QmlEngineHolder*"] {
            QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);

            translator.load(QLocale::system(), "", "", ":/assets/i18n");
            QApplication::installTranslator(&translator);

            auto icon = QIcon::fromTheme("livewallpaper", QIcon(":/assets/livewallpaper.svg"));
            engine->app->setWindowIcon(icon);

            engine->app->setQuitOnLastWindowClosed(false);
        });
    }
    engine.load_file("qrc:/assets/main.qml".into());
    engine.exec();
    wallpapers
        .borrow()
        .config
        .borrow()
        .save()
        .expect("Failed to save configs");
}
