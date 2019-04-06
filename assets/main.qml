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

        Rectangle {
            color: Qt.rgba(0, 0, 0, 0.3)

            Pane {
                height: parent.height
                width: 600
                anchors.horizontalCenter: parent.horizontalCenter

                ColumnLayout {
                    anchors.fill: parent

                    GroupBox {
                        Layout.fillWidth: true
                        title: qsTr("Desktop Enviroment")

                        ColumnLayout {
                            width: parent.width

                            RadioButton {
                                checked: true
                                text: qsTr("GNOME")
                            }
                            RadioButton {
                                text: qsTr("KDE")
                            }
                            RadioButton {
                                text: qsTr("Xfce")
                            }
                            RadioButton {
                                text: qsTr("LXQt")
                            }
                            RadioButton {
                                text: qsTr("LXDE")
                            }
                            RadioButton {
                                text: qsTr("Cinnamon")
                            }
                            RadioButton {
                                text: qsTr("Deepin")
                            }
                            RadioButton {
                                text: qsTr("Budgie")
                            }
                            RadioButton {
                                text: qsTr("Enlightenment")
                            }
                            RadioButton {
                                text: qsTr("MATE")
                            }
                            RadioButton {
                                id: customCommandBtn
                                text: qsTr("Custom command")
                            }
                            TextField {
                                Layout.fillWidth: true
                                enabled: customCommandBtn.checked
                                text: "command"
                            }
                        }
                    }
                }
            }
        }
    }
}
