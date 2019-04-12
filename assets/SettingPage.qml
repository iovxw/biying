import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3

Rectangle {
    color: Qt.rgba(0, 0, 0, 0.3)

    Pane {
        height: parent.height
        width: 600
        anchors.horizontalCenter: parent.horizontalCenter

        ScrollView {
            anchors.fill: parent
            contentWidth: width
            contentHeight: contentChildren[0].height
            anchors.rightMargin: 10
            anchors.leftMargin: 10
            clip: true

            ColumnLayout {
                width: parent.width
                spacing: 10
                GroupBox {
                    Layout.fillWidth: true
                    title: qsTr("Desktop Enviroment")

                    ColumnLayout {
                        width: parent.width
                        ComboBox {
                            id: currentDE
                            Layout.fillWidth: true
                            model: ["GNOME", "KDE", "Xfce", "LXQt", "LXDE", "Cinnamon", "Deepin", "Budgie", "Enlightenment", "MATE", "Other"]
                        }

                        Label {
                            text: qsTr("Command to set wallpaper:")
                        }

                        TextField {
                            Layout.fillWidth: true
                            enabled: currentDE.currentIndex == currentDE.count - 1
                            selectByMouse: true
                            text: "command"
                        }
                    }
                }
                GroupBox {
                    Layout.fillWidth: true
                    title: qsTr("Automatically Change Wallpaper")
                    GridLayout {
                        width: parent.width
                        rowSpacing: 10
                        columns: 2
                        Label {
                            text: qsTr("Enable")
                        }
                        Switch {
                            id: autoChangeWallpaperBtn
                            Layout.alignment: Qt.AlignRight
                        }

                        Label {
                            text: qsTr("Interval")
                        }
                        RowLayout {
                            Layout.alignment: Qt.AlignRight
                            SpinBox {
                                enabled: autoChangeWallpaperBtn.checked
                                value: 5
                                to: 999
                            }
                            Label {
                                text: qsTr("minuts")
                            }
                        }

                        GroupBox {
                            Layout.columnSpan: 2
                            Layout.fillWidth: true
                            title: qsTr("Mode")

                            RowLayout {
                                width: parent.width
                                enabled: autoChangeWallpaperBtn.checked
                                RadioButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    checked: true
                                    text: qsTr("Newest")
                                }
                                RadioButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: qsTr("Favourites")
                                }
                                RadioButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: qsTr("Random")
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
