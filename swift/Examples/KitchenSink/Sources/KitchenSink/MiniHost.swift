import Foundation
import SwiftUI
import UnpeelAppKitUI

enum DemoKind: String, CaseIterable, Identifiable, Sendable {
    case todo
    case markdown
    case media
    case surface
    case canvas

    var id: String { rawValue }

    var title: String {
        switch self {
        case .todo: "Todo"
        case .markdown: "Markdown"
        case .media: "Media"
        case .surface: "Surface Planets"
        case .canvas: "Canvas + Controls"
        }
    }

    var systemImage: String {
        switch self {
        case .todo: "checklist"
        case .markdown: "doc.richtext"
        case .media: "photo"
        case .surface: "globe.americas.fill"
        case .canvas: "rectangle.on.rectangle.angled"
        }
    }

    var usesSurface: Bool {
        self == .surface || self == .canvas
    }
}

enum PaneMode: String, CaseIterable, Identifiable {
    case terminal
    case native
    case web
    case split

    var id: String { rawValue }

    var rendererState: UIRendererState {
        switch self {
        case .terminal: .terminal
        case .native, .web: .component
        case .split: UIRendererState(rendererVisible: true, terminalVisible: true)
        }
    }
}

enum ChildProcessState: Equatable {
    case starting
    case running
    case stopping
    case exited(Int32?)

    var label: String {
        switch self {
        case .starting: "Starting"
        case .running: "Running"
        case .stopping: "Stopping"
        case let .exited(code): code.map { "Exited (\($0))" } ?? "Exited"
        }
    }

    var isRunning: Bool {
        self == .running || self == .starting
    }
}

struct BuiltExample: Sendable {
    let kind: DemoKind
    let executable: String
    let environment: [String: String]
}

@MainActor
final class MiniHost: ObservableObject {
    @Published private(set) var sessions: [HostedAppSession] = []
    @Published var selectedSessionID: String? {
        didSet { updatePresentedSessions() }
    }
    @Published private(set) var buildMessage = "Building Rust examples…"
    @Published private(set) var buildError: String?

    init() {
        Task { await prepare() }
    }

    var selectedSession: HostedAppSession? {
        sessions.first { $0.id == selectedSessionID }
    }

    private func prepare() async {
        do {
            let repository = try Self.locateRepository()
            let examples = try await Task.detached(priority: .userInitiated) {
                try Self.buildExamples(repository: repository)
            }.value
            var prepared: [HostedAppSession] = []
            do {
                for example in examples {
                    prepared.append(try HostedAppSession(
                        kind: example.kind,
                        executable: example.executable,
                        extraEnvironment: example.environment
                    ))
                }
            } catch {
                prepared.forEach { $0.shutdown() }
                throw error
            }
            sessions = prepared
            let requestedDemo = ProcessInfo.processInfo.environment[
                "UNPEEL_KITCHEN_SINK_SESSION"
            ]
            selectedSessionID = sessions.first(where: {
                $0.kind.rawValue == requestedDemo
            })?.id ?? sessions.first?.id
            buildMessage = "Ready"
        } catch {
            buildError = error.localizedDescription
            buildMessage = "Build failed"
            let diagnostic = "KitchenSink mini-host: \(error.localizedDescription)\n"
            try? FileHandle.standardError.write(contentsOf: Data(diagnostic.utf8))
        }
    }

