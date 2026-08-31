import AppKit
import SwiftUI

@main
struct KitchenSinkApp: App {
    @StateObject private var host = MiniHost()

    var body: some Scene {
        WindowGroup("App Kit Kitchen Sink") {
            ContentView(host: host)
                .frame(minWidth: 1_080, minHeight: 700)
                .onReceive(NotificationCenter.default.publisher(
                    for: NSApplication.willTerminateNotification
                )) { _ in
                    host.shutdown()
                }
        }
        .defaultSize(width: 1_360, height: 860)
    }
}
