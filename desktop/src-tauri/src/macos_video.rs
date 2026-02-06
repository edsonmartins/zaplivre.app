#[cfg(target_os = "macos")]
mod native {
    use std::ffi::c_void;
    use std::ptr;

    type OSStatus = i32;
    type CFAllocatorRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFMutableDictionaryRef = *mut c_void;
    type CFNumberRef = *const c_void;

    type CMVideoFormatDescriptionRef = *mut c_void;
    type VTDecompressionSessionRef = *mut c_void;
    type CMSampleBufferRef = *mut c_void;
    type CMBlockBufferRef = *mut c_void;
    type CVPixelBufferRef = *mut c_void;

    const K_CF_ALLOCATOR_DEFAULT: CFAllocatorRef = ptr::null();
    const K_CV_PIXEL_FORMAT_TYPE_32_BGRA: u32 = 1111970369; // 'BGRA'
    const K_CV_PIXEL_FORMAT_TYPE_420F: u32 = 875704438; // '420f'

    #[repr(C)]
    struct CMTime {
        value: i64,
        timescale: i32,
        flags: u32,
        epoch: i64,
    }

    #[repr(C)]
    struct CMSampleTimingInfo {
        duration: CMTime,
        presentation_time_stamp: CMTime,
        decode_time_stamp: CMTime,
    }

    #[repr(C)]
    struct VTDecompressionOutputCallbackRecord {
        callback: VTDecompressionOutputCallback,
        refcon: *mut c_void,
    }

    type VTDecompressionOutputCallback = extern "C" fn(
        *mut c_void,
        *mut c_void,
        OSStatus,
        u32,
        CVPixelBufferRef,
        CMTime,
        CMTime,
    );

    #[link(name = "VideoToolbox", kind = "framework")]
    extern "C" {
        fn VTDecompressionSessionCreate(
            allocator: CFAllocatorRef,
            format_description: CMVideoFormatDescriptionRef,
            decoder_spec: CFDictionaryRef,
            image_buffer_attributes: CFDictionaryRef,
            output_callback: *const VTDecompressionOutputCallbackRecord,
            decompression_session_out: *mut VTDecompressionSessionRef,
        ) -> OSStatus;

        fn VTDecompressionSessionDecodeFrame(
            session: VTDecompressionSessionRef,
            sample_buffer: CMSampleBufferRef,
            decode_flags: u32,
            source_frame_refcon: *mut c_void,
            info_flags_out: *mut u32,
        ) -> OSStatus;

        fn VTDecompressionSessionInvalidate(session: VTDecompressionSessionRef);
    }

