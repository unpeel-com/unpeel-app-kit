import AppKit
import SwiftUI
@preconcurrency import SwiftTerm

struct TerminalLaunch: Sendable {
    let executable: String
    let currentDirectory: String
    let environment: [String: String]
}

/// SwiftTerm does not claim first responder from its selection-only mouse path.
/// The mini-host makes that explicit so a click always arms the PTY for typing.
final class KitchenSinkTerminalView: LocalProcessTerminalView {
    var wantsInitialFocus = false
    var applicationSelectedText: String?
    var mirrorsApplicationMouseSelection = false

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard wantsInitialFocus, window != nil else { return }
        wantsInitialFocus = false
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.window?.makeFirstResponder(self)
        }
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }

    override func mouseDown(with event: NSEvent) {
        // Arm the terminal before SwiftTerm decides whether this press belongs
        // to the child application's mouse protocol or local text selection.
        window?.makeFirstResponder(self)
        guard shouldMirrorApplicationSelection(event) else {
            super.mouseDown(with: event)
            return
        }
        performNativeSelectionPass { super.mouseDown(with: event) }
        super.mouseDown(with: event)
    }

    override func mouseDragged(with event: NSEvent) {
        guard shouldMirrorApplicationSelection(event) else {
            super.mouseDragged(with: event)
            return
        }
        performNativeSelectionPass { super.mouseDragged(with: event) }
        super.mouseDragged(with: event)
    }

    override func mouseUp(with event: NSEvent) {
        guard shouldMirrorApplicationSelection(event) else {
            super.mouseUp(with: event)
            return
        }
        // Let SwiftTerm finish its selection before its second pass reports
        // the release to the Ratatui application.
        performNativeSelectionPass { super.mouseUp(with: event) }
        super.mouseUp(with: event)
    }

    override func copy(_ sender: Any) {
        // Shift-drag belongs to SwiftTerm and should keep its normal copy path.
        // Ordinary pointer gestures belong to the TUI while mouse reporting is
        // active, so its synchronized semantic selection is the only source of
        // selected text in that case.
        if !selection.getSelectedText().isEmpty || applicationSelectedText == nil {
            super.copy(sender)
            return
        }

        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(applicationSelectedText ?? "", forType: .string)
    }

    private func shouldMirrorApplicationSelection(_ event: NSEvent) -> Bool {
        guard mirrorsApplicationMouseSelection, allowMouseReporting else { return false }
        if event.modifierFlags.contains(.shift), !terminal.mouseShiftCapture {
            return false
        }
        if case .off = terminal.mouseMode { return false }
        return true
    }

    private func performNativeSelectionPass(_ body: () -> Void) {
        let reportingWasEnabled = allowMouseReporting
        allowMouseReporting = false
        body()
        allowMouseReporting = reportingWasEnabled
    }
}

/// The single engine boundary in the kitchen sink. The product Host can swap
/// this implementation for GhosttyKit without changing any session logic.
@MainActor
final class TerminalEngineController: NSObject {
    let view: KitchenSinkTerminalView
    var onTermination: ((Int32?) -> Void)?
    private(set) var isRunning = false

    override init() {
        view = KitchenSinkTerminalView(
            frame: CGRect(x: 0, y: 0, width: 900, height: 600),
            font: NSFont.monospacedSystemFont(ofSize: 12.5, weight: .regular),
            options: TerminalOptions(termName: "xterm-256color", scrollback: 20_000)
        )
        super.init()
        view.allowMouseReporting = true
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
    let autoFocus: Bool
    let applicationSelectedText: String?
    let mirrorsApplicationMouseSelection: Bool

    final class Coordinator {
        var autoFocus: Bool

        init(autoFocus: Bool) {
            self.autoFocus = autoFocus
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(autoFocus: autoFocus)
    }

    func makeNSView(context _: Context) -> KitchenSinkTerminalView {
        engine.view.wantsInitialFocus = autoFocus
        engine.view.applicationSelectedText = applicationSelectedText
        engine.view.mirrorsApplicationMouseSelection = mirrorsApplicationMouseSelection
        return engine.view
    }

    func updateNSView(_ view: KitchenSinkTerminalView, context: Context) {
        view.applicationSelectedText = applicationSelectedText
        view.mirrorsApplicationMouseSelection = mirrorsApplicationMouseSelection
        let becameAutoFocused = autoFocus && !context.coordinator.autoFocus
        context.coordinator.autoFocus = autoFocus
        guard becameAutoFocused, view.window?.firstResponder !== view else { return }
        DispatchQueue.main.async { [weak view] in
            guard let view, view.window?.isKeyWindow == true else { return }
            view.window?.makeFirstResponder(view)
        }
    }
}
