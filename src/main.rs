#![feature(type_alias_enum_variants)]
#![feature(try_trait)]
#![feature(result_map_or_else)]

use cpp::*;
use qmetaobject::*;

mod implementation;
mod listmodel;
mod config;

cpp! {{
    #include <QtCore/QTranslator>
    #include <QtWidgets/QApplication>

    static QTranslator translator;
}}

qrc! { init_ressource,
     "/" {
         "assets/main.qml",
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
    }
    engine.load_file(":/assets/main.qml".into());
    engine.exec();
}
