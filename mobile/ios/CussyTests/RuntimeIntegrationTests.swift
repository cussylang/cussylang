import Foundation
import XCTest
@testable import Cussy

final class RuntimeIntegrationTests: XCTestCase {
    func testHelloCallsRealRuntime() throws {
        let result = try RuntimeBridge.run(source: "int whitecap(){jole(\"Hello, Cussy! 👋\");verify 0;}")
        XCTAssertTrue(result.ok)
        XCTAssertEqual(result.exitCode, 0)
        XCTAssertEqual(result.output, "Hello, Cussy! 👋\n")
        XCTAssertEqual(result.diagnostic, "")
    }

    func testEmbeddedMathModule() throws {
        let result = try RuntimeBridge.run(source: "graph math; int whitecap(){jole(sqrt(81.0),6*7);verify 0;}")
        XCTAssertTrue(result.ok, result.diagnostic)
        XCTAssertEqual(result.output, "9 42\n")
    }

    func testArrayMutation() throws {
        let result = try RuntimeBridge.run(source: "int whitecap(){int values[]=[3,5,8];values[1]+=2;jole(values[0]+values[1]+values[2]);verify 0;}")
        XCTAssertTrue(result.ok, result.diagnostic)
        XCTAssertEqual(result.output, "18\n")
    }

    func testRuntimeErrorPreservesEarlierOutput() throws {
        let result = try RuntimeBridge.run(source: "int whitecap(){jole(\"before\");int zero=0;jole(1/zero);verify 0;}")
        XCTAssertFalse(result.ok)
        XCTAssertNil(result.exitCode)
        XCTAssertEqual(result.output, "before\n")
        XCTAssertTrue(result.diagnostic.contains("AURA_OVERFLOW"), result.diagnostic)
    }

    func testSyntaxErrorIsReadable() throws {
        let result = try RuntimeBridge.run(source: "int whitecap( {")
        XCTAssertFalse(result.ok)
        XCTAssertNil(result.exitCode)
        XCTAssertFalse(result.diagnostic.isEmpty)
        XCTAssertTrue(result.diagnostic.contains("program.cussy"), result.diagnostic)
    }

    func testFuelStopsAnInfiniteLoop() throws {
        let result = try RuntimeBridge.run(source: "int whitecap(){ticker(verified){}verify 0;}", fuel: 200)
        XCTAssertFalse(result.ok)
        XCTAssertTrue(result.diagnostic.contains("LIMIT"), result.diagnostic)
    }

    func testBridgeRejectsOversizedSourceBeforeCallingRuntime() {
        XCTAssertThrowsError(try RuntimeBridge.run(source: String(repeating: " ", count: 1_048_577))) { error in
            guard case RuntimeBridgeError.sourceTooLarge = error else {
                return XCTFail("Unexpected error: \(error)")
            }
        }
    }

    func testUTF8FileImport() throws {
        try withTemporaryFile(data: Data("int whitecap(){jole(\"jole 🐱\");verify 0;}".utf8)) { url in
            let imported = try SourceImporter.load(url)
            XCTAssertEqual(imported.name, "program.cussy")
            XCTAssertEqual(imported.text, "int whitecap(){jole(\"jole 🐱\");verify 0;}")
        }
    }

    func testInvalidUTF8IsRejected() throws {
        try withTemporaryFile(data: Data([0xff, 0xfe, 0x80])) { url in
            XCTAssertThrowsError(try SourceImporter.load(url)) { error in
                XCTAssertEqual(error as? SourceImportError, .invalidUTF8)
            }
        }
    }

    func testOversizedImportIsRejected() throws {
        try withTemporaryFile(data: Data(repeating: 65, count: 1_048_577)) { url in
            XCTAssertThrowsError(try SourceImporter.load(url)) { error in
                XCTAssertEqual(error as? SourceImportError, .tooLarge)
            }
        }
    }

    func testWrongExtensionIsRejected() throws {
        try withTemporaryFile(data: Data("jole".utf8), name: "program.txt") { url in
            XCTAssertThrowsError(try SourceImporter.load(url)) { error in
                XCTAssertEqual(error as? SourceImportError, .wrongFileType)
            }
        }
    }

    private func withTemporaryFile(data: Data, name: String = "program.cussy", body: (URL) throws -> Void) throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent(name)
        try data.write(to: url)
        try body(url)
    }
}
