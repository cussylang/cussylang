import Foundation

struct SavedDraft: Codable, Sendable, Equatable {
    let name: String
    let text: String
}

actor DraftStore {
    static let shared = DraftStore()
    private let directory: URL
    private var latestRevision = 0

    init(directory: URL = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("Cussy", isDirectory: true)) {
        self.directory = directory
    }

    func load() throws -> SavedDraft? {
        let url = directory.appendingPathComponent("draft.json")
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        // JSON escaping can expand a 1 MiB source by up to six times.
        let limit = 8 * 1_048_576
        var data = Data()
        while data.count <= limit {
            guard let chunk = try handle.read(upToCount: min(65_536, limit + 1 - data.count)),
                  !chunk.isEmpty else { break }
            data.append(chunk)
        }
        guard data.count <= limit else { throw RuntimeBridgeError.sourceTooLarge }
        let draft = try JSONDecoder().decode(SavedDraft.self, from: data)
        guard draft.text.utf8.count <= RuntimeBridge.maximumSourceBytes else {
            throw RuntimeBridgeError.sourceTooLarge
        }
        return draft
    }

    func save(_ draft: SavedDraft, revision: Int) throws {
        // A cancelled debounce task may already have submitted a save. Prevent
        // an older task from overwriting a newer draft if actor jobs reorder.
        guard revision >= latestRevision else { return }
        guard draft.text.utf8.count <= RuntimeBridge.maximumSourceBytes else {
            throw RuntimeBridgeError.sourceTooLarge
        }
        latestRevision = revision
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let data = try JSONEncoder().encode(draft)
        try data.write(to: directory.appendingPathComponent("draft.json"), options: .atomic)
    }
}
