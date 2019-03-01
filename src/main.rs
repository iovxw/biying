use cpp::*;
use cstr::*;
use qmetaobject::*;

cpp! {{
    #include <QtCore/QTranslator>
    #include <QtWidgets/QApplication>

    static QTranslator translator;
}}

#[derive(QObject, Default)]
struct Greeter {
    base: qt_base_class!(trait QObject),
    name: qt_property!(QString; NOTIFY name_changed),
    name_changed: qt_signal!(),
    compute_greetings: qt_method!(fn compute_greetings(&self, verb : String) -> QString {
        return (verb + " " + &self.name.to_string()).into()
    }),
}

qrc! { init_ressource,
     "/" {
         "assets/main.qml",
         "assets/i18n/zh_CN.qm",
     },
}

fn main() {
    init_ressource();
    qml_register_type::<Greeter>(cstr!("Greeter"), 1, 0, cstr!("Greeter"));
    let mut engine = QmlEngine::new();
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