    nonisolated private static func locateRepository() throws -> String {
        let environment = ProcessInfo.processInfo.environment
        var candidates: [URL] = []
        if let override = environment["UNPEEL_APP_KIT_ROOT"], !override.isEmpty {
            candidates.append(URL(fileURLWithPath: override, isDirectory: true))
        }
        candidates.append(URL(fileURLWithPath: FileManager.default.currentDirectoryPath))
        candidates.append(URL(fileURLWithPath: #filePath).deletingLastPathComponent())

        for candidate in candidates {
            var directory = candidate.standardizedFileURL
            for _ in 0..<10 {
                let manifest = directory.appendingPathComponent("Cargo.toml")
                if let contents = try? String(contentsOf: manifest, encoding: .utf8),
                   contents.contains("name = \"unpeel-app-kit\"")
                {
                    return directory.path
                }
                let parent = directory.deletingLastPathComponent()
                if parent == directory { break }
                directory = parent
            }
        }
        throw MiniHostError.repositoryNotFound
    }

    nonisolated private static func buildExamples(repository: String) throws -> [BuiltExample] {
        let targetDirectory = URL(fileURLWithPath: repository, isDirectory: true)
            .appendingPathComponent("target/kitchen-sink", isDirectory: true)
        let process = Process()
        let diagnostics = Pipe()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [
            "cargo", "build", "--quiet",
            "--manifest-path", "\(repository)/Cargo.toml",
            "--target-dir", targetDirectory.path,
            "--features", "markdown-text-area,media,surface-embed",
            "--example", "todo",
            "--example", "markdown",
            "--example", "media",
            "--example", "surface_planets",
            "--example", "surface_canvas",
        ]
        var environment = ProcessInfo.processInfo.environment
        environment.removeValue(forKey: "UNPEEL_UI_SOCKET")
        environment.removeValue(forKey: "UNPEEL_UI_TOKEN")
        process.environment = environment
        process.standardOutput = diagnostics
        process.standardError = diagnostics
        try process.run()
        let output = diagnostics.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            let message = String(data: output, encoding: .utf8) ?? "cargo build failed"
            throw MiniHostError.buildFailed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }
        var examples: [(DemoKind, String, [String: String])] = [
            (.todo, "todo", [:]),
            (.markdown, "markdown", [:]),
            (.media, "media", [:]),
        ]
        if let guest = planetGuestPath(repository: repository) {
            let environment = ["UNPEEL_SURFACE_PLANETS_WASM": guest]
            examples.append((.surface, "surface_planets", environment))
            examples.append((.canvas, "surface_canvas", environment))
        }
        return examples.map { kind, executableName, extraEnvironment in
            BuiltExample(
                kind: kind,
                executable: targetDirectory
                    .appendingPathComponent("debug/examples/\(executableName)")
                    .path,
                environment: extraEnvironment
            )
        }
    }

    nonisolated private static func planetGuestPath(repository: String) -> String? {
        let environment = ProcessInfo.processInfo.environment
        let sibling = URL(fileURLWithPath: repository, isDirectory: true)
            .deletingLastPathComponent()
            .appendingPathComponent(
                "unpeel-surface/target/wasm32-unknown-unknown/release/"
                    + "surface_planets_example.wasm"
            ).path
        return [environment["UNPEEL_SURFACE_PLANETS_WASM"], sibling]
            .compactMap { $0 }
            .first { FileManager.default.fileExists(atPath: $0) }
    }

    func shutdown() {
        sessions.forEach { $0.shutdown() }
    }

    private func updatePresentedSessions() {
        for session in sessions {
            session.setPresented(session.id == selectedSessionID)
        }
    }
}

enum MiniHostError: LocalizedError {
    case repositoryNotFound
    case buildFailed(String)

    var errorDescription: String? {
        switch self {
        case .repositoryNotFound:
            "Could not locate the unpeel-app-kit Cargo.toml. Set UNPEEL_APP_KIT_ROOT."
        case let .buildFailed(message):
            message.isEmpty ? "cargo build failed" : message
        }
    }
}

@MainActor
final class HostedAppSession: ObservableObject, Identifiable {
    let id: String
    let kind: DemoKind
    let sessionDirectory: String
    let socketPath: String
    let signingKey: String
    let terminalEngine: TerminalEngineController
    let surfaceBroker: SurfaceMiniBroker?

    @Published var paneMode: PaneMode = .split {
        didSet { updateRendererState() }
    }
    @Published private(set) var processState: ChildProcessState = .starting
    @Published private(set) var connectionState: UIUnixSessionClient.ConnectionState = .stopped
    @Published private(set) var rendererEnabled = true
    @Published private(set) var snapshot: UISnapshot?
    @Published private(set) var lastDelivery: UIProjectionDelivery?
    @Published private(set) var lastAck: UIAck?
    @Published private(set) var presence: [UIPresenceMember] = []
    @Published private(set) var appInstanceID: String?
    @Published private(set) var instanceGeneration = 0
    @Published private(set) var lastAttachResumed: Bool?
    @Published private(set) var fallbackMessage: String?

    @Published var agentGrants: Set<String> = ["view", "edit"]
    @Published private(set) var agentAttached = false
    @Published private(set) var agentConnectionState: UIUnixSessionClient.ConnectionState = .stopped
    @Published private(set) var agentSnapshot: UISnapshot?
    @Published private(set) var agentLastAck: UIAck?

    private let executable: String
    private let extraEnvironment: [String: String]
    private let issuer: UIParticipantTokenIssuer
    private let primaryClientID: String
    private let primaryRendererID: String
    private let primaryParticipant: UIParticipant
    private var primaryClient: UIUnixSessionClient?
    private var agentClient: UIUnixSessionClient?
    private var primaryClientStarted = false
    private var isPresented = false
    private var restartAfterTermination = false
    private var agentSequence = 0

