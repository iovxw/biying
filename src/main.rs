#![feature(type_alias_enum_variants)]
#![feature(try_trait)]

use cpp::*;
use qmetaobject::*;

mod implementation;
mod listmodel;

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

    let wallpapers = implementation::Wallpapers::default();
    let wallpapers = QObjectBox::new(wallpapers);
    let wallpapers = wallpapers.pinned();
    //unsafe {
    //    let list = QObjectPinned::new(&wallpapers.borrow().list);
    //    engine.set_object_property("wallpaperList".into(), list);
    //}
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
