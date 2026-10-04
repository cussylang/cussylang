import Foundation

struct ImportedSource: Sendable {
    let name: String
    let text: String
}

enum SourceImportError: LocalizedError, Equatable {
    case wrongFileType
    case notAFile
    case tooLarge
    case invalidUTF8

    var errorDescription: String? {
        switch self {
        case .wrongFileType:
            return "Choose a file ending in .cussy."
        case .notAFile:
            return "Choose a regular .cussy file."
        case .tooLarge:
            return "This file is larger than 1 MiB. Choose a smaller program."
        case .invalidUTF8:
            return "This file is not valid UTF-8 text. Save it as UTF-8 and try again."
        }
    }
}

enum SourceImporter {
    static func load(_ url: URL) throws -> ImportedSource {
        guard url.isFileURL else { throw SourceImportError.notAFile }
        guard url.pathExtension.lowercased() == "cussy" else {
            throw SourceImportError.wrongFileType
        }
        let scopedAccess = url.startAccessingSecurityScopedResource()
        defer {
            if scopedAccess { url.stopAccessingSecurityScopedResource() }
        }
        let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        guard values.isRegularFile != false else { throw SourceImportError.notAFile }
        let limit = RuntimeBridge.maximumSourceBytes
        if let size = values.fileSize, size > limit { throw SourceImportError.tooLarge }

        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var data = Data()
        // Metadata can be stale: the read itself is capped at one byte above the
        // limit, and partial reads are continued until EOF or the limit.
        while data.count <= limit {
            let amount = min(65_536, limit + 1 - data.count)
            guard let chunk = try handle.read(upToCount: amount), !chunk.isEmpty else { break }
            data.append(chunk)
        }
        guard data.count <= limit else { throw SourceImportError.tooLarge }
        guard let text = String(data: data, encoding: .utf8) else {
            throw SourceImportError.invalidUTF8
        }
        return ImportedSource(name: url.lastPathComponent, text: text)
    }
}
