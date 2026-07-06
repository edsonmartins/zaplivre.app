//
//  VoiceRecorderViewModel.swift
//  ZapLivre
//
//  ViewModel for managing voice message recording
//

import Foundation
import Combine

/// ViewModel for voice message recording
class VoiceRecorderViewModel: ObservableObject {
    @Published var isRecording = false
    @Published var recordingDuration: TimeInterval = 0
    @Published var error: String?

    private let audioRecorder = AudioRecorder()
    private var cancellables = Set<AnyCancellable>()

    init() {
        // Observe audio recorder state
        audioRecorder.$recordingState
            .sink { [weak self] state in
                switch state {
                case .idle:
                    self?.isRecording = false
                case .recording:
                    self?.isRecording = true
                case .error(let message):
                    self?.isRecording = false
                    self?.error = message
                }
            }
            .store(in: &cancellables)

        // Observe recording duration
        audioRecorder.$recordingDuration
            .assign(to: &$recordingDuration)
    }

    /// Start recording
    func startRecording() {
        let result = audioRecorder.startRecording()
        if case .failure(let error) = result {
            self.error = error.localizedDescription
        }
    }

    /// Stop recording and return file URL
    func stopRecording() -> URL? {
        return audioRecorder.stopRecording()
    }

    /// Cancel recording
    func cancelRecording() {
        audioRecorder.cancelRecording()
    }

    /// Format duration for display
    func formatDuration(_ duration: TimeInterval) -> String {
        return audioRecorder.formatDuration(duration)
    }

    /// Clear error
    func clearError() {
        error = nil
    }
}
