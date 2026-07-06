package com.zaplivre.ui.components

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

/**
 * FilePicker component - Allows user to select files from device storage
 *
 * Uses ActivityResultContracts.OpenDocument for file selection.
 * Supports all file types (MIME type: star/star).
 */
@Composable
fun FilePickerButton(
    onFilePicked: (Uri) -> Unit,
    enabled: Boolean = true,
    modifier: Modifier = Modifier
) {
    // File picker launcher
    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument()
    ) { uri: Uri? ->
        uri?.let { onFilePicked(it) }
    }

    // File picker button
    IconButton(
        onClick = {
            // Open file picker with all file types
            filePickerLauncher.launch(arrayOf("*/*"))
        },
        enabled = enabled,
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.AttachFile,
            contentDescription = "Attach file",
            tint = if (enabled) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
            }
        )
    }
}

/**
 * Data class for file information
 */
data class FileInfo(
    val uri: Uri,
    val fileName: String,
    val fileSize: Long,
    val mimeType: String
)
