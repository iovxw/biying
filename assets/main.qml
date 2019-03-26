import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3
import QtGraphicalEffects 1.0

ApplicationWindow {
    property int previewH: 480/3
    property int previewW: 800/3

    id: window
    visible: true
    //: Window title
    title: qsTr("Biying Wallpaper")

    width: 640
    minimumWidth: previewW
    minimumHeight: previewH
    height: 480

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

    GridView {
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
                    onClicked: model.like = !model.like
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

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        popup.close()
                    }
                }

                BusyIndicator {
                    height: parent.height / 2
                    anchors.centerIn: parent
                }

                Image {
                    height: parent.height - popupBtn1.height - popup.padding
                    width: parent.width
                    fillMode: Image.PreserveAspectFit
                    source: model.image
                    visible: model.image
                }

                Button {
                    id: popupBtn1
                    text: qsTr("Set as Wallpaper")
                    anchors.right: popupBtn2.left
                    anchors.bottom: parent.bottom
                    anchors.rightMargin: 5
                }

                Button {
                    id: popupBtn2
                    icon.name: "emblem-favorite-symbolic"
                    icon.color: if (model.like) { "red" } else { "white"  }
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    onClicked: model.like = !model.like
                }
            }
        }

        footer: BusyIndicator {
            height: 60
            width: parent.width
        }
    }
}
