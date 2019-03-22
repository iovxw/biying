import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3
import QtGraphicalEffects 1.0

ApplicationWindow {
    property int previewH: 480/3
    property int previewW: 800/3

    visible: true
    //: Window title
    title: qsTr("Biying Wallpaper")

    width: 640
    minimumWidth: previewW
    minimumHeight: previewH
    height: 480

    background: FastBlur {
        source: Image {
            source: "background.png"
        }
        radius: 64
    }

    Flickable {
        anchors.fill: parent
        flickableDirection: Flickable.VerticalFlick
        boundsBehavior: Flickable.DragOverBounds
        contentHeight: column.height
        ScrollBar.vertical: ScrollBar { }

        Component.onCompleted: {
            wallpapers.fetch_next_page()
            wallpapers.onError.connect(function(err) {
                console.log("error:", err)
            })
        }

        onMovementEnded: if (atYEnd) {
            wallpapers.fetch_next_page()
        }

        Column {
            id: column
            width: parent.width

            Grid {
                columns: parent.width / previewW
                anchors.horizontalCenter: parent.horizontalCenter

                Repeater {
                    model: wallpapers.list

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
                    }
                }
            }

            BusyIndicator {
                height: previewH / 2
                anchors.horizontalCenter: parent.horizontalCenter
            }
        }
    }
}
