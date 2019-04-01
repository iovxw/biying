import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3
import QtGraphicalEffects 1.0

GridView {
    property int previewH: 480/3
    property int previewW: 800/3

    cellHeight: previewH
    cellWidth: previewW
    width: Math.floor(parent.width / previewW) * previewW
    height: parent.height
    anchors.horizontalCenter: parent.horizontalCenter
    model: wallpapers.list

    Component.onCompleted: {
        wallpapers.fetch_next_page()
        wallpapers.onError.connect(function(err) {
            console.log("error:", err)
        })
    }

    onMovementEnded: if (atYEnd) {
        wallpapers.fetch_next_page()
    }

    delegate: Rectangle {
        height: previewH
        width: previewW
        color: Qt.rgba(Math.random(), Math.random(), Math.random(), 0.3)

        BusyIndicator {
            height: parent.height / 2
            anchors.centerIn: parent
        }

        Image {
            anchors.fill: parent
            source: model.preview

            MouseArea {
                anchors.fill: parent
                onClicked: if (parent.status == Image.Ready) {
                    popup.open()
                    wallpapers.download(index)
                }
            }

            Button {
                height: parent.height / 4
                width: height
                icon.name: "emblem-favorite-symbolic"
                icon.color: if (model.like) { "red" } else { "white"  }
                icon.width: width
                icon.height: height
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                onClicked: wallpapers.like(index)
            }
        }

        Popup {
            id: popup
            width: window.width
            height: window.height
            anchors.centerIn: Overlay.overlay
            background: FastBlur {
                source: Image {
                    width: popup.width
                    height: popup.height
                    source: model.preview
                    fillMode: Image.PreserveAspectCrop
                }
                radius: 128
            }

            BusyIndicator {
                height: 60
                anchors.centerIn: parent
                visible: !model.image
            }

            Image {
                id: wallpaperImage
                height: parent.height - popupBtn1.height - popup.padding
                width: parent.width
                fillMode: Image.PreserveAspectFit
                source: model.image
                visible: model.image

                MouseArea {
                    id: wallpaperImageArea
                    anchors.fill: parent
                    hoverEnabled: true
                }

                Rectangle {
                    height: childrenRect.height + popup.padding
                    width: childrenRect.width + popup.padding
                    anchors.right: infolist.right
                    anchors.top: parent.top
                    anchors.topMargin: (parent.height - parent.paintedHeight) / 2
                    color: infolist.color
                    visible: infolist.visible

                    Text {
                        x: popup.padding / 2
                        y: popup.padding / 2
                        color: "white"
                        text: "© " + model.copyright
                    }
                }

                Rectangle {
                    id: infolist
                    height: parent.paintedHeight * 0.2
                    width: parent.paintedWidth
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: (parent.width - parent.paintedWidth) / 2
                    anchors.bottomMargin: (parent.height - parent.paintedHeight) / 2
                    color: Qt.rgba(0, 0, 0, 0.3)
                    visible: wallpaperImageArea.containsMouse

                    ListView {
                        model: ListModel {
                            id: infoModel
                        }
                        anchors.fill: parent
                        anchors.margins: popup.padding
                        clip: true
                        delegate: Text {
                            color: "white"
                            text: model.market + ": " + model.info
                        }
                    }
                    Component.onCompleted: {
                        // a workaround
                        infoModel.append(model.metas)
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                onClicked: {
                    popup.close()
                }
            }

            Button {
                id: popupBtn1
                text: qsTr("Set as Wallpaper")
                anchors.right: popupBtn2.left
                anchors.bottom: parent.bottom
                anchors.rightMargin: popup.padding / 2
            }

            Button {
                id: popupBtn2
                icon.name: "emblem-favorite-symbolic"
                icon.color: if (model.like) { "red" } else { "white"  }
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                onClicked: wallpapers.like(index)
            }
        }
    }

    footer: BusyIndicator {
        height: 60
        width: parent.width
    }
}
