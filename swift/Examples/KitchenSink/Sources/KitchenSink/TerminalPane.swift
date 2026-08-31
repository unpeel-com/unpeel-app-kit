import AppKit
import SwiftUI
@preconcurrency import SwiftTerm

struct TerminalLaunch: Sendable {
    let executable: String
    let currentDirectory: String
    let environment: [String: String]
}

/// The single engine boundary in the kitchen sink. The product Host can swap
/// this implementation for GhosttyKit without changing any session logic.
@MainActor
final class TerminalEngineController: NSObject {
    let view: LocalProcessTerminalView
    var onTermination: ((Int32?) -> Void)?
    private(set) var isRunning = false

    override init() {
        view = LocalProcessTerminalView(
            frame: CGRect(x: 0, y: 0, width: 900, height: 600),
            font: NSFont.monospacedSystemFont(ofSize: 12.5, weight: .regular),
            options: TerminalOptions(termName: "xterm-256color", scrollback: 20_000)
        )
        super.init()
        view.processDelegate = self
        view.nativeBackgroundColor = NSColor(
            calibratedRed: 0.055,
            green: 0.063,
            blue: 0.078,
            alpha: 1
        )
        view.nativeForegroundColor = NSColor(
            calibratedRed: 0.86,
            green: 0.88,
            blue: 0.91,
            alpha: 1
        )
        view.caretColor = .systemTeal
    }

    func start(_ launch: TerminalLaunch) {
        guard !isRunning else { return }
        isRunning = true
        let environment = launch.environment
            .map { "\($0.key)=\($0.value)" }
            .sorted()
        view.startProcess(
            executable: launch.executable,
            args: [],
            environment: environment,
            execName: URL(fileURLWithPath: launch.executable).lastPathComponent,
            currentDirectory: launch.currentDirectory
        )
    }

    func terminate() {
        guard isRunning else { return }
        view.terminate()
    }
}

extension TerminalEngineController: @preconcurrency LocalProcessTerminalViewDelegate {
    func sizeChanged(
        source _: LocalProcessTerminalView,
        newCols _: Int,
        newRows _: Int
    ) {}

    func setTerminalTitle(source _: LocalProcessTerminalView, title _: String) {}

    func hostCurrentDirectoryUpdate(source _: TerminalView, directory _: String?) {}

    func processTerminated(source _: TerminalView, exitCode: Int32?) {
        isRunning = false
        onTermination?(exitCode)
    }
}

struct TerminalPane: NSViewRepresentable {
    let engine: TerminalEngineController

    func makeNSView(context _: Context) -> LocalProcessTerminalView {
        engine.view
    }

    func updateNSView(_: LocalProcessTerminalView, context _: Context) {}
}
