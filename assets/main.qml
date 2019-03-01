import QtQuick 2.12
import QtQuick.Layouts 1.12
import QtQuick.Controls 2.12

ApplicationWindow {
    property int previewH: 150
    property int previewW: 200

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

            Repeater {
                model: [1,2,3,4,5]

                delegate: Rectangle {
                    height: previewH
                    width: previewW
                    color: Qt.rgba(Math.random(), Math.random(), Math.random(), Math.random())

                    Text {
                        text: modelData
                    }
                }
            }
        }
    }
}
