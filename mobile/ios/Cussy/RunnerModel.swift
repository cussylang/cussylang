import Foundation
import Combine

@MainActor
final class RunnerModel: ObservableObject {
    @Published var source = """
    int whitecap() {
        jole("Hello, Cussy!");
        verify 0;
    }
    """ {
        didSet { scheduleDraftSave() }
    }
    @Published private(set) var fileName = "hello.cussy"
    @Published private(set) var isBusy = true
    @Published private(set) var output = ""
    @Published private(set) var displayedOutput = ""
    @Published private(set) var outputIsTruncated = false
    @Published private(set) var diagnostic = ""
    @Published private(set) var status = "Opening draft…"
    @Published private(set) var succeeded: Bool?
    @Published private(set) var draftStatus = "Drafts stay on this device."
    private var restoredDraft = false
    private var hasStartedRestore = false
    private var draftRevision = 0
    private var saveTask: Task<Void, Never>?
    private var pendingImport: URL?

    func restoreDraftIfNeeded() async {
        guard !hasStartedRestore else { return }
        hasStartedRestore = true
        isBusy = true
        status = "Opening draft…"
        defer {
            restoredDraft = true
            finishWork()
        }
        do {
            if let draft = try await DraftStore.shared.load() {
                fileName = draft.name
                source = draft.text
                draftStatus = "Draft saved on this device."
            }
            status = "Ready"
        } catch {
            diagnostic = "Could not restore your saved draft: \(error.localizedDescription)"
            draftStatus = "Saved draft could not be opened."
            status = "Draft unavailable"
            succeeded = false
        }
    }

    func run() {
        guard !isBusy else { return }
        let program = source
        isBusy = true
        status = "Running…"
        output = ""
        displayedOutput = ""
        outputIsTruncated = false
        diagnostic = ""
        succeeded = nil
        Task {
            defer { finishWork() }
            do {
                let response = try await Task.detached(priority: .userInitiated) {
                    try RuntimeBridge.run(source: program)
                }.value
                output = response.output
                displayedOutput = String(response.output.prefix(65_536))
                outputIsTruncated = displayedOutput.utf8.count < response.output.utf8.count
                diagnostic = response.diagnostic
                succeeded = response.ok
                if response.ok, let exitCode = response.exitCode {
                    status = "Finished · exit \(exitCode)"
                } else {
                    status = response.ok ? "Finished" : "Program error"
                }
            } catch {
                diagnostic = error.localizedDescription
                succeeded = false
                status = "Could not run"
            }
        }
    }

    func importFile(_ result: Result<URL, Error>) {
        guard !isBusy else {
            if case .success(let url) = result { pendingImport = url }
            return
        }
        switch result {
        case .failure(let error):
            // Cancelling the system picker is not a program or import failure.
            if (error as NSError).code != NSUserCancelledError {
                diagnostic = error.localizedDescription
                status = "Could not import"
                succeeded = false
            }
        case .success(let url):
            isBusy = true
            status = "Opening…"
            Task {
                defer { finishWork() }
                do {
                    let imported = try await Task.detached(priority: .userInitiated) {
                        try SourceImporter.load(url)
                    }.value
                    fileName = imported.name
                    source = imported.text
                    output = ""
                    displayedOutput = ""
                    outputIsTruncated = false
                    diagnostic = ""
                    succeeded = nil
                    status = "Ready"
                } catch {
                    diagnostic = error.localizedDescription
                    succeeded = false
                    status = "Could not import"
                }
            }
        }
    }

    func documentForExport() -> CussySourceDocument? {
        guard source.utf8.count <= RuntimeBridge.maximumSourceBytes else {
            diagnostic = RuntimeBridgeError.sourceTooLarge.localizedDescription
            status = "Could not export"
            succeeded = false
            return nil
        }
        return CussySourceDocument(text: source)
    }

    var exportName: String {
        URL(fileURLWithPath: fileName).deletingPathExtension().lastPathComponent
    }

    func exportFinished(_ result: Result<URL, Error>) {
        if case .failure(let error) = result, (error as NSError).code != NSUserCancelledError {
            diagnostic = "Could not export: \(error.localizedDescription)"
            status = "Could not export"
            succeeded = false
        }
    }

    func saveDraftNow() {
        scheduleDraftSave(delay: false)
    }

    private func finishWork() {
        isBusy = false
        if let url = pendingImport {
            pendingImport = nil
            importFile(.success(url))
        }
    }

    private func scheduleDraftSave(delay: Bool = true) {
        guard restoredDraft else { return }
        saveTask?.cancel()
        guard source.utf8.count <= RuntimeBridge.maximumSourceBytes else {
            draftStatus = "Draft exceeds 1 MiB and cannot be saved."
            return
        }
        draftRevision += 1
        let revision = draftRevision
        let draft = SavedDraft(name: fileName, text: source)
        draftStatus = "Saving draft…"
        saveTask = Task {
            do {
                if delay { try await Task.sleep(nanoseconds: 350_000_000) }
                try Task.checkCancellation()
                try await DraftStore.shared.save(draft, revision: revision)
                guard revision == draftRevision else { return }
                draftStatus = "Draft saved on this device."
            } catch is CancellationError {
                // A newer edit owns the pending save.
            } catch {
                guard revision == draftRevision else { return }
                draftStatus = "Could not save draft: \(error.localizedDescription)"
            }
        }
    }
}
