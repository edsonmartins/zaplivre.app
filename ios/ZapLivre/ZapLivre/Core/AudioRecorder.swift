//
//  AudioRecorder.swift
//  ZapLivre
//
//  Audio recorder using AVAudioRecorder
//

import Foundation
import AVFoundation

/// Audio recording state
enum RecordingState {
    case idle
    case recording(URL)
    case error(String)
}

/// Audio recorder class
class AudioRecorder: NSObject, ObservableObject {
    @Published var recordingState: RecordingState = .idle
    @Published var recordingDuration: TimeInterval = 0

    private var audioRecorder: AVAudioRecorder?
    private var timer: Timer?
    private var outputURL: URL?

    static let maxDuration: TimeInterval = 60.0 // 60 seconds max

    /// Start recording audio
    func startRecording() -> Result<URL, Error> {
        // Request microphone permission
        AVAudioSession.sharedInstance().requestRecordPermission { [weak self] granted in
            guard granted else {
                self?.recordingState = .error("Microphone permission denied")
                return
            }
        }

        do {
            // Configure audio session
            let audioSession = AVAudioSession.sharedInstance()
            try audioSession.setCategory(.record, mode: .default)
            try audioSession.setActive(true)

            // Create output file URL
            let documentsPath = FileManager.default.temporaryDirectory
            let fileName = "voice_message_\(Date().timeIntervalSince1970).m4a"
            let fileURL = documentsPath.appendingPathComponent(fileName)
            outputURL = fileURL

            // Set up audio recorder settings (AAC format)
            let settings: [String: Any] = [
                AVFormatIDKey: kAudioFormatMPEG4AAC,
                AVSampleRateKey: 44100.0,
                AVNumberOfChannelsKey: 1,
                AVEncoderAudioQualityKey: AVAudioQuality.high.rawValue
            ]

            // Create and start audio recorder
            audioRecorder = try AVAudioRecorder(url: fileURL, settings: settings)
            audioRecorder?.delegate = self
            audioRecorder?.record()

            recordingState = .recording(fileURL)
            recordingDuration = 0

            // Start timer
            startTimer()

            return .success(fileURL)
        } catch {
            recordingState = .error(error.localizedDescription)
            return .failure(error)
        }
    }

    /// Stop recording and return the audio file URL
    func stopRecording() -> URL? {
        audioRecorder?.stop()
        stopTimer()

        do {
            try AVAudioSession.sharedInstance().setActive(false)
        } catch {
            print("Failed to deactivate audio session: \(error)")
        }

        let url = outputURL
        outputURL = nil
        recordingState = .idle

        return url
    }

    /// Cancel recording and delete the file
    func cancelRecording() {
        audioRecorder?.stop()
        stopTimer()

        if let url = outputURL {
            try? FileManager.default.removeItem(at: url)
        }

        outputURL = nil
        recordingState = .idle
        recordingDuration = 0
    }

    /// Start the duration timer
    private func startTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            guard let self = self else { return }

            self.recordingDuration += 0.1

            // Auto-stop when max duration reached
            if self.recordingDuration >= Self.maxDuration {
                self.stopRecording()
            }
        }
    }

    /// Stop the timer
    private func stopTimer() {
        timer?.invalidate()
        timer = nil
    }

    /// Format duration for display (MM:SS)
    func formatDuration(_ duration: TimeInterval) -> String {
        let minutes = Int(duration) / 60
        let seconds = Int(duration) % 60
        return String(format: "%02d:%02d", minutes, seconds)
    }
}

// MARK: - AVAudioRecorderDelegate

extension AudioRecorder: AVAudioRecorderDelegate {
    func audioRecorderDidFinishRecording(_ recorder: AVAudioRecorder, successfully flag: Bool) {
        if !flag {
            recordingState = .error("Recording failed")
        }
    }

    func audioRecorderEncodeErrorDidOccur(_ recorder: AVAudioRecorder, error: Error?) {
        if let error = error {
            recordingState = .error(error.localizedDescription)
        }
    }
}
