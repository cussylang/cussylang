import SwiftUI
import UniformTypeIdentifiers

struct CussySourceDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.cussySource] }
    var text: String

    init(text: String) {
        self.text = text
    }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents else {
            throw SourceImportError.notAFile
        }
        guard data.count <= RuntimeBridge.maximumSourceBytes else {
            throw SourceImportError.tooLarge
        }
        guard let text = String(data: data, encoding: .utf8) else {
            throw SourceImportError.invalidUTF8
        }
        self.text = text
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        guard text.utf8.count <= RuntimeBridge.maximumSourceBytes else {
            throw RuntimeBridgeError.sourceTooLarge
        }
        return FileWrapper(regularFileWithContents: Data(text.utf8))
    }
}
