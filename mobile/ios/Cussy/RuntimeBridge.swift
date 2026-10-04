import CussyRuntime
import Foundation

struct RunResponse: Decodable, Sendable {
    let ok: Bool
    let exitCode: Int64?
    let output: String
    let diagnostic: String

    private enum CodingKeys: String, CodingKey {
        case ok, output, diagnostic
        case exitCode = "exit_code"
    }
}

enum RuntimeBridgeError: LocalizedError {
    case sourceTooLarge
    case unavailableResponse
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .sourceTooLarge:
            return "This program is larger than 1 MiB. Open or write a smaller program."
        case .unavailableResponse:
            return "Cussy could not prepare a result. Try a smaller program."
        case .invalidResponse:
            return "The Cussy runtime returned an unreadable result."
        }
    }
}

enum RuntimeBridge {
    static let maximumSourceBytes = 1_048_576
    static let defaultFuel: UInt64 = 5_000_000

    /// Synchronous C ABI boundary. Call from a worker task, never the UI thread.
    static func run(source: String, fuel: UInt64 = defaultFuel) throws -> RunResponse {
        guard source.utf8.count <= maximumSourceBytes else {
            throw RuntimeBridgeError.sourceTooLarge
        }
        let bytes = Array(source.utf8)
        return try bytes.withUnsafeBufferPointer { buffer in
            guard let pointer = cussy_mobile_run(buffer.baseAddress, buffer.count, fuel) else {
                throw RuntimeBridgeError.unavailableResponse
            }
            defer { cussy_mobile_free(pointer) }
            guard let json = String(validatingUTF8: pointer), let data = json.data(using: .utf8) else {
                throw RuntimeBridgeError.invalidResponse
            }
            do {
                return try JSONDecoder().decode(RunResponse.self, from: data)
            } catch {
                throw RuntimeBridgeError.invalidResponse
            }
        }
    }
}
