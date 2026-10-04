import SwiftUI
import UIKit
import UniformTypeIdentifiers

extension UTType {
    static let cussySource = UTType(exportedAs: "org.cussylang.source", conformingTo: .plainText)
}

@MainActor
struct ContentView: View {
    @StateObject private var model = RunnerModel()
    @State private var showingImporter = false
    @State private var showingExporter = false
    @State private var exportDocument = CussySourceDocument(text: "")
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        NavigationStack {
            GeometryReader { geometry in
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        introduction
                        if horizontalSizeClass == .regular && geometry.size.width > 760 {
                            HStack(alignment: .top, spacing: 20) {
                                editor(height: max(340, geometry.size.height - 200))
                                results
                                    .frame(maxWidth: .infinity, alignment: .topLeading)
                            }
                        } else {
                            editor(height: max(260, geometry.size.height * 0.43))
                            results
                        }
                    }
                    .padding(20)
                    .frame(maxWidth: 1200, alignment: .leading)
                    .frame(maxWidth: .infinity)
                }
                .background(Color(uiColor: .systemGroupedBackground))
            }
            .navigationTitle("Cussy")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Menu {
                        Button {
                            showingImporter = true
                        } label: {
                            Label("Open .cussy file", systemImage: "doc.badge.plus")
                        }
                        Button {
                            guard let document = model.documentForExport() else { return }
                            exportDocument = document
                            showingExporter = true
                        } label: {
                            Label("Export .cussy file", systemImage: "square.and.arrow.up")
                        }
                    } label: {
                        Label("Files", systemImage: "folder")
                    }
                    .disabled(model.isBusy)
                    .accessibilityIdentifier("openFileButton")
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: model.run) {
                        Label("Run", systemImage: "play.fill")
                            .fontWeight(.semibold)
                    }
                    .disabled(model.isBusy)
                    .keyboardShortcut(.return, modifiers: .command)
                    .accessibilityIdentifier("runButton")
                }
            }
            .fileImporter(isPresented: $showingImporter, allowedContentTypes: [.cussySource]) {
                model.importFile($0)
            }
            .fileExporter(isPresented: $showingExporter, document: exportDocument,
                          contentType: .cussySource, defaultFilename: model.exportName) {
                model.exportFinished($0)
            }
            .task { await model.restoreDraftIfNeeded() }
            .onChange(of: scenePhase) { phase in
                if phase != .active { model.saveDraftNow() }
            }
            .onOpenURL { url in
                model.importFile(.success(url))
            }
        }
        .tint(Color("AccentColor"))
    }

    private var introduction: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: "curlybraces")
                .font(.system(size: 24, weight: .semibold, design: .monospaced))
                .foregroundStyle(Color("AccentColor"))
                .frame(width: 48, height: 48)
                .background(Color("AccentColor").opacity(0.10), in: RoundedRectangle(cornerRadius: 12))
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 4) {
                Text("A little code. A lot of jole.")
                    .font(.headline)
                Text("Edit the sample or open a .cussy file. Everything runs on this device.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func editor(height: CGFloat) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(model.fileName, systemImage: "doc.text")
                .font(.subheadline.weight(.semibold))
                .lineLimit(1)
                .truncationMode(.middle)
            TextEditor(text: $model.source)
                .font(.system(.body, design: .monospaced))
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled(true)
                .scrollContentBackground(.hidden)
                .frame(height: height)
                .disabled(model.isBusy)
                .accessibilityLabel("Cussy source code")
                .accessibilityIdentifier("sourceEditor")
            Text("⌘ Return to run with a keyboard")
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(model.draftStatus)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color(uiColor: .secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 16))
    }

    private var results: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 8) {
                if model.isBusy {
                    ProgressView()
                        .controlSize(.small)
                } else {
                    Image(systemName: model.succeeded == false ? "exclamationmark.circle" : "terminal")
                        .foregroundStyle(model.succeeded == false ? Color.orange : Color.secondary)
                }
                Text(model.status)
                    .font(.subheadline.weight(.semibold))
                    .accessibilityIdentifier("runStatus")
                Spacer()
                if !model.output.isEmpty {
                    Button {
                        UIPasteboard.general.string = model.output
                    } label: {
                        Label("Copy output", systemImage: "doc.on.doc")
                            .labelStyle(.iconOnly)
                    }
                    .accessibilityLabel("Copy complete output")
                }
            }
            if model.displayedOutput.isEmpty {
                Text(model.succeeded == nil ? "Your program’s output will appear here." : "No output.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            } else {
                ScrollView([.horizontal, .vertical]) {
                    Text(model.displayedOutput)
                        .font(.system(.body, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 400)
                .accessibilityIdentifier("programOutput")
            }
            if model.outputIsTruncated {
                Text("Showing the first 65,536 characters. Copy output for the full result.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if !model.diagnostic.isEmpty {
                Divider()
                Text("Details")
                    .font(.subheadline.weight(.semibold))
                Text(model.diagnostic)
                    .font(.system(.footnote, design: .monospaced))
                    .textSelection(.enabled)
                    .foregroundStyle(.primary)
                    .accessibilityIdentifier("programDiagnostic")
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, minHeight: 160, alignment: .topLeading)
        .background(Color(uiColor: .secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 16))
    }
}
