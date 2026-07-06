//
//  AudioManager.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//
//  Audio I/O manager using AVAudioEngine for VoIP calls

import Foundation
import AVFoundation
import Combine

/// Manages audio input/output for VoIP calls using AVAudioEngine
class AudioManager: ObservableObject {
    // MARK: - Published Properties
    @Published var isRecording = false
    @Published var isPlaying = false
    @Published var inputVolume: Float = 0.0
    @Published var outputVolume: Float = 1.0

    // MARK: - Audio Engine
    private let audioEngine = AVAudioEngine()
    private let playerNode = AVAudioPlayerNode()

    // Audio format: 48kHz, mono, 16-bit (Opus standard)
    private let audioFormat: AVAudioFormat

    // Audio session
    private let audioSession = AVAudioSession.sharedInstance()

    // Audio buffers
    private var audioBufferQueue: [AVAudioPCMBuffer] = []
    private let bufferQueueLock = NSLock()

    // Audio callback for sending to WebRTC
    var onAudioCaptured: ((Data) -> Void)?

    // MARK: - Configuration
    private let sampleRate: Double = 48000.0  // 48kHz (Opus standard)
    private let channelCount: AVAudioChannelCount = 1  // Mono
    private let frameDuration: Double = 0.020  // 20ms frames (Opus standard)

    // MARK: - Initialization
    init() {
        // Initialize audio format
        guard let format = AVAudioFormat(
            commonFormat: .pcmFormatInt16,
            sampleRate: sampleRate,
            channels: channelCount,
            interleaved: true
        ) else {
            fatalError("Failed to create audio format")
        }
        self.audioFormat = format

        setupAudioEngine()
    }

    deinit {
        stop()
    }

    // MARK: - Audio Engine Setup
    private func setupAudioEngine() {
        // Attach player node
        audioEngine.attach(playerNode)

        // Connect player node to output
        audioEngine.connect(
            playerNode,
            to: audioEngine.mainMixerNode,
            format: audioFormat
        )

        print("✅ AudioManager initialized (48kHz, mono, 16-bit)")
    }

    // MARK: - Audio Session Configuration
    func configureAudioSession() throws {
        try audioSession.setCategory(
            .playAndRecord,
            mode: .voiceChat,
            options: [.allowBluetooth, .defaultToSpeaker]
        )

        try audioSession.setPreferredSampleRate(sampleRate)
        try audioSession.setPreferredIOBufferDuration(frameDuration)
        try audioSession.setActive(true)

        print("✅ Audio session configured for VoIP")
    }

    // MARK: - Start/Stop Audio
    func start() throws {
        // Configure audio session
        try configureAudioSession()

        // Install tap on input node to capture audio
        let inputNode = audioEngine.inputNode
        let inputFormat = inputNode.outputFormat(forBus: 0)

        // Convert to our format if needed
        let converter = AVAudioConverter(from: inputFormat, to: audioFormat)
        guard converter != nil else {
            throw AudioError.formatConversionFailed
        }

        // Install tap with 20ms buffer (960 samples at 48kHz)
        let frameLength = AVAudioFrameCount(sampleRate * frameDuration)

        inputNode.installTap(
            onBus: 0,
            bufferSize: frameLength,
            format: inputFormat
        ) { [weak self] (buffer, time) in
            self?.processInputBuffer(buffer, converter: converter!)
        }

        // Start audio engine
        try audioEngine.start()

        // Start player node
        playerNode.play()

        isRecording = true
        isPlaying = true

        print("✅ Audio engine started")
    }

    func stop() {
        audioEngine.inputNode.removeTap(onBus: 0)
        audioEngine.stop()
        playerNode.stop()

        isRecording = false
        isPlaying = false

        // Deactivate audio session
        try? audioSession.setActive(false, options: .notifyOthersOnDeactivation)

        print("🛑 Audio engine stopped")
    }

    // MARK: - Audio Processing
    private func processInputBuffer(_ buffer: AVAudioPCMBuffer, converter: AVAudioConverter) {
        // Convert to our format (48kHz, mono, 16-bit)
        guard let convertedBuffer = AVAudioPCMBuffer(
            pcmFormat: audioFormat,
            frameCapacity: AVAudioFrameCount(sampleRate * frameDuration)
        ) else {
            return
        }

        var error: NSError?
        let inputBlock: AVAudioConverterInputBlock = { _, outStatus in
            outStatus.pointee = .haveData
            return buffer
        }

        converter.convert(to: convertedBuffer, error: &error, withInputFrom: inputBlock)

        if let error = error {
            print("⚠️ Audio conversion error: \(error)")
            return
        }

        // Update input volume level
        DispatchQueue.main.async {
            self.inputVolume = self.calculateVolume(from: convertedBuffer)
        }

        // Convert to Data and send to callback
        if let audioData = convertedBuffer.toData() {
            onAudioCaptured?(audioData)
        }
    }

