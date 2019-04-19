#![feature(type_alias_enum_variants)]
#![feature(try_trait)]
#![feature(result_map_or_else)]
#![recursion_limit="128"]

use cpp::*;
use qmetaobject::*;

mod implementation;
mod listmodel;
mod config;

cpp! {{
    #include <memory>
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
         "assets/i18n/zh_CN.qm",
     },
}

fn main() {
    init_ressource();
    let mut engine = QmlEngine::new();

    let wallpapers = implementation::Wallpapers::new();
    let wallpapers = QObjectBox::new(wallpapers);
    let wallpapers = wallpapers.pinned();

    engine.set_object_property("wallpapers".into(), wallpapers);

    let engine = &mut engine;
    unsafe {
        cpp!([] {
            translator.load(QLocale::system(), "", "", ":/assets/i18n");
            QApplication::installTranslator(&translator);
        });
        cpp!([engine as "QmlEngineHolder*"] {
            engine->app->setQuitOnLastWindowClosed(false);
        });
    }
    engine.load_file("qrc:/assets/main.qml".into());
    engine.exec();
}
