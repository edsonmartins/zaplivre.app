package com.zaplivre.ui.components

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext

/**
 * AvatarPicker - Pick avatar from camera or gallery
 */
@Composable
fun AvatarPickerDialog(
    onImageSelected: (Uri) -> Unit,
    onDismiss: () -> Unit
) {
    val context = LocalContext.current

    // Gallery launcher
    val galleryLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        uri?.let {
            onImageSelected(it)
            onDismiss()
        }
    }

    // Camera launcher
    var cameraImageUri by remember { mutableStateOf<Uri?>(null) }
    val cameraLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.TakePicture()
    ) { success ->
        if (success && cameraImageUri != null) {
            onImageSelected(cameraImageUri!!)
            onDismiss()
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Escolher foto de perfil") },
        text = { Text("Selecione de onde deseja escolher a foto") },
        confirmButton = {
            Button(
                onClick = {
                    // TODO: Create camera image URI
                    // cameraLauncher.launch(cameraImageUri)
                    galleryLauncher.launch("image/*")
                }
            ) {
                Text("Câmera")
            }
        },
        dismissButton = {
            TextButton(
                onClick = {
                    galleryLauncher.launch("image/*")
                }
            ) {
                Text("Galeria")
            }
        }
    )
}