    // MARK: - Playback
    /// Play received audio from WebRTC
    func playAudio(_ audioData: Data) {
        guard let buffer = audioData.toPCMBuffer(format: audioFormat) else {
            print("⚠️ Failed to convert audio data to PCM buffer")
            return
        }

        // Queue buffer for playback
        bufferQueueLock.lock()
        audioBufferQueue.append(buffer)
        bufferQueueLock.unlock()

        // Schedule buffer on player node
        playerNode.scheduleBuffer(buffer) { [weak self] in
            self?.bufferQueueLock.lock()
            if !self!.audioBufferQueue.isEmpty {
                self?.audioBufferQueue.removeFirst()
            }
            self?.bufferQueueLock.unlock()
        }
    }

    // MARK: - Volume Control
    func setOutputVolume(_ volume: Float) {
        outputVolume = max(0.0, min(1.0, volume))
        playerNode.volume = outputVolume
    }

    func mute() {
        playerNode.volume = 0.0
    }

    func unmute() {
        playerNode.volume = outputVolume
    }

    // MARK: - Audio Routing
    func enableSpeaker(_ enabled: Bool) throws {
        let options: AVAudioSession.CategoryOptions = enabled
            ? [.allowBluetooth, .defaultToSpeaker]
            : [.allowBluetooth]

        try audioSession.setCategory(
            .playAndRecord,
            mode: .voiceChat,
            options: options
        )

        // Force route to speaker if enabled
        if enabled {
            try audioSession.overrideOutputAudioPort(.speaker)
        } else {
            try audioSession.overrideOutputAudioPort(.none)
        }

        print(enabled ? "🔊 Speaker enabled" : "📱 Earpiece enabled")
    }

    // MARK: - Utilities
    private func calculateVolume(from buffer: AVAudioPCMBuffer) -> Float {
        guard let channelData = buffer.floatChannelData?[0] else {
            return 0.0
        }

        let channelDataValue = channelData
        let channelDataValueArray = Array(UnsafeBufferPointer(start: channelDataValue, count: Int(buffer.frameLength)))

        let rms = sqrt(channelDataValueArray.map { $0 * $0 }.reduce(0, +) / Float(buffer.frameLength))
        let avgPower = 20 * log10(rms)
        let normalizedPower = max(0, min(1, (avgPower + 50) / 50))

        return normalizedPower
    }
}

// MARK: - Audio Errors
enum AudioError: Error {
    case formatConversionFailed
    case engineStartFailed
    case sessionConfigurationFailed
}

// MARK: - AVAudioPCMBuffer Extensions
extension AVAudioPCMBuffer {
    /// Convert PCM buffer to Data (raw bytes)
    func toData() -> Data? {
        guard let channelData = int16ChannelData else {
            return nil
        }

        let channelDataValue = channelData.pointee
        let channelDataValueArray = Array(UnsafeBufferPointer(start: channelDataValue, count: Int(frameLength)))

        return Data(bytes: channelDataValueArray, count: Int(frameLength) * MemoryLayout<Int16>.size)
    }
}

extension Data {
    /// Convert Data to AVAudioPCMBuffer
    func toPCMBuffer(format: AVAudioFormat) -> AVAudioPCMBuffer? {
        let frameLength = UInt32(count) / UInt32(MemoryLayout<Int16>.size)

        guard let buffer = AVAudioPCMBuffer(
            pcmFormat: format,
            frameCapacity: frameLength
        ) else {
            return nil
        }

        buffer.frameLength = frameLength

        guard let channelData = buffer.int16ChannelData else {
            return nil
        }

        withUnsafeBytes { (bytes: UnsafeRawBufferPointer) in
            guard let baseAddress = bytes.baseAddress else { return }
            channelData.pointee.update(from: baseAddress.assumingMemoryBound(to: Int16.self), count: Int(frameLength))
        }

        return buffer
    }
}