    init(kind: DemoKind, executable: String, extraEnvironment: [String: String] = [:]) throws {
        self.kind = kind
        self.executable = executable
        self.extraEnvironment = extraEnvironment
        let suffix = UUID().uuidString.lowercased().prefix(8)
        id = "\(kind.rawValue)-\(suffix)"
        sessionDirectory = "/tmp/upkit-\(suffix)-\(kind.rawValue)"
        socketPath = "\(sessionDirectory)/ui.sock"
        signingKey = Data((0..<48).map { _ in UInt8.random(in: .min ... .max) })
            .base64EncodedString()
        try FileManager.default.createDirectory(
            atPath: sessionDirectory,
            withIntermediateDirectories: true,
            attributes: [.posixPermissions: 0o700]
        )
        surfaceBroker = kind.usesSurface
            ? try? SurfaceMiniBroker(sessionDirectory: sessionDirectory)
            : nil
        issuer = try UIParticipantTokenIssuer(signingKey: signingKey, appSessionID: id)
        primaryClientID = "kitchen-human-\(suffix)"
        primaryRendererID = "kitchen-native-\(suffix)"
        primaryParticipant = UIParticipant(
            id: "local-human",
            displayName: "Kitchen Sink",
            color: "#35c2b4",
            grants: ["view", "interact", "edit", "command", "admin"]
        )
        terminalEngine = TerminalEngineController()
        configurePrimaryClient()
        terminalEngine.onTermination = { [weak self] exitCode in
            self?.processDidTerminate(exitCode: exitCode)
        }
        startProcess()
    }

    var title: String { kind.title }

    var connectionLabel: String {
        Self.connectionLabel(connectionState)
    }

    var agentConnectionLabel: String {
        Self.connectionLabel(agentConnectionState)
    }

    var deliveryLabel: String {
        guard let lastDelivery else { return "No projection yet" }
        return switch lastDelivery {
        case let .snapshot(revision):
            "Snapshot r\(revision)"
        case let .delta(base, revision, operationCount):
            "Delta r\(base)→r\(revision) · \(operationCount) op\(operationCount == 1 ? "" : "s")"
        }
    }

    var agentProjectionLabel: String {
        guard let agentSnapshot else { return "No agent projection" }
        return switch agentSnapshot.root.component {
        case let .page(page): page.title
        case let .markdownEditor(editor): editor.title ?? "Markdown"
        case let .media(media): media.alt
        case let .surface(surface): "Surface: \(surface.reference.streamID)"
        case let .canvasPage(page): page.title
        case let .unsupported(kind): "Unsupported: \(kind)"
        }
    }

    func killProcess() {
        guard terminalEngine.isRunning else { return }
        restartAfterTermination = false
        processState = .stopping
        terminalEngine.terminate()
    }

    func shutdown() {
        restartAfterTermination = false
        agentClient?.stop()
        primaryClient?.stop()
        terminalEngine.terminate()
        surfaceBroker?.stop()
    }

    func restartProcess() {
        restartAfterTermination = true
        if terminalEngine.isRunning {
            processState = .stopping
            terminalEngine.terminate()
        } else {
            restartAfterTermination = false
            startProcess()
        }
    }

    func disconnectRenderer() {
        rendererEnabled = false
        primaryClient?.stop()
    }

    func reconnectRenderer() {
        rendererEnabled = true
        primaryClientStarted = true
        primaryClient?.start(rendererState: effectiveRendererState)
    }

    func setPresented(_ presented: Bool) {
        guard isPresented != presented else { return }
        isPresented = presented
        updateRendererState()
    }

    func attachAgent() {
        agentClient?.stop()
        agentSequence += 1
        let clientID = "kitchen-agent-\(id)-\(agentSequence)"
        let rendererID = "agent-renderer-\(id)-\(agentSequence)"
        let participant = UIParticipant(
            id: "agent-\(id)",
            kind: .agent,
            sourceSessionID: "neighboring-agent-session",
            displayName: "Kitchen Agent",
            color: "#a879ff",
            grants: agentGrants.sorted()
        )
        let issuer = issuer
        let configuration = UIUnixSessionClient.Configuration(
            socketPath: socketPath,
            participantTokenProvider: {
                try issuer.issue(
                    participant: participant,
                    clientID: clientID,
                    rendererID: rendererID,
                    viewID: "main",
                    tokenID: UUID().uuidString.lowercased(),
                    validFor: 300
                )
            },
            clientID: clientID,
            renderer: UIRendererMetadata(id: rendererID, kind: "agent"),
            viewID: "main",
            supportedComponentCapabilities: componentCapabilities
        )
        let client = UIUnixSessionClient(
            configuration: configuration,
            onMessage: { [weak self] message in
                Task { @MainActor [weak self] in self?.handleAgent(message) }
            },
            onState: { [weak self] state in
                Task { @MainActor [weak self] in
                    self?.agentConnectionState = state
                }
            }
        )
        agentClient = client
        agentAttached = true
        agentSnapshot = nil
        agentLastAck = nil
        client.start(rendererState: .hidden)
    }

