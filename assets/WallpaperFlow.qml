import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3
import QtGraphicalEffects 1.0

GridView {
    property int previewH: 480/3
    property int previewW: 800/3

    cellHeight: previewH
    cellWidth: previewW
    anchors.fill: parent
    anchors.leftMargin: (parent.width % previewW) / 2
    clip: true

    property bool loading
    signal nextPage()
    signal download(int index)
    signal likeClicked(int index)
    signal setWallpaperClicked(int index)

    Component.onCompleted: nextPage()

    ScrollBar.vertical: ScrollBar { }

    onMovementEnded: if (atYEnd && !loading) {
        nextPage()
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
                    if (!model.loading) {
                        download(index)
                    }
                }
            }

            Button {
                id: likeBtn
                height: parent.height / 4
                width: height
                icon.name: "emblem-favorite-symbolic"
                icon.source: "emblem-favorite-symbolic.svg"
                icon.width: width
                icon.height: height
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                onClicked: likeClicked(index)
                states: [
                    State {
                        when: model.like
                        PropertyChanges {
                            target: likeBtn
                            icon.color: "red"
                        }
                    }
                ]
            }
        }

        Popup {
            id: popup
            width: window.width
            height: window.height
            anchors.centerIn: Overlay.overlay
            background: FastBlur {
                source: Image {
                    cache: false
                    width: popup.width
                    height: popup.height
                    source: model.preview
                    fillMode: Image.PreserveAspectCrop
                }
                radius: 128
            }

            BusyIndicator {
                height: 60
                visible: model.loading
                anchors.centerIn: parent
            }

            Image {
                id: wallpaperImage
                height: parent.height - popupBtn1.height - popup.padding
                width: parent.width
                fillMode: Image.PreserveAspectFit
                source: model.image
                visible: !model.loading

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
                    anchors.topMargin: Math.round((parent.height - parent.paintedHeight) / 2)
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
                    anchors.leftMargin: Math.round((parent.width - parent.paintedWidth) / 2)
                    anchors.bottomMargin: Math.round((parent.height - parent.paintedHeight) / 2)
                    color: Qt.rgba(0, 0, 0, 0.3)
                    visible: wallpaperImageArea.containsMouse

                    ListView {
                        model: metas
                        anchors.fill: parent
                        anchors.margins: popup.padding
                        clip: true
                        delegate: Text {
                            color: "white"
                            // https://bugreports.qt.io/browse/QTBUG-49983
                            text: modelData.market + ": " + modelData.info
                        }
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
                onClicked: setWallpaperClicked(index)
            }

            Button {
                id: popupBtn2
                icon.name: "emblem-favorite-symbolic"
                icon.source: "emblem-favorite-symbolic.svg"
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                onClicked: likeClicked(index)
                states: [
                    State {
                        when: model.like
                        PropertyChanges {
                            target: popupBtn2
                            icon.color: "red"
                        }
                    }
                ]
            }
        }
    }

    footer: BusyIndicator {
        visible: loading
        height: 60
        width: parent.width
    }
}
