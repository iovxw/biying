#![feature(type_alias_enum_variants)]
#![feature(try_trait)]
#![feature(try_blocks)]
#![feature(result_map_or_else)]
#![feature(vec_remove_item)]
#![recursion_limit = "128"]

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
         "assets/icon.png",
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
        cpp!([engine as "QmlEngineHolder*"] {
            QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);

            translator.load(QLocale::system(), "", "", ":/assets/i18n");
            QApplication::installTranslator(&translator);

            auto icon = QIcon::fromTheme("livewallpaper", QIcon(":/assets/icon.png"));
            engine->app->setWindowIcon(icon);

            engine->app->setQuitOnLastWindowClosed(false);
        });
    }
    engine.load_file("qrc:/assets/main.qml".into());
    engine.exec();
    wallpapers.borrow().config.borrow().save().expect("Failed to save configs");
}