    func detachAgent() {
        agentClient?.stop()
        agentClient = nil
        agentAttached = false
        agentSnapshot = nil
        agentConnectionState = .stopped
    }

    func exerciseAgent() {
        guard let snapshot = agentSnapshot else { return }
        let action: UIAction?
        switch snapshot.root.component {
        case let .canvasPage(page):
            guard let button = page.controls.compactMap({ control -> UIButtonSpec? in
                guard case let .button(button) = control else { return nil }
                return button
            }).first else { return }
            action = UIAction(
                nodeID: button.id,
                action: button.action,
                kind: .activate
            )
        case let .page(page):
            guard case let .list(list) = page.body,
                  let item = list.items.first,
                  let toggle = [item.leading, item.trailing, item.accessory]
                    .compactMap({ slot -> UIToggleSpec? in
                        guard case let .toggle(toggle) = slot else { return nil }
                        return toggle
                    })
                    .first
            else { return }
            action = UIAction(
                nodeID: toggle.id,
                action: toggle.setValue,
                kind: .change,
                value: .bool(!toggle.value)
            )
        case let .markdownEditor(editor):
            guard let replaceRange = editor.actions.replaceRange else { return }
            let lines = editor.text.split(
                separator: "\n",
                omittingEmptySubsequences: false
            )
            let position = UITextPosition(
                line: max(0, lines.count - 1),
                utf16Column: lines.last.map { String($0).utf16.count } ?? 0
            )
            action = UIAction(
                nodeID: snapshot.root.id,
                action: replaceRange,
                kind: .change,
                value: .textEdit(UITextEdit(
                    range: UITextRange(start: position, end: position),
                    text: "\n- Edited by Kitchen Agent"
                ))
            )
        case let .media(media):
            guard let activate = media.activate else { return }
            action = UIAction(
                nodeID: snapshot.root.id,
                action: activate,
                kind: .activate
            )
        case .surface:
            action = nil
        case .unsupported:
            action = nil
        }
        if let action { agentClient?.send(action) }
    }

    func sendPrimary(_ action: UIAction) {
        primaryClient?.send(action)
    }

    private func configurePrimaryClient() {
        let issuer = issuer
        let participant = primaryParticipant
        let clientID = primaryClientID
        let rendererID = primaryRendererID
        let configuration = UIUnixSessionClient.Configuration(
            socketPath: socketPath,
            participantTokenProvider: {
                try issuer.issue(
                    participant: participant,
                    clientID: clientID,
                    rendererID: rendererID,
                    viewID: "main",
                    tokenID: UUID().uuidString.lowercased(),
                    validFor: 300
                )
            },
            clientID: clientID,
            renderer: UIRendererMetadata(id: rendererID, kind: "swiftUI"),
            viewID: "main",
            supportedComponentCapabilities: componentCapabilities
        )
        primaryClient = UIUnixSessionClient(
            configuration: configuration,
            onMessage: { [weak self] message in
                Task { @MainActor [weak self] in self?.handlePrimary(message) }
            },
            onProjectionDelivery: { [weak self] delivery in
                Task { @MainActor [weak self] in self?.lastDelivery = delivery }
            },
            onState: { [weak self] state in
                Task { @MainActor [weak self] in self?.handlePrimaryState(state) }
            },
            onTerminalFallback: { [weak self] kind in
                Task { @MainActor [weak self] in
                    self?.fallbackMessage = "Unknown component \(kind); using terminal"
                }
            }
        )
    }

