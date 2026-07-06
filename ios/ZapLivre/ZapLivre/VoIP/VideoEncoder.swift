//
//  VideoEncoder.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//

import AVFoundation
import VideoToolbox

/// VideoEncoder - Encodes CVPixelBuffer frames into H.264 NALUs.
final class VideoEncoder {
    private let width: Int
    private let height: Int
    private let onEncoded: ([UInt8], Bool) -> Void
    private var session: VTCompressionSession?

    private let queue = DispatchQueue(label: "com.zaplivre.videoEncoder")

    init(width: Int, height: Int, onEncoded: @escaping ([UInt8], Bool) -> Void) {
        self.width = width
        self.height = height
        self.onEncoded = onEncoded
    }

    func start() {
        guard session == nil else { return }

        var compressionSession: VTCompressionSession?
        let status = VTCompressionSessionCreate(
            allocator: kCFAllocatorDefault,
            width: Int32(width),
            height: Int32(height),
            codecType: kCMVideoCodecType_H264,
            encoderSpecification: nil,
            imageBufferAttributes: nil,
            compressedDataAllocator: nil,
            outputCallback: { outputRefCon, _, status, _, sampleBuffer in
                guard status == noErr,
                      let sampleBuffer = sampleBuffer,
                      CMSampleBufferDataIsReady(sampleBuffer) else {
                    return
                }

                let encoder = Unmanaged<VideoEncoder>
                    .fromOpaque(outputRefCon!)
                    .takeUnretainedValue()
                encoder.handleEncodedSample(sampleBuffer)
            },
            refcon: Unmanaged.passUnretained(self).toOpaque(),
            compressionSessionOut: &compressionSession
        )

        guard status == noErr, let compressionSession = compressionSession else {
            print("❌ Failed to create VTCompressionSession: \(status)")
            return
        }

        VTSessionSetProperty(compressionSession, key: kVTCompressionPropertyKey_RealTime, value: kCFBooleanTrue)
        VTSessionSetProperty(compressionSession, key: kVTCompressionPropertyKey_ProfileLevel, value: kVTProfileLevel_H264_Baseline_AutoLevel)
        VTSessionSetProperty(compressionSession, key: kVTCompressionPropertyKey_AllowFrameReordering, value: kCFBooleanFalse)
        VTSessionSetProperty(compressionSession, key: kVTCompressionPropertyKey_AverageBitRate, value: NSNumber(value: 800_000))
        VTSessionSetProperty(compressionSession, key: kVTCompressionPropertyKey_ExpectedFrameRate, value: NSNumber(value: 15))
        VTSessionSetProperty(compressionSession, key: kVTCompressionPropertyKey_MaxKeyFrameInterval, value: NSNumber(value: 30))

        VTCompressionSessionPrepareToEncodeFrames(compressionSession)

        session = compressionSession
        print("✅ VideoEncoder started (\(width)x\(height))")
    }

    func stop() {
        guard let session = session else { return }
        VTCompressionSessionCompleteFrames(session, untilPresentationTimeStamp: .invalid)
        VTCompressionSessionInvalidate(session)
        self.session = nil
        print("🛑 VideoEncoder stopped")
    }

    func encode(pixelBuffer: CVPixelBuffer, pts: CMTime) {
        guard let session = session else { return }

        queue.async { [weak self] in
            guard let self = self else { return }
            var flags: VTEncodeInfoFlags = []
            let status = VTCompressionSessionEncodeFrame(
                session,
                imageBuffer: pixelBuffer,
                presentationTimeStamp: pts,
                duration: CMTime.invalid,
                frameProperties: nil,
                sourceFrameRefcon: nil,
                infoFlagsOut: &flags
            )
            if status != noErr {
                print("❌ VTCompressionSessionEncodeFrame failed: \(status)")
            }
        }
    }

    private func handleEncodedSample(_ sampleBuffer: CMSampleBuffer) {
        guard let dataBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else { return }
        let isKeyframe = VideoEncoder.isKeyframe(sampleBuffer)
        let avccData = dataBuffer.toData()
        let nalus = VideoEncoder.extractNalus(from: avccData)

        var output = [UInt8]()
        if isKeyframe, let format = CMSampleBufferGetFormatDescription(sampleBuffer) {
            if let spsPps = VideoEncoder.extractSpsPps(format: format) {
                output.append(contentsOf: spsPps)
            }
        }

        for nalu in nalus {
            output.append(contentsOf: [0x00, 0x00, 0x00, 0x01])
            output.append(contentsOf: nalu)
        }

        onEncoded(output, isKeyframe)
    }

    private static func isKeyframe(_ sampleBuffer: CMSampleBuffer) -> Bool {
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(sampleBuffer, createIfNecessary: false) as? [[CFString: Any]],
              let first = attachments.first else {
            return false
        }
        let notSync = first[kCMSampleAttachmentKey_NotSync] as? Bool ?? false
        return !notSync
    }

    private static func extractSpsPps(format: CMFormatDescription) -> [UInt8]? {
        var spsPointer: UnsafePointer<UInt8>?
        var spsSize: Int = 0
        var spsCount: Int = 0
        var ppsPointer: UnsafePointer<UInt8>?
        var ppsSize: Int = 0
        var ppsCount: Int = 0

        guard CMVideoFormatDescriptionGetH264ParameterSetAtIndex(format, parameterSetIndex: 0, parameterSetPointerOut: &spsPointer, parameterSetSizeOut: &spsSize, parameterSetCountOut: &spsCount, nalUnitHeaderLengthOut: nil) == noErr,
              CMVideoFormatDescriptionGetH264ParameterSetAtIndex(format, parameterSetIndex: 1, parameterSetPointerOut: &ppsPointer, parameterSetSizeOut: &ppsSize, parameterSetCountOut: &ppsCount, nalUnitHeaderLengthOut: nil) == noErr,
              let sps = spsPointer,
              let pps = ppsPointer else {
            return nil
        }

        var data = [UInt8]()
        data.append(contentsOf: [0x00, 0x00, 0x00, 0x01])
        data.append(contentsOf: Array(UnsafeBufferPointer(start: sps, count: spsSize)))
        data.append(contentsOf: [0x00, 0x00, 0x00, 0x01])
        data.append(contentsOf: Array(UnsafeBufferPointer(start: pps, count: ppsSize)))
        return data
    }

    private static func extractNalus(from avccData: Data) -> [[UInt8]] {
        var nalus = [[UInt8]]()
        var offset = 0

        while offset + 4 <= avccData.count {
            let length = avccData.subdata(in: offset..<(offset + 4)).withUnsafeBytes { ptr -> UInt32 in
                return ptr.load(as: UInt32.self).bigEndian
            }
            offset += 4
            guard offset + Int(length) <= avccData.count else { break }
            let nalu = avccData.subdata(in: offset..<(offset + Int(length)))
            nalus.append([UInt8](nalu))
            offset += Int(length)
        }

        return nalus
    }
}

private extension CMBlockBuffer {
    func toData() -> Data {
        var length = 0
        var dataPointer: UnsafeMutablePointer<Int8>?
        let status = CMBlockBufferGetDataPointer(self, atOffset: 0, lengthAtOffsetOut: nil, totalLengthOut: &length, dataPointerOut: &dataPointer)
        guard status == kCMBlockBufferNoErr, let pointer = dataPointer else {
            return Data()
        }
        return Data(bytes: pointer, count: length)
    }
}
