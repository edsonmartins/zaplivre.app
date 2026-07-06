//
//  CameraManager.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import AVFoundation
import UIKit

/// CameraManager - Manages camera capture for video calls using AVFoundation
class CameraManager: NSObject, ObservableObject {
    
    // MARK: - Properties
    
    private let captureSession = AVCaptureSession()
    private var videoOutput: AVCaptureVideoDataOutput?
    private var currentCamera: AVCaptureDevice?
    private var previewLayer: AVCaptureVideoPreviewLayer?
    
    @Published var cameraPosition: AVCaptureDevice.Position = .front
    @Published var isCapturing: Bool = false
    
    private let videoQueue = DispatchQueue(label: "com.zaplivre.videoQueue")
    private var onFrameCallback: ((CMSampleBuffer) -> Void)?
    private var onEncodedFrame: (([UInt8], UInt32, UInt32) -> Void)?
    private var videoEncoder: VideoEncoder?
    private let captureWidth: UInt32 = 640
    private let captureHeight: UInt32 = 480
    
    // MARK: - Initialization
    
    override init() {
        super.init()
    }
    
    // MARK: - Camera Control
    
    /// Start camera capture
    /// - Parameter onFrame: Callback for each captured frame
    func startCapture(onFrame: @escaping (CMSampleBuffer) -> Void) {
        onFrameCallback = onFrame
        
        requestCameraPermission { [weak self] granted in
            guard granted else {
                print("❌ Camera permission denied")
                return
            }
            
            self?.setupCaptureSession()
        }
    }

    /// Start camera capture with H.264 encoding
    /// - Parameter onEncoded: Callback for each encoded frame (H.264 Annex B)
    func startCaptureEncoded(onEncoded: @escaping ([UInt8], UInt32, UInt32) -> Void) {
        onEncodedFrame = onEncoded
        if videoEncoder == nil {
            videoEncoder = VideoEncoder(width: Int(captureWidth), height: Int(captureHeight)) { [weak self] frame, _ in
                guard let self = self else { return }
                self.onEncodedFrame?(frame, self.captureWidth, self.captureHeight)
            }
        }
        videoEncoder?.start()
        startCapture { _ in }
    }
    
    /// Stop camera capture
    func stopCapture() {
        if captureSession.isRunning {
            captureSession.stopRunning()
        }
        videoEncoder?.stop()
        videoEncoder = nil
        isCapturing = false
        print("🛑 Camera capture stopped")
    }
    
    /// Switch camera (front ↔ back)
    func switchCamera() {
        cameraPosition = (cameraPosition == .front) ? .back : .front
        
        // Reconfigure session with new camera
        captureSession.beginConfiguration()
        
        // Remove old inputs
        captureSession.inputs.forEach { captureSession.removeInput($0) }
        
        // Add new camera input
        guard let newCamera = AVCaptureDevice.default(
            .builtInWideAngleCamera,
            for: .video,
            position: cameraPosition
        ) else {
            captureSession.commitConfiguration()
            return
        }
        
        do {
            let input = try AVCaptureDeviceInput(device: newCamera)
            if captureSession.canAddInput(input) {
                captureSession.addInput(input)
                currentCamera = newCamera
            }
        } catch {
            print("❌ Switch camera failed: \(error)")
        }
        
        captureSession.commitConfiguration()
        
        print("📷 Camera switched to \(cameraPosition == .front ? "FRONT" : "BACK")")
    }
    
    // MARK: - Preview Layer
    
    /// Get preview layer for displaying camera feed
    func getPreviewLayer() -> AVCaptureVideoPreviewLayer {
        if let existingLayer = previewLayer {
            return existingLayer
        }
        
        let layer = AVCaptureVideoPreviewLayer(session: captureSession)
        layer.videoGravity = .resizeAspectFill
        previewLayer = layer
        return layer
    }
    
    // MARK: - Private Methods
    
    private func setupCaptureSession() {
        captureSession.beginConfiguration()
        
        // Set preset (resolution)
        captureSession.sessionPreset = .vga640x480
        
        // Camera input
        guard let camera = AVCaptureDevice.default(
            .builtInWideAngleCamera,
            for: .video,
            position: cameraPosition
        ) else {
            print("❌ No camera available")
            captureSession.commitConfiguration()
            return
        }
        
        currentCamera = camera
        
        do {
            let input = try AVCaptureDeviceInput(device: camera)
            if captureSession.canAddInput(input) {
                captureSession.addInput(input)
            }
        } catch {
            print("❌ Camera input failed: \(error)")
            captureSession.commitConfiguration()
            return
        }
        
        // Video output
        let output = AVCaptureVideoDataOutput()
        output.setSampleBufferDelegate(self, queue: videoQueue)
        output.videoSettings = [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
        ]
        
        if captureSession.canAddOutput(output) {
            captureSession.addOutput(output)
            videoOutput = output
        }
        
        captureSession.commitConfiguration()
        
        // Start capture
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            self?.captureSession.startRunning()
            
            DispatchQueue.main.async {
                self?.isCapturing = true
                print("✅ Camera capture started")
            }
        }
    }
    
    private func requestCameraPermission(completion: @escaping (Bool) -> Void) {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            completion(true)
            
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { granted in
                DispatchQueue.main.async {
                    completion(granted)
                }
            }
            
        case .denied, .restricted:
            completion(false)
            
        @unknown default:
            completion(false)
        }
    }
    
    // MARK: - Cleanup
    
    deinit {
        stopCapture()
    }
}

// MARK: - AVCaptureVideoDataOutputSampleBufferDelegate

extension CameraManager: AVCaptureVideoDataOutputSampleBufferDelegate {
    
    func captureOutput(
        _ output: AVCaptureOutput,
        didOutput sampleBuffer: CMSampleBuffer,
        from connection: AVCaptureConnection
    ) {
        // Send frame to callback
        onFrameCallback?(sampleBuffer)

        if let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) {
            let pts = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
            videoEncoder?.encode(pixelBuffer: pixelBuffer, pts: pts)
        }
    }
    
    func captureOutput(
        _ output: AVCaptureOutput,
        didDrop sampleBuffer: CMSampleBuffer,
        from connection: AVCaptureConnection
    ) {
        // Frame dropped - can be used for statistics
        print("⚠️ Video frame dropped")
    }
}