    #[link(name = "CoreMedia", kind = "framework")]
    extern "C" {
        fn CMVideoFormatDescriptionCreateFromH264ParameterSets(
            allocator: CFAllocatorRef,
            parameter_set_count: usize,
            parameter_set_pointers: *const *const u8,
            parameter_set_sizes: *const usize,
            nal_unit_header_length: i32,
            format_description_out: *mut CMVideoFormatDescriptionRef,
        ) -> OSStatus;

        fn CMBlockBufferCreateWithMemoryBlock(
            allocator: CFAllocatorRef,
            memory_block: *mut c_void,
            block_length: usize,
            block_allocator: CFAllocatorRef,
            custom_block_source: *mut c_void,
            offset_to_data: usize,
            data_length: usize,
            flags: u32,
            block_buffer_out: *mut CMBlockBufferRef,
        ) -> OSStatus;

        fn CMBlockBufferReplaceDataBytes(
            source_bytes: *const c_void,
            block_buffer: CMBlockBufferRef,
            offset_into_destination: usize,
            data_length: usize,
        ) -> OSStatus;

        fn CMSampleBufferCreateReady(
            allocator: CFAllocatorRef,
            data_buffer: CMBlockBufferRef,
            format_description: CMVideoFormatDescriptionRef,
            num_samples: i32,
            num_sample_timing_entries: i32,
            sample_timing_array: *const CMSampleTimingInfo,
            num_sample_size_entries: i32,
            sample_size_array: *const usize,
            sample_buffer_out: *mut CMSampleBufferRef,
        ) -> OSStatus;

        fn CFNumberCreate(
            allocator: CFAllocatorRef,
            the_type: i32,
            value_ptr: *const c_void,
        ) -> CFNumberRef;

        fn CFDictionaryCreateMutable(
            allocator: CFAllocatorRef,
            capacity: isize,
            key_call_backs: *const c_void,
            value_call_backs: *const c_void,
        ) -> CFMutableDictionaryRef;

        fn CFDictionarySetValue(the_dict: CFMutableDictionaryRef, key: *const c_void, value: *const c_void);

        fn CFRelease(cf: *const c_void);

        fn CVPixelBufferLockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> OSStatus;
        fn CVPixelBufferUnlockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> OSStatus;
        fn CVPixelBufferGetWidth(pixel_buffer: CVPixelBufferRef) -> usize;
        fn CVPixelBufferGetHeight(pixel_buffer: CVPixelBufferRef) -> usize;
        fn CVPixelBufferGetBytesPerRow(pixel_buffer: CVPixelBufferRef) -> usize;
        fn CVPixelBufferGetPlaneCount(pixel_buffer: CVPixelBufferRef) -> usize;
        fn CVPixelBufferGetBaseAddress(pixel_buffer: CVPixelBufferRef) -> *mut c_void;
        fn CVPixelBufferGetBaseAddressOfPlane(pixel_buffer: CVPixelBufferRef, plane: usize) -> *mut c_void;
        fn CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer: CVPixelBufferRef, plane: usize) -> usize;
        fn CVPixelBufferGetPixelFormatType(pixel_buffer: CVPixelBufferRef) -> u32;
    }

    #[link(name = "CoreVideo", kind = "framework")]
    extern "C" {
        static kCVPixelBufferPixelFormatTypeKey: *const c_void;
    }

    const K_CF_NUMBER_S32_TYPE: i32 = 3;

    #[derive(Default)]
    pub struct DecodedFrame {
        pub width: u32,
        pub height: u32,
        pub rgba: Vec<u8>,
    }

    pub struct MacVideoDecoder {
        session: Option<VTDecompressionSessionRef>,
        format_desc: Option<CMVideoFormatDescriptionRef>,
        sps: Option<Vec<u8>>,
        pps: Option<Vec<u8>>,
        output: Box<std::sync::Mutex<Option<DecodedFrame>>>,
    }

    unsafe impl Send for MacVideoDecoder {}
    unsafe impl Sync for MacVideoDecoder {}

    impl MacVideoDecoder {
        pub fn new() -> Self {
            Self {
                session: None,
                format_desc: None,
                sps: None,
                pps: None,
                output: Box::new(std::sync::Mutex::new(None)),
            }
        }

        pub fn decode_annexb(&mut self, frame: &[u8]) -> Option<DecodedFrame> {
            let nalus = split_annexb_nalus(frame);
            if nalus.is_empty() {
                return None;
            }

            let mut is_key = false;
            for nalu in &nalus {
                let nalu_type = nalu[0] & 0x1F;
                if nalu_type == 7 {
                    self.sps = Some(nalu.clone());
                } else if nalu_type == 8 {
                    self.pps = Some(nalu.clone());
                } else if nalu_type == 5 {
                    is_key = true;
                }
            }

            if self.session.is_none() {
                let (sps, pps) = match (self.sps.as_ref(), self.pps.as_ref()) {
                    (Some(sps), Some(pps)) => (sps.clone(), pps.clone()),
                    _ => return None,
                };
                if self.build_session(&sps, &pps).is_err() {
                    return None;
                }
            }

            let avcc = nalus_to_avcc(&nalus);
            let sample = build_sample_buffer(self.format_desc?, &avcc)?;
            let mut info_flags: u32 = 0;

            unsafe {
                let status = VTDecompressionSessionDecodeFrame(
                    self.session?,
                    sample,
                    if is_key { 0 } else { 0 },
                    self.output.as_ref() as *const _ as *mut c_void,
                    &mut info_flags,
                );
                if status != 0 {
                    return None;
                }
            }

            self.output.lock().ok()?.take()
        }

        fn build_session(&mut self, sps: &[u8], pps: &[u8]) -> Result<(), OSStatus> {
            let format_desc = build_format_description(sps, pps)?;
            let session = build_decompression_session(format_desc, self.output.as_ref() as *const _ as *mut c_void)?;
            self.format_desc = Some(format_desc);
            self.session = Some(session);
            Ok(())
        }
    }

    impl Drop for MacVideoDecoder {
        fn drop(&mut self) {
            unsafe {
                if let Some(session) = self.session {
                    VTDecompressionSessionInvalidate(session);
                }
            }
            if let Some(desc) = self.format_desc {
                unsafe { CFRelease(desc as *const c_void) };
            }
        }
    }

    fn build_format_description(
        sps: &[u8],
        pps: &[u8],
    ) -> Result<CMVideoFormatDescriptionRef, OSStatus> {
        let sps_ptr = sps.as_ptr();
        let pps_ptr = pps.as_ptr();
        let params = [sps_ptr, pps_ptr];
        let sizes = [sps.len(), pps.len()];
        let mut desc: CMVideoFormatDescriptionRef = ptr::null_mut();
        let status = unsafe {
            CMVideoFormatDescriptionCreateFromH264ParameterSets(
                K_CF_ALLOCATOR_DEFAULT,
                2,
                params.as_ptr(),
                sizes.as_ptr(),
                4,
                &mut desc,
            )
        };
        if status != 0 {
            return Err(status);
        }
        Ok(desc)
    }

    fn build_decompression_session(
        format_desc: CMVideoFormatDescriptionRef,
        refcon: *mut c_void,
    ) -> Result<VTDecompressionSessionRef, OSStatus> {
        let pixel_format = K_CV_PIXEL_FORMAT_TYPE_32_BGRA;
        let attrs = unsafe {
            let dict = CFDictionaryCreateMutable(K_CF_ALLOCATOR_DEFAULT, 1, ptr::null(), ptr::null());
            let number = CFNumberCreate(K_CF_ALLOCATOR_DEFAULT, K_CF_NUMBER_S32_TYPE, &pixel_format as *const _ as *const c_void);
            CFDictionarySetValue(dict, kCVPixelBufferPixelFormatTypeKey, number);
            CFRelease(number as *const c_void);
            dict as CFDictionaryRef
        };

        let record = VTDecompressionOutputCallbackRecord {
            callback: decompression_callback,
            refcon,
        };

        let mut session: VTDecompressionSessionRef = ptr::null_mut();
        let status = unsafe {
            VTDecompressionSessionCreate(
                K_CF_ALLOCATOR_DEFAULT,
                format_desc,
                ptr::null(),
                attrs,
                &record,
                &mut session,
            )
        };
        unsafe { CFRelease(attrs as *const c_void) };
        if status != 0 {
            return Err(status);
        }
        Ok(session)
    }

    fn build_sample_buffer(
        format_desc: CMVideoFormatDescriptionRef,
        avcc: &[u8],
    ) -> Option<CMSampleBufferRef> {
        let mut block: CMBlockBufferRef = ptr::null_mut();
        let status = unsafe {
            CMBlockBufferCreateWithMemoryBlock(
                K_CF_ALLOCATOR_DEFAULT,
                ptr::null_mut(),
                avcc.len(),
                K_CF_ALLOCATOR_DEFAULT,
                ptr::null_mut(),
                0,
                avcc.len(),
                0,
                &mut block,
            )
        };
        if status != 0 {
            return None;
        }

        let status = unsafe {
            CMBlockBufferReplaceDataBytes(
                avcc.as_ptr() as *const c_void,
                block,
                0,
                avcc.len(),
            )
        };
        if status != 0 {
            return None;
        }

        let timing = CMSampleTimingInfo {
            duration: CMTime { value: 0, timescale: 1, flags: 0, epoch: 0 },
            presentation_time_stamp: CMTime { value: 0, timescale: 1, flags: 0, epoch: 0 },
            decode_time_stamp: CMTime { value: 0, timescale: 1, flags: 0, epoch: 0 },
        };

        let mut sample: CMSampleBufferRef = ptr::null_mut();
        let status = unsafe {
            CMSampleBufferCreateReady(
                K_CF_ALLOCATOR_DEFAULT,
                block,
                format_desc,
                1,
                1,
                &timing,
                0,
                ptr::null(),
                &mut sample,
            )
        };
        if status != 0 {
            return None;
        }

        Some(sample)
    }

    extern "C" fn decompression_callback(
        refcon: *mut c_void,
        _source_frame_refcon: *mut c_void,
        status: OSStatus,
        _info_flags: u32,
        image_buffer: CVPixelBufferRef,
        _pts: CMTime,
        _duration: CMTime,
    ) {
        if status != 0 || image_buffer.is_null() {
            return;
        }

        let output = unsafe { &*(refcon as *const std::sync::Mutex<Option<DecodedFrame>>) };
        let frame = match extract_rgba(image_buffer) {
            Some(frame) => frame,
            None => return,
        };

        if let Ok(mut guard) = output.lock() {
            *guard = Some(frame);
        }
    }

    fn extract_rgba(pixel_buffer: CVPixelBufferRef) -> Option<DecodedFrame> {
        let status = unsafe { CVPixelBufferLockBaseAddress(pixel_buffer, 0) };
        if status != 0 {
            return None;
        }

        let width = unsafe { CVPixelBufferGetWidth(pixel_buffer) } as u32;
        let height = unsafe { CVPixelBufferGetHeight(pixel_buffer) } as u32;
        let format = unsafe { CVPixelBufferGetPixelFormatType(pixel_buffer) };

        let mut rgba = vec![0u8; (width * height * 4) as usize];

        if format == K_CV_PIXEL_FORMAT_TYPE_32_BGRA {
            let bytes_per_row = unsafe { CVPixelBufferGetBytesPerRow(pixel_buffer) };
            let base = unsafe { CVPixelBufferGetBaseAddress(pixel_buffer) } as *const u8;
            for y in 0..height as usize {
                let row = unsafe { std::slice::from_raw_parts(base.add(y * bytes_per_row), bytes_per_row) };
                for x in 0..width as usize {
                    let src = x * 4;
                    let dst = (y * width as usize + x) * 4;
                    rgba[dst] = row[src + 2];
                    rgba[dst + 1] = row[src + 1];
                    rgba[dst + 2] = row[src];
                    rgba[dst + 3] = 255;
                }
            }
        } else if format == K_CV_PIXEL_FORMAT_TYPE_420F {
            let plane_count = unsafe { CVPixelBufferGetPlaneCount(pixel_buffer) };
            if plane_count < 2 {
                unsafe { CVPixelBufferUnlockBaseAddress(pixel_buffer, 0) };
                return None;
            }
            let y_base = unsafe { CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 0) } as *const u8;
            let uv_base = unsafe { CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 1) } as *const u8;
            let y_stride = unsafe { CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 0) };
            let uv_stride = unsafe { CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 1) };

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let y_value = unsafe { *y_base.add(y * y_stride + x) } as f32;
                    let uv_index = (y / 2) * uv_stride + (x / 2) * 2;
                    let u = unsafe { *uv_base.add(uv_index) } as f32 - 128.0;
                    let v = unsafe { *uv_base.add(uv_index + 1) } as f32 - 128.0;

                    let r = (y_value + 1.402 * v).clamp(0.0, 255.0);
                    let g = (y_value - 0.344 * u - 0.714 * v).clamp(0.0, 255.0);
                    let b = (y_value + 1.772 * u).clamp(0.0, 255.0);

                    let dst = (y * width as usize + x) * 4;
                    rgba[dst] = r as u8;
                    rgba[dst + 1] = g as u8;
                    rgba[dst + 2] = b as u8;
                    rgba[dst + 3] = 255;
                }
            }
        }

        unsafe { CVPixelBufferUnlockBaseAddress(pixel_buffer, 0) };

        Some(DecodedFrame { width, height, rgba })
    }

    fn split_annexb_nalus(data: &[u8]) -> Vec<Vec<u8>> {
        let mut nalus = Vec::new();
        let mut start: Option<usize> = None;
        let mut i = 0;

        while i + 3 < data.len() {
            let is_start_code_4 = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1;
            let is_start_code_3 = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1;

            if is_start_code_4 || is_start_code_3 {
                let start_code_len = if is_start_code_4 { 4 } else { 3 };
                if let Some(nalu_start) = start {
                    if nalu_start < i {
                        nalus.push(data[nalu_start..i].to_vec());
                    }
                }
                start = Some(i + start_code_len);
                i += start_code_len;
                continue;
            }
            i += 1;
        }

        if let Some(nalu_start) = start {
            if nalu_start < data.len() {
                nalus.push(data[nalu_start..].to_vec());
            }
        }

        nalus
    }

    fn nalus_to_avcc(nalus: &[Vec<u8>]) -> Vec<u8> {
        let total: usize = nalus.iter().map(|n| 4 + n.len()).sum();
        let mut out = Vec::with_capacity(total);
        for nalu in nalus {
            let len = nalu.len() as u32;
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(nalu);
        }
        out
    }
}

#[cfg(target_os = "macos")]
pub use native::MacVideoDecoder;