    private func startProcess() {
        try? FileManager.default.removeItem(atPath: socketPath)
        // TerminalSurfaceOptions.envVars are additions to libghostty's normal
        // inherited environment. Keep this list session-scoped: copying the
        // host's entire environment can duplicate variables Ghostty already
        // supplied and needlessly forwards host-only credentials.
        var environment: [String: String] = [:]
        environment["UNPEEL_UI_SOCKET"] = socketPath
        environment["UNPEEL_UI_TOKEN"] = signingKey
        environment["UNPEEL_SESSION_ID"] = id
        environment["UNPEEL_SESSION_DIR"] = sessionDirectory
        environment["UNPEEL_KITCHEN_SINK"] = "1"
        environment["TERM"] = "xterm-256color"
        environment["COLORTERM"] = "truecolor"
        if let surfaceBroker {
            environment["UNPEEL_SURFACE_SOCKET"] = surfaceBroker.socketPath
            environment["UNPEEL_SURFACE_REMOTE_WIDTH"] = String(
                SurfaceMiniBroker.logicalWidth
            )
            environment["UNPEEL_SURFACE_REMOTE_HEIGHT"] = String(
                SurfaceMiniBroker.logicalHeight
            )
            // The producer publishes retained USRF scenes only. The local
            // terminal, native card, and web card each render those scenes on
            // their own GPU; the producer must not also create a wgpu/Kitty
            // projection inside its PTY.
            environment["SURFACE_TERMINAL_PROJECTION"] = "retained-only"
        }
        for (key, value) in extraEnvironment {
            environment[key] = value
        }
        processState = .starting
        terminalEngine.start(TerminalLaunch(
            executable: executable,
            currentDirectory: sessionDirectory,
            environment: environment
        ))
        processState = .running
        waitForSocketAndStartRenderer()
    }

    private func waitForSocketAndStartRenderer() {
        guard !primaryClientStarted, rendererEnabled else { return }
        Task { [weak self] in
            for _ in 0..<100 {
                guard let self, self.rendererEnabled, !self.primaryClientStarted else { return }
                if FileManager.default.fileExists(atPath: self.socketPath) {
                    self.primaryClientStarted = true
                    self.primaryClient?.start(rendererState: self.effectiveRendererState)
                    return
                }
                try? await Task.sleep(for: .milliseconds(50))
            }
        }
    }

    private func processDidTerminate(exitCode: Int32?) {
        processState = .exited(exitCode)
        if restartAfterTermination {
            restartAfterTermination = false
            Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(250))
                self?.startProcess()
            }
        }
    }

    private func handlePrimary(_ message: UIMessage) {
        switch message {
        case let .snapshot(snapshot):
            self.snapshot = snapshot
            fallbackMessage = nil
        case let .ack(ack):
            lastAck = ack
        case let .presence(presence):
            self.presence = presence.members
        case let .error(error):
            fallbackMessage = "\(error.code): \(error.message)"
        case .attach, .attached, .delta, .event, .lifecycle, .requestSnapshot:
            break
        }
    }

    private func handleAgent(_ message: UIMessage) {
        switch message {
        case let .snapshot(snapshot):
            agentSnapshot = snapshot
        case let .ack(ack):
            agentLastAck = ack
        case let .presence(presence):
            self.presence = presence.members
        case .attach, .attached, .delta, .event, .lifecycle, .requestSnapshot, .error:
            break
        }
    }

    private func handlePrimaryState(_ state: UIUnixSessionClient.ConnectionState) {
        connectionState = state
        if case let .attached(instance, resumed) = state {
            if appInstanceID != instance {
                appInstanceID = instance
                instanceGeneration += 1
            }
            lastAttachResumed = resumed
        }
    }

    private static func connectionLabel(_ state: UIUnixSessionClient.ConnectionState) -> String {
        switch state {
        case .stopped: "Stopped"
        case .connecting: "Connecting"
        case let .attached(_, resumed): resumed ? "Attached · resumed" : "Attached · snapshot"
        case .waitingToReconnect: "Waiting to reconnect"
        }
    }

    private var effectiveRendererState: UIRendererState {
        isPresented ? paneMode.rendererState : .hidden
    }

    private func updateRendererState() {
        primaryClient?.setRendererState(effectiveRendererState)
    }

    /// Surface is advertised only when this Host has both the private USRF
    /// broker and local-GPU presenter assets. Otherwise the protocol's normal
    /// whole-pane terminal fallback remains authoritative.
    private var componentCapabilities: [String] {
        if surfaceBroker != nil {
            return UnpeelUIProtocol.supportedComponentCapabilities
                + [
                    UnpeelUIProtocol.surfaceCapability,
                    UnpeelUIProtocol.canvasPageCapability,
                ]
        }
        return UnpeelUIProtocol.supportedComponentCapabilities.filter {
            $0 != UnpeelUIProtocol.surfaceCapability
        }
    }
}
