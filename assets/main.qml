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
    height: 500
    minimumWidth: 640
    minimumHeight: 480

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

    TabBar {
        id: bar
        width: parent.width

        TabButton {
            text: qsTr("Wallpapers")
        }
        TabButton {
            text: qsTr("Setting")
        }
    }

    StackLayout {
        width: parent.width
        height: parent.height - bar.height
        currentIndex: bar.currentIndex
        anchors.top: bar.bottom

        Item {
            MainPage {
                id: mainPage
            }
        }

        SettingPage {}
    }
}
