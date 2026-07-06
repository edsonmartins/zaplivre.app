//
//  VideoFrameHandler.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import Foundation
import AVFoundation
import VideoToolbox
// Note: No need to import zaplivre - the generated Swift code (zaplivre.swift)
// is part of the same target. The bridging header already imports zaplivreFFI.h

/// VideoFrameHandler - Implements FfiVideoFrameCallback for rendering remote video
///
/// Receives H.264 encoded frames from FFI, decodes them using VideoToolbox,
/// and renders to an AVSampleBufferDisplayLayer.
class VideoFrameHandler: FfiVideoFrameCallback {

    private let callId: String
    private var displayLayer: AVSampleBufferDisplayLayer?
    private var formatDescription: CMVideoFormatDescription?
    private var lastSps: [UInt8]?
    private var lastPps: [UInt8]?

    // Callback for frame rendered events
    var onFrameRendered: ((UInt32, UInt32) -> Void)?

    init(callId: String) {
        self.callId = callId
    }

    /// Set the display layer for video rendering
    ///
    /// - Parameter layer: AVSampleBufferDisplayLayer to render frames to
    func setDisplayLayer(_ layer: AVSampleBufferDisplayLayer) {
        self.displayLayer = layer

        // Configure display layer
        layer.videoGravity = .resizeAspect
        layer.preventsDisplaySleepDuringVideoPlayback = true

        print("✅ Display layer configured for call: \(callId)")
    }

    /// Called from FFI when a remote video frame is received
    ///
    /// - Parameters:
    ///   - callId: Call identifier
    ///   - frameData: Raw H.264 frame data (NALUs)
    ///   - width: Frame width in pixels
    ///   - height: Frame height in pixels
    func onVideoFrame(callId: String, frameData: [UInt8], width: UInt32, height: UInt32) {
        // Ignore frames from other calls
        guard callId == self.callId else { return }

        // Check if display layer is set
        guard let displayLayer = displayLayer else {
            print("⚠️ Display layer not set, skipping frame")
            return
        }

        let nalus = splitNalUnits(frameData)
        for nalu in nalus {
            guard !nalu.isEmpty else { continue }
            let naluType = nalu[0] & 0x1F
            if naluType == 7 { // SPS
                lastSps = nalu
                updateFormatDescriptionIfNeeded()
                continue
            }
            if naluType == 8 { // PPS
                lastPps = nalu
                updateFormatDescriptionIfNeeded()
                continue
            }

            guard let sampleBuffer = createSampleBuffer(from: nalu) else {
                continue
            }

            DispatchQueue.main.async { [weak self] in
                displayLayer.enqueue(sampleBuffer)
                self?.onFrameRendered?(width, height)
            }
        }
    }

    private func updateFormatDescriptionIfNeeded() {
        guard formatDescription == nil,
              let sps = lastSps,
              let pps = lastPps else { return }

        var formatDesc: CMFormatDescription?
        let status = sps.withUnsafeBytes { spsPtr in
            pps.withUnsafeBytes { ppsPtr in
                guard let spsPointer = spsPtr.bindMemory(to: UInt8.self).baseAddress,
                      let ppsPointer = ppsPtr.bindMemory(to: UInt8.self).baseAddress else {
                    return OSStatus(-1)
                }
                let pointers: [UnsafePointer<UInt8>] = [spsPointer, ppsPointer]
                let sizes = [sps.count, pps.count]
                return CMVideoFormatDescriptionCreateFromH264ParameterSets(
                    allocator: kCFAllocatorDefault,
                    parameterSetCount: 2,
                    parameterSetPointers: pointers,
                    parameterSetSizes: sizes,
                    nalUnitHeaderLength: 4,
                    formatDescriptionOut: &formatDesc
                )
            }
        }

        guard status == noErr, let formatDesc = formatDesc else {
            print("❌ Failed to create format description: \(status)")
            return
        }

        formatDescription = formatDesc
    }

    /// Create CMSampleBuffer from a single NALU (no start code).
    private func createSampleBuffer(from nalu: [UInt8]) -> CMSampleBuffer? {
        guard let formatDescription = formatDescription else {
            return nil
        }

        var length = UInt32(nalu.count).bigEndian
        var data = Data(bytes: &length, count: 4)
        data.append(contentsOf: nalu)

        var blockBuffer: CMBlockBuffer?
        var status = CMBlockBufferCreateWithMemoryBlock(
            allocator: kCFAllocatorDefault,
            memoryBlock: nil,
            blockLength: data.count,
            blockAllocator: kCFAllocatorDefault,
            customBlockSource: nil,
            offsetToData: 0,
            dataLength: data.count,
            flags: 0,
            blockBufferOut: &blockBuffer
        )

        guard status == kCMBlockBufferNoErr, let blockBuffer = blockBuffer else {
            return nil
        }

        status = data.withUnsafeBytes { buffer in
            CMBlockBufferReplaceDataBytes(
                with: buffer.baseAddress!,
                blockBuffer: blockBuffer,
                offsetIntoDestination: 0,
                dataLength: data.count
            )
        }

        guard status == kCMBlockBufferNoErr else {
            return nil
        }

        var sampleBuffer: CMSampleBuffer?
        var timingInfo = CMSampleTimingInfo(
            duration: CMTime.invalid,
            presentationTimeStamp: CMClockGetTime(CMClockGetHostTimeClock()),
            decodeTimeStamp: CMTime.invalid
        )

        status = CMSampleBufferCreate(
            allocator: kCFAllocatorDefault,
            dataBuffer: blockBuffer,
            dataReady: true,
            makeDataReadyCallback: nil,
            refcon: nil,
            formatDescription: formatDescription,
            sampleCount: 1,
            sampleTimingEntryCount: 1,
            sampleTimingArray: &timingInfo,
            sampleSizeEntryCount: 0,
            sampleSizeArray: nil,
            sampleBufferOut: &sampleBuffer
        )

        guard status == noErr, let sampleBuffer = sampleBuffer else {
            return nil
        }

        return sampleBuffer
    }

    private func splitNalUnits(_ data: [UInt8]) -> [[UInt8]] {
        var nalus = [[UInt8]]()
        var start = -1
        var i = 0

        while i + 3 < data.count {
            let isStartCode4 = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1
            let isStartCode3 = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1

            if isStartCode4 || isStartCode3 {
                let startCodeLength = isStartCode4 ? 4 : 3
                if start >= 0 {
                    let nalu = Array(data[start..<i])
                    nalus.append(nalu)
                }
                start = i + startCodeLength
                i += startCodeLength
                continue
            }
            i += 1
        }

        if start >= 0 && start < data.count {
            nalus.append(Array(data[start..<data.count]))
        }

        return nalus
    }

    /// Cleanup resources
    func release() {
        displayLayer = nil
        formatDescription = nil
        lastSps = nil
        lastPps = nil
        print("🧹 VideoFrameHandler released for call: \(callId)")
    }
}
