//
//  QRScannerViewController.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//
//  QR Code scanner using AVFoundation

import UIKit
import AVFoundation

protocol QRScannerDelegate: AnyObject {
    func qrScanner(_ scanner: QRScannerViewController, didScanCode code: String)
    func qrScannerDidCancel(_ scanner: QRScannerViewController)
}

class QRScannerViewController: UIViewController {
    weak var delegate: QRScannerDelegate?

    // MARK: - Camera
    private var captureSession: AVCaptureSession?
    private var previewLayer: AVCaptureVideoPreviewLayer?

    // MARK: - UI Elements
    private let scannerOverlay = UIView()
    private let scannerFrame = UIView()
    private let instructionLabel = UILabel()
    private let cancelButton = UIButton(type: .system)

    // MARK: - Lifecycle
    override func viewDidLoad() {
        super.viewDidLoad()

        view.backgroundColor = .black
        setupCamera()
        setupUI()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)

        if captureSession?.isRunning == false {
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                self?.captureSession?.startRunning()
            }
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)

        if captureSession?.isRunning == true {
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                self?.captureSession?.stopRunning()
            }
        }
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        previewLayer?.frame = view.layer.bounds
    }

    // MARK: - Camera Setup
    private func setupCamera() {
        captureSession = AVCaptureSession()

        guard let captureSession = captureSession else { return }
        guard let videoCaptureDevice = AVCaptureDevice.default(for: .video) else {
            showError("Câmera não disponível")
            return
        }

        let videoInput: AVCaptureDeviceInput

        do {
            videoInput = try AVCaptureDeviceInput(device: videoCaptureDevice)
        } catch {
            showError("Erro ao acessar a câmera")
            return
        }

        if captureSession.canAddInput(videoInput) {
            captureSession.addInput(videoInput)
        } else {
            showError("Não foi possível configurar a câmera")
            return
        }

        let metadataOutput = AVCaptureMetadataOutput()

        if captureSession.canAddOutput(metadataOutput) {
            captureSession.addOutput(metadataOutput)

            metadataOutput.setMetadataObjectsDelegate(self, queue: DispatchQueue.main)
            metadataOutput.metadataObjectTypes = [.qr]
        } else {
            showError("Não foi possível configurar o scanner")
            return
        }

        previewLayer = AVCaptureVideoPreviewLayer(session: captureSession)
        previewLayer?.frame = view.layer.bounds
        previewLayer?.videoGravity = .resizeAspectFill

        if let previewLayer = previewLayer {
            view.layer.addSublayer(previewLayer)
        }
    }

    // MARK: - UI Setup
    private func setupUI() {
        // Semi-transparent overlay
        scannerOverlay.backgroundColor = UIColor.black.withAlphaComponent(0.5)
        scannerOverlay.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(scannerOverlay)

        // Scanner frame (transparent square)
        let frameSize: CGFloat = 250
        scannerFrame.backgroundColor = .clear
        scannerFrame.layer.borderColor = UIColor.white.cgColor
        scannerFrame.layer.borderWidth = 2
        scannerFrame.layer.cornerRadius = 12
        scannerFrame.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(scannerFrame)

        // Instruction label
        instructionLabel.text = "Aponte para o QR Code"
        instructionLabel.textColor = .white
        instructionLabel.font = .systemFont(ofSize: 18, weight: .medium)
        instructionLabel.textAlignment = .center
        instructionLabel.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(instructionLabel)

        // Cancel button
        cancelButton.setTitle("Cancelar", for: .normal)
        cancelButton.setTitleColor(.white, for: .normal)
        cancelButton.backgroundColor = UIColor.white.withAlphaComponent(0.2)
        cancelButton.layer.cornerRadius = 8
        cancelButton.titleLabel?.font = .systemFont(ofSize: 17, weight: .medium)
        cancelButton.translatesAutoresizingMaskIntoConstraints = false
        cancelButton.addTarget(self, action: #selector(cancelTapped), for: .touchUpInside)
        view.addSubview(cancelButton)

        // Constraints
        NSLayoutConstraint.activate([
            scannerOverlay.topAnchor.constraint(equalTo: view.topAnchor),
            scannerOverlay.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            scannerOverlay.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            scannerOverlay.bottomAnchor.constraint(equalTo: view.bottomAnchor),

            scannerFrame.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            scannerFrame.centerYAnchor.constraint(equalTo: view.centerYAnchor, constant: -50),
            scannerFrame.widthAnchor.constraint(equalToConstant: frameSize),
            scannerFrame.heightAnchor.constraint(equalToConstant: frameSize),

            instructionLabel.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            instructionLabel.bottomAnchor.constraint(equalTo: scannerFrame.topAnchor, constant: -24),
            instructionLabel.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 32),
            instructionLabel.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -32),

            cancelButton.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            cancelButton.topAnchor.constraint(equalTo: scannerFrame.bottomAnchor, constant: 40),
            cancelButton.widthAnchor.constraint(equalToConstant: 120),
            cancelButton.heightAnchor.constraint(equalToConstant: 44),
        ])

        // Create cutout for scanner frame
        addScannerCutout()
    }

    private func addScannerCutout() {
        let maskLayer = CAShapeLayer()
        let path = UIBezierPath(rect: view.bounds)

        // Scanner frame rect
        let frameSize: CGFloat = 250
        let frameX = (view.bounds.width - frameSize) / 2
        let frameY = (view.bounds.height - frameSize) / 2 - 50
        let scannerRect = CGRect(x: frameX, y: frameY, width: frameSize, height: frameSize)

        // Create cutout
        let cutoutPath = UIBezierPath(roundedRect: scannerRect, cornerRadius: 12)
        path.append(cutoutPath)
        path.usesEvenOddFillRule = true

        maskLayer.path = path.cgPath
        maskLayer.fillRule = .evenOdd

        scannerOverlay.layer.mask = maskLayer
    }

    // MARK: - Actions
    @objc private func cancelTapped() {
        delegate?.qrScannerDidCancel(self)
    }

    private func showError(_ message: String) {
        let alert = UIAlertController(title: "Erro", message: message, preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "OK", style: .default) { [weak self] _ in
            self?.delegate?.qrScannerDidCancel(self!)
        })
        present(alert, animated: true)
    }

    // MARK: - Scan Success
    private func found(code: String) {
        // Vibrate
        AudioServicesPlaySystemSound(SystemSoundID(kSystemSoundID_Vibrate))

        // Stop scanning
        captureSession?.stopRunning()

        // Notify delegate
        delegate?.qrScanner(self, didScanCode: code)
    }
}

// MARK: - AVCaptureMetadataOutputObjectsDelegate
extension QRScannerViewController: AVCaptureMetadataOutputObjectsDelegate {
    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {

        // Get the first QR code found
        if let metadataObject = metadataObjects.first {
            guard let readableObject = metadataObject as? AVMetadataMachineReadableCodeObject else { return }
            guard let stringValue = readableObject.stringValue else { return }

            found(code: stringValue)
        }
    }
}
