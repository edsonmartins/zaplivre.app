//
//  MessageSearchView.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import SwiftUI

/// MessageSearchView - Search messages within a conversation or globally
struct MessageSearchView: View {
    let conversationId: String?  // nil = global search
    let peerName: String?
    let onMessageTap: (FfiMessageWrapper) -> Void

    @Environment(\.dismiss) var dismiss
    @State private var searchQuery = ""
    @State private var searchResults: [FfiMessageWrapper] = []
    @State private var isSearching = false
    @State private var searchTask: Task<Void, Never>?

    var body: some View {
        NavigationView {
            VStack(spacing: 0) {
                // Search bar
                SearchBarView(
                    query: $searchQuery,
                    isSearching: isSearching,
                    onClear: { searchQuery = "" }
                )
                .padding()

                // Results
                if isSearching {
                    Spacer()
                    ProgressView()
                    Spacer()
                } else if searchQuery.isEmpty {
                    // Initial state
                    Spacer()
                    VStack(spacing: 12) {
                        Image(systemName: "magnifyingglass")
                            .font(.system(size: 64))
                            .foregroundColor(.secondary.opacity(0.5))

                        Text("Digite para buscar mensagens")
                            .font(.body)
                            .foregroundColor(.secondary)

                        if conversationId == nil {
                            Text("Busca em todas as conversas")
                                .font(.caption)
                                .foregroundColor(.secondary.opacity(0.7))
                        }
                    }
                    Spacer()
                } else if searchResults.isEmpty {
                    // No results
                    Spacer()
                    VStack(spacing: 8) {
                        Text("Nenhum resultado encontrado")
                            .font(.title3)
                            .fontWeight(.medium)
                            .foregroundColor(.secondary)

                        Text("Tente outros termos de busca")
                            .font(.body)
                            .foregroundColor(.secondary.opacity(0.7))
                    }
                    Spacer()
                } else {
                    // Results list
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            // Results count
                            Text("\(searchResults.count) resultado(s) encontrado(s)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)

                            // Results
                            ForEach(searchResults) { message in
                                SearchResultRow(
                                    message: message,
                                    query: searchQuery,
                                    onTap: {
                                        onMessageTap(message)
                                        dismiss()
                                    }
                                )
                            }
                        }
                    }
                }
            }
            .navigationTitle(conversationId != nil ? "Buscar em \(peerName ?? "conversa")" : "Buscar mensagens")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancelar") {
                        dismiss()
                    }
                }
            }
        }
        .onChange(of: searchQuery) { newQuery in
            performSearch(query: newQuery)
        }
        .onDisappear {
            searchTask?.cancel()
        }
    }

    private func performSearch(query: String) {
        // Cancel previous search
        searchTask?.cancel()

        guard !query.isEmpty else {
            searchResults = []
            return
        }

        searchTask = Task {
            // Debounce
            try? await Task.sleep(nanoseconds: 300_000_000) // 300ms

            guard !Task.isCancelled else { return }

            await MainActor.run {
                isSearching = true
            }

            do {
                let results = try await ZapLivreCore.shared.searchMessages(
                    query: query,
                    limit: 100
                )

                guard !Task.isCancelled else { return }

                await MainActor.run {
                    // Filter by conversation if specified
                    if let conversationId = conversationId {
                        searchResults = results.filter { $0.conversationId == conversationId }
                    } else {
                        searchResults = results
                    }
                    isSearching = false
                }
            } catch {
                print("❌ Search error: \(error)")
                await MainActor.run {
                    isSearching = false
                }
            }
        }
    }
}

/// SearchBarView - Custom search input field
struct SearchBarView: View {
    @Binding var query: String
    let isSearching: Bool
    let onClear: () -> Void

    var body: some View {
        HStack {
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)

                TextField("Buscar mensagens...", text: $query)
                    .accessibilityIdentifier("search_input")
                    .autocorrectionDisabled()

                if isSearching {
                    ProgressView()
                        .progressViewStyle(.circular)
                        .scaleEffect(0.8)
                } else if !query.isEmpty {
                    Button(action: onClear) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(8)
            .background(Color(.systemGray6))
            .cornerRadius(10)
        }
    }
}

/// SearchResultRow - Single search result with highlighted query
struct SearchResultRow: View {
    let message: FfiMessageWrapper
    let query: String
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                // Message content with highlighting
                Text(highlightedText)
                    .font(.body)
                    .lineLimit(2)
                    .foregroundColor(.primary)
                    .frame(maxWidth: .infinity, alignment: .leading)

                // Message metadata
                HStack {
                    // Sender info (for global search)
                    Text(String(message.senderPeerId.prefix(12)) + "...")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    Spacer()

                    // Timestamp
                    Text(formatTime(message.createdAt))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Divider()
                    .padding(.top, 8)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
        }
        .buttonStyle(.plain)
    }

    private var highlightedText: AttributedString {
        let content = message.content ?? "[Mídia]"
        var attributedString = AttributedString(content)

        let lowerContent = content.lowercased()
        let lowerQuery = query.lowercased()

        var searchStartIndex = lowerContent.startIndex

        while let range = lowerContent.range(of: lowerQuery, range: searchStartIndex..<lowerContent.endIndex) {
            // Convert String.Index to AttributedString.Index
            let lowerBound = AttributedString.Index(range.lowerBound, within: attributedString)!
            let upperBound = AttributedString.Index(range.upperBound, within: attributedString)!
            let attributedRange = lowerBound..<upperBound

            // Highlight the match
            attributedString[attributedRange].backgroundColor = Color.yellow.opacity(0.5)
            attributedString[attributedRange].font = .body.bold()

            searchStartIndex = range.upperBound
        }

        return attributedString
    }

    private func formatTime(_ date: Date) -> String {
        let formatter = DateFormatter()
        let calendar = Calendar.current
        if calendar.isDateInToday(date) {
            formatter.dateFormat = "HH:mm"
        } else if calendar.isDateInYesterday(date) {
            return "Ontem"
        } else {
            formatter.dateFormat = "dd/MM/yy"
        }
        return formatter.string(from: date)
    }
}

#Preview {
    MessageSearchView(
        conversationId: nil,
        peerName: nil,
        onMessageTap: { _ in }
    )
}
