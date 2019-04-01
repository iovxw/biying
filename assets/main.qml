import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3
import QtGraphicalEffects 1.0

ApplicationWindow {
    id: window
    visible: true
    //: Window title
    title: qsTr("Biying Wallpaper")

    width: 900
    minimumWidth: 640
    minimumHeight: 480
    height: 500

    background: FastBlur {
        source: Image {
            id: windowBkgImg
            width: window.width
            height: window.height
            source: "background.png"
            fillMode: Image.PreserveAspectCrop
        }
        radius: 64
    }

    MainPage {
        id: mainPage
    }
}
