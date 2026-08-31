import Foundation
import Network

/// Reconnecting native client for an App-owned `unpeel.ui/1` Unix socket.
///
/// This class belongs in the trusted native Host or workspace broker. Never
/// instantiate it in web content or disclose its broker token to a WebView.
public final class UIUnixSessionClient: @unchecked Sendable {
    public struct Configuration: Sendable {
        public let socketPath: String
        public let authToken: String
        public let participant: UIParticipant
        public let clientID: String
        public let renderer: UIRendererMetadata
        public let viewID: String

        public init(
            socketPath: String,
            authToken: String,
            participant: UIParticipant,
            clientID: String,
            renderer: UIRendererMetadata,
            viewID: String
        ) {
            self.socketPath = socketPath
            self.authToken = authToken
            self.participant = participant
            self.clientID = clientID
            self.renderer = renderer
            self.viewID = viewID
        }
    }

    public enum ConnectionState: Equatable, Sendable {
        case stopped
        case connecting
        case attached(appInstanceID: String, resumed: Bool)
        case waitingToReconnect
    }

    private static let maximumFrameBytes = 16 * 1_024 * 1_024

    private let configuration: Configuration
    private let queue: DispatchQueue
    private let onMessage: @Sendable (UIMessage) -> Void
    private let onState: @Sendable (ConnectionState) -> Void
    private var connection: NWConnection?
    private var receiveBuffer = Data()
    private var running = false
    private var ready = false
    private var reconnectAttempt = 0
    private var rendererState = UIRendererState.terminal
    private var negotiatedProtocolVersion: Int?
    private var appInstanceID: String?
    private var latestSnapshot: UISnapshot?
    private var pendingEvents: [String: UIEvent] = [:]
    private var pendingEventOrder: [String] = []

    public init(
        configuration: Configuration,
        onMessage: @escaping @Sendable (UIMessage) -> Void,
        onState: @escaping @Sendable (ConnectionState) -> Void = { _ in }
    ) {
        self.configuration = configuration
        self.onMessage = onMessage
        self.onState = onState
        queue = DispatchQueue(label: "com.unpeel.app-kit.ui.\(configuration.clientID)")
    }

    public func start(rendererState: UIRendererState = .terminal) {
        queue.async { [weak self] in
            guard let self, !running else { return }
            running = true
            self.rendererState = rendererState
            connect()
        }
    }

    public func stop() {
        queue.async { [weak self] in
            guard let self else { return }
            running = false
            ready = false
            negotiatedProtocolVersion = nil
            connection?.stateUpdateHandler = nil
            connection?.cancel()
            connection = nil
            onState(.stopped)
        }
    }

    /// Wraps a renderer-local action in authenticated session identity.
    public func send(_ action: UIAction, eventID: String = UUID().uuidString.lowercased()) {
        queue.async { [weak self] in
            guard let self, let snapshot = latestSnapshot else { return }
            let event = UIEvent(
                snapshot: snapshot,
                participantID: configuration.participant.id,
                rendererID: configuration.renderer.id,
                eventID: eventID,
                action: action
            )
            if let existing = pendingEvents[eventID] {
                if ready {
                    sendMessage(.event(existing))
                }
                return
            }
            pendingEvents[eventID] = event
            pendingEventOrder.append(eventID)
            if ready {
                sendMessage(.event(event))
            }
        }
    }

    public func setRendererState(_ state: UIRendererState) {
        queue.async { [weak self] in
            guard let self else { return }
            rendererState = state
            guard ready, let snapshot = latestSnapshot else { return }
            sendMessage(.lifecycle(UILifecycle(
                snapshot: snapshot,
                rendererID: configuration.renderer.id,
                state: state
            )))
        }
    }

    public func requestSnapshot() {
        queue.async { [weak self] in
            guard let self, ready,
                  let appInstanceID,
                  let snapshot = latestSnapshot
            else { return }
            sendMessage(.requestSnapshot(UIRequestSnapshot(
                protocolName: UnpeelUIProtocol.name,
                protocolVersion: negotiatedProtocolVersion ?? UnpeelUIProtocol.version,
                appInstanceID: appInstanceID,
                clientID: configuration.clientID,
                rendererID: configuration.renderer.id,
                viewID: snapshot.viewID
            )))
        }
    }

    private func connect() {
        guard running else { return }
        onState(.connecting)
        ready = false
        negotiatedProtocolVersion = nil
        receiveBuffer.removeAll(keepingCapacity: true)

        let connection = NWConnection(
            to: .unix(path: configuration.socketPath),
            using: .tcp
        )
        self.connection = connection
        connection.stateUpdateHandler = { [weak self, weak connection] state in
            guard let self, let connection else { return }
            handle(state, connection: connection)
        }
        connection.start(queue: queue)
    }

    private func handle(_ state: NWConnection.State, connection: NWConnection) {
        guard self.connection === connection else { return }
        switch state {
        case .ready:
            reconnectAttempt = 0
            sendAttach()
            receiveNext()
        case .failed, .cancelled:
            ready = false
            self.connection = nil
            scheduleReconnect()
        case .setup, .preparing, .waiting:
            break
        @unknown default:
            break
        }
    }

