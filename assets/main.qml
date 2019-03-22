import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.1

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
                        color: Qt.rgba(Math.random(), Math.random(), Math.random(), Math.random())

                        BusyIndicator {
                            height: 64
                            anchors.centerIn: parent
                        }

                        Image {
                            anchors.fill: parent
                            source: model.preview

                            MouseArea {
                                anchors.fill: parent
                                onClicked: {
                                    model.like = !model.like;
                                    console.log("like:", index, model.like)
                                }
                            }
                        }
                    }
                }
            }

            BusyIndicator {
                height: 64
                anchors.horizontalCenter: parent.horizontalCenter
            }
        }
    }
}
