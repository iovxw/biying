import QtQuick 2.8
import QtQuick.Layouts 1.3
import QtQuick.Controls 2.3
import QtGraphicalEffects 1.0
import Qt.labs.platform 1.0

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

    Component.onCompleted: {
        wallpapers.onError.connect(function(err) {
            console.log("error:", err)
        })
    }

    onClosing: {
        if (!wallpapers.config.auto_change.enable) {
            Qt.quit()
        }
    }

    SystemTrayIcon {
        visible: wallpapers.config.auto_change.enable
        iconSource: "icon.png"
        iconName: "livewallpaper-indicator"

        onActivated: {
            window.show()
            window.raise()
            window.requestActivate()
        }

        menu: Menu {
            MenuItem {
                text: qsTr("Open")
                onTriggered: {
                    window.show()
                    window.raise()
                    window.requestActivate()
                }
            }
            MenuItem {
                text: qsTr("Quit")
                onTriggered: Qt.quit()
            }
        }
    }

    TabBar {
        id: bar
        width: parent.width
        currentIndex: 1

        TabButton {
            text: qsTr("Favourites")
        }
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
            WallpaperFlow {
                model: wallpapers.favourites
                loading: wallpapers.favourites_loading
                onNextPage: {
                    wallpapers.next_page_favourites()
                }
                onDownload: {
                    wallpapers.download(index, true)
                }
                onLikeClicked: {
                    wallpapers.like(index, true)
                }
                onSetWallpaperClicked: {
                    wallpapers.set_wallpaper(index, true)
                }
            }
        }

        Item {
            WallpaperFlow {
                id: mainPage
                model: wallpapers.list
                loading: wallpapers.list_loading
                onNextPage: {
                    wallpapers.fetch_next_page()
                }
                onDownload: {
                    wallpapers.download(index, false)
                }
                onLikeClicked: {
                    wallpapers.like(index, false)
                }
                onSetWallpaperClicked: {
                    wallpapers.set_wallpaper(index, false)
                }
            }
        }

        SettingPage {}
    }
}
