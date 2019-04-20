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
                            id: currentDe
                            Layout.fillWidth: true
                            textRole: "name"
                            Component.onCompleted: currentIndex = wallpapers.qconfig.de_index
                            onCurrentIndexChanged: {
                                wallpapers.qconfig.de_index = currentIndex
                                let index = model.index(currentIndex, 0)
                                currentDeCmd.text = model.data(index, Qt.UserRole + 1)
                            }
                            model: wallpapers.qconfig.de
                        }

                        Label {
                            text: qsTr("Command to set wallpaper:")
                        }

                        TextField {
                            id: currentDeCmd
                            Layout.fillWidth: true
                            enabled: currentDe.currentIndex == currentDe.count - 1
                            selectByMouse: true
                            Component.onCompleted: {
                                let index = currentDe.model.index(currentDe.currentIndex, 0)
                                text = currentDe.model.data(index, Qt.UserRole + 1)
                            }
                            onTextChanged: if (enabled) {
                                let index = currentDe.model.index(currentDe.currentIndex, 0)
                                currentDe.model.setData(index, text, Qt.UserRole + 1)
                            }
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
                            Component.onCompleted: checked = wallpapers.qconfig.auto_change.enable
                            onCheckedChanged: wallpapers.qconfig.auto_change.enable = checked
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
                                editable: true
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
                                    Component.onCompleted: checked = wallpapers.qconfig.auto_change.mode == 0
                                    onCheckedChanged: wallpapers.qconfig.auto_change.mode = 0
                                    text: qsTr("Newest")
                                }
                                RadioButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    Component.onCompleted: checked = wallpapers.qconfig.auto_change.mode == 1
                                    onCheckedChanged: wallpapers.qconfig.auto_change.mode = 1
                                    text: qsTr("Favourites")
                                }
                                RadioButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    Component.onCompleted: checked = wallpapers.qconfig.auto_change.mode == 2
                                    onCheckedChanged: wallpapers.qconfig.auto_change.mode = 2
                                    text: qsTr("Random")
                                }
                            }
                        }
                    }
                }

                GroupBox {
                    title: qsTr("Resolution")
                    Layout.fillWidth: true

                    RowLayout {
                        width: parent.width

                        Label {
                            text: qsTr("Preview")
                        }
                        ComboBox {
                            Component.onCompleted: currentIndex = wallpapers.qconfig.resolution.preview_index
                            onCurrentIndexChanged: wallpapers.qconfig.resolution.preview_index = currentIndex
                            model: wallpapers.qconfig.resolution.preview
                        }
                        Label {
                            text: qsTr("Download")
                        }
                        ComboBox {
                            Component.onCompleted: currentIndex = wallpapers.qconfig.resolution.download_index
                            onCurrentIndexChanged: wallpapers.qconfig.resolution.download_index = currentIndex
                            model: wallpapers.qconfig.resolution.download
                        }
                    }
                }

                GroupBox {
                    title: qsTr("Disk usage")
                    Layout.fillWidth: true

                    GridLayout {
                        width: parent.width
                        columns: 4
                        rowSpacing: 10

                        Label {
                            text: qsTr("Favourites")
                            Layout.preferredWidth: 1
                        }
                        Label {
                            text: qsTr("1 MB")
                            Layout.preferredWidth: 1
                        }
                        Label {
                            text: qsTr("Others")
                            Layout.preferredWidth: 1
                        }
                        Label {
                            text: qsTr("1 MB")
                            Layout.preferredWidth: 1
                        }

                        Label {
                            Layout.columnSpan: 2
                            text: qsTr("Autoremove wallpapers from")
                        }
                        RowLayout {
                            Layout.columnSpan: 2
                            Layout.alignment: Qt.AlignRight
                            SpinBox {
                                Component.onCompleted: value = wallpapers.qconfig.autoremove
                                onValueChanged: wallpapers.qconfig.autoremove = value
                                to: 999
                                editable: true
                            }
                            Label {
                                text: qsTr("days ago")
                            }
                        }

                        Button {
                            Layout.columnSpan: 4
                            Layout.alignment: Qt.AlignRight
                            text: qsTr("Clear all other wallpapers")
                        }
                    }
                }
            }
        }
    }
}
