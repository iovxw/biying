import QtQuick 2.12
import QtQuick.Layouts 1.12
import QtQuick.Controls 2.12

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
        contentHeight: grid.height
        ScrollBar.vertical: ScrollBar { }
        Grid {
            id: grid
            columns: parent.width / previewW
            anchors.horizontalCenter: parent.horizontalCenter

            Button {
                text: "刷新！"
                onClicked: {
                    wallpapers.fetch_next_page()
                    wallpapers.onError.connect(function(err) {
                        console.log("error:", err)
                    })
                }
            }

            Repeater {
                model: wallpapers.list

                delegate: Rectangle {
                    height: previewH
                    width: previewW
                    color: Qt.rgba(Math.random(), Math.random(), Math.random(), Math.random())

                   // onClick: {
                   //     // start loading animation
                   //     wallpapers.loadNextPage();
                   // }

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
    }
}
