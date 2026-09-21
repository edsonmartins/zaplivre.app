package com.zaplivre.ui.components

import android.util.Log
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.core.content.ContextCompat
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.MultiFormatReader
import com.google.zxing.NotFoundException
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Leitor de QR de contato em tela cheia (CameraX + zxing, ambos já usados pelo
 * app). O iOS lia o QR mas o Android só o exibia: um usuário Android não tinha
 * como adicionar um contato iOS que não registrou username.
 *
 * Exige a permissão de câmera concedida antes de ser mostrado.
 */
@Composable
fun QrScannerDialog(
    onResult: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val analysisExecutor = remember { Executors.newSingleThreadExecutor() }
    val delivered = remember { AtomicBoolean(false) }
    val reader = remember {
        MultiFormatReader().apply {
            setHints(
                mapOf(
                    DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE),
                    DecodeHintType.TRY_HARDER to true,
                )
            )
        }
    }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false),
    ) {
        Box(modifier = Modifier.fillMaxSize().background(Color.Black)) {
            AndroidView(
                modifier = Modifier.fillMaxSize(),
                factory = { viewContext ->
                    val previewView = PreviewView(viewContext)
                    val providerFuture = ProcessCameraProvider.getInstance(viewContext)
                    providerFuture.addListener({
                        val provider = providerFuture.get()
                        val preview = Preview.Builder().build().also {
                            it.setSurfaceProvider(previewView.surfaceProvider)
                        }
                        val analysis = ImageAnalysis.Builder()
                            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                            .build()
                        analysis.setAnalyzer(analysisExecutor) { image ->
                            val text = decode(reader, image)
                            if (text != null && delivered.compareAndSet(false, true)) {
                                ContextCompat.getMainExecutor(viewContext).execute {
                                    onResult(text)
                                }
                            }
                        }
                        try {
                            provider.unbindAll()
                            provider.bindToLifecycle(
                                lifecycleOwner,
                                CameraSelector.DEFAULT_BACK_CAMERA,
                                preview,
                                analysis,
                            )
                        } catch (e: Exception) {
                            Log.e(TAG, "Failed to start the camera", e)
                        }
                    }, ContextCompat.getMainExecutor(viewContext))
                    previewView
                },
            )
            Text(
                text = "Aponte para o QR do contato",
                color = Color.White,
                modifier = Modifier.align(Alignment.TopCenter).padding(top = 48.dp),
            )
            TextButton(
                onClick = onDismiss,
                modifier = Modifier.align(Alignment.BottomCenter).padding(bottom = 32.dp),
            ) { Text("Cancelar", color = Color.White) }
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            try {
                ProcessCameraProvider.getInstance(context).get().unbindAll()
            } catch (e: Exception) {
                Log.w(TAG, "Failed to release the camera", e)
            }
            analysisExecutor.shutdown()
        }
    }
}

/** Decode a QR from the luminance (Y) plane; null when there is none. */
private fun decode(reader: MultiFormatReader, image: ImageProxy): String? {
    return try {
        val plane = image.planes[0]
        val buffer = plane.buffer
        val data = ByteArray(buffer.remaining()).also { buffer.get(it) }
        val source = PlanarYUVLuminanceSource(
            data,
            plane.rowStride,
            image.height,
            0,
            0,
            image.width,
            image.height,
            false,
        )
        reader.decodeWithState(BinaryBitmap(HybridBinarizer(source))).text
    } catch (_: NotFoundException) {
        null
    } catch (e: Exception) {
        Log.w(TAG, "QR decode failed", e)
        null
    } finally {
        reader.reset()
        image.close()
    }
}

private const val TAG = "QrScanner"