    private func sendAttach() {
        let attach = UIAttach(
            authToken: configuration.authToken,
            participant: configuration.participant,
            clientID: configuration.clientID,
            renderer: configuration.renderer,
            viewID: configuration.viewID,
            expectedAppInstanceID: appInstanceID,
            lastSeenRevision: latestSnapshot?.revision,
            state: rendererState
        )
        sendMessage(.attach(attach))
    }

    private func receiveNext() {
        guard let connection else { return }
        connection.receive(
            minimumIncompleteLength: 1,
            maximumLength: 64 * 1_024
        ) { [weak self, weak connection] data, _, isComplete, error in
            guard let self, let connection, self.connection === connection else { return }
            if let data, !data.isEmpty {
                receiveBuffer.append(data)
                decodeAvailableFrames()
            }
            if isComplete || error != nil {
                ready = false
                connection.cancel()
                return
            }
            receiveNext()
        }
    }

    private func decodeAvailableFrames() {
        while let newline = receiveBuffer.firstIndex(of: 0x0A) {
            var frame = receiveBuffer[..<newline]
            receiveBuffer.removeSubrange(...newline)
            if frame.last == 0x0D {
                frame = frame.dropLast()
            }
            guard !frame.isEmpty, frame.count <= Self.maximumFrameBytes else {
                connection?.cancel()
                return
            }
            do {
                let message = try JSONDecoder().decode(UIMessage.self, from: Data(frame))
                handle(message)
            } catch {
                connection?.cancel()
                return
            }
        }
        if receiveBuffer.count > Self.maximumFrameBytes {
            connection?.cancel()
        }
    }

    private func handle(_ message: UIMessage) {
        if let negotiatedProtocolVersion,
           let messageVersion = message.protocolVersion,
           messageVersion != negotiatedProtocolVersion
        {
            connection?.cancel()
            return
        }
        switch message {
        case let .attached(attached):
            guard UnpeelUIProtocol.negotiate(
                    minimum: attached.minProtocolVersion,
                    maximum: attached.maxProtocolVersion
                  ) == attached.protocolVersion,
                  attached.participantID == configuration.participant.id,
                  attached.clientID == configuration.clientID,
                  attached.rendererID == configuration.renderer.id,
                  attached.viewID == configuration.viewID
            else {
                connection?.cancel()
                return
            }
            let sameInstance = appInstanceID == nil || appInstanceID == attached.appInstanceID
            if !sameInstance {
                pendingEvents.removeAll()
                pendingEventOrder.removeAll()
                latestSnapshot = nil
            }
            appInstanceID = attached.appInstanceID
            negotiatedProtocolVersion = attached.protocolVersion
            ready = true
            onState(.attached(
                appInstanceID: attached.appInstanceID,
                resumed: attached.resumed
            ))
            if attached.resumed {
                for eventID in pendingEventOrder {
                    guard let event = pendingEvents[eventID] else { continue }
                    sendMessage(.event(event))
                }
            }
        case let .snapshot(snapshot):
            guard snapshot.protocolVersion == negotiatedProtocolVersion,
                  snapshot.clientID == configuration.clientID,
                  snapshot.viewID == configuration.viewID,
                  snapshot.appInstanceID == appInstanceID
            else { return }
            latestSnapshot = snapshot
        case let .ack(ack):
            guard ack.protocolVersion == negotiatedProtocolVersion,
                  ack.clientID == configuration.clientID,
                  ack.rendererID == configuration.renderer.id
            else { return }
            if ack.status != .pending {
                pendingEvents.removeValue(forKey: ack.eventID)
                pendingEventOrder.removeAll { $0 == ack.eventID }
            }
        case .error:
            break
        case .attach, .event, .lifecycle, .requestSnapshot:
            connection?.cancel()
            return
        case let .presence(presence):
            guard presence.protocolVersion == negotiatedProtocolVersion,
                  presence.appInstanceID == appInstanceID,
                  presence.viewID == configuration.viewID
            else { return }
        }
        onMessage(message)
    }

    private func sendMessage(_ message: UIMessage) {
        guard let connection else { return }
        do {
            var data = try JSONEncoder().encode(message)
            guard data.count <= Self.maximumFrameBytes else {
                connection.cancel()
                return
            }
            data.append(0x0A)
            connection.send(content: data, completion: .contentProcessed { [weak connection] error in
                if error != nil {
                    connection?.cancel()
                }
            })
        } catch {
            connection.cancel()
        }
    }

    private func scheduleReconnect() {
        guard running else {
            onState(.stopped)
            return
        }
        reconnectAttempt = min(reconnectAttempt + 1, 8)
        let delay = min(pow(2.0, Double(reconnectAttempt - 1)) * 0.1, 5.0)
        onState(.waitingToReconnect)
        queue.asyncAfter(deadline: .now() + delay) { [weak self] in
            guard let self, running, connection == nil else { return }
            connect()
        }
    }
}
