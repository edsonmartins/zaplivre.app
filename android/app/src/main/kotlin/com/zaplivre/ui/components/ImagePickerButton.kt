package com.zaplivre.ui.components

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Image
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * Button to pick images using Android Photo Picker
 *
 * @param onImagesPicked Callback when images are selected
 * @param maxSelection Maximum number of images to select (default: 1)
 * @param modifier Modifier for styling
 */
@Composable
fun ImagePickerButton(
    onImagesPicked: (List<Uri>) -> Unit,
    maxSelection: Int = 1,
    modifier: Modifier = Modifier,
    enabled: Boolean = true
) {
    // Photo Picker launcher (Android 13+, backported to Android 11 via Google Play)
    val photoPicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickMultipleVisualMedia(maxItems = maxSelection)
    ) { uris ->
        if (uris.isNotEmpty()) {
            onImagesPicked(uris)
        }
    }

    IconButton(
        onClick = {
            photoPicker.launch(
                PickVisualMediaRequest(
                    mediaType = ActivityResultContracts.PickVisualMedia.ImageOnly
                )
            )
        },
        enabled = enabled,
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.Image,
            contentDescription = "Pick images",
            tint = if (enabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
        )
    }
}

/**
 * Button variant with text label
 */
@Composable
fun ImagePickerButtonWithLabel(
    onImagesPicked: (List<Uri>) -> Unit,
    maxSelection: Int = 1,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    text: String = "Select Images"
) {
    val photoPicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickMultipleVisualMedia(maxItems = maxSelection)
    ) { uris ->
        if (uris.isNotEmpty()) {
            onImagesPicked(uris)
        }
    }

    Button(
        onClick = {
            photoPicker.launch(
                PickVisualMediaRequest(
                    mediaType = ActivityResultContracts.PickVisualMedia.ImageOnly
                )
            )
        },
        enabled = enabled,
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.Image,
            contentDescription = null,
            modifier = Modifier.size(20.dp)
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(text)
    }
}

/**
 * Compact FAB variant for adding images in chat
 */
@Composable
fun ImagePickerFab(
    onImagesPicked: (List<Uri>) -> Unit,
    maxSelection: Int = 5,
    modifier: Modifier = Modifier
) {
    val photoPicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickMultipleVisualMedia(maxItems = maxSelection)
    ) { uris ->
        if (uris.isNotEmpty()) {
            onImagesPicked(uris)
        }
    }

    FloatingActionButton(
        onClick = {
            photoPicker.launch(
                PickVisualMediaRequest(
                    mediaType = ActivityResultContracts.PickVisualMedia.ImageOnly
                )
            )
        },
        modifier = modifier
    ) {
        Icon(
            imageVector = Icons.Default.Image,
            contentDescription = "Add images"
        )
    }
}
