package com.zaplivre.ui.theme

import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

/**
 * Tema ZapLivre. O colorScheme Material é derivado dos tokens [ZapColors] para
 * que componentes Material (TopAppBar, FAB, switches…) herdem a identidade azul
 * automaticamente; os detalhes de assinatura (bolhas, gradiente spark, avatares)
 * vêm de [ZapColor]/[LocalZapColors]. Espelha o design system do iOS.
 */
private fun materialLight(z: ZapColors) = lightColorScheme(
    primary = z.primary,
    onPrimary = z.onPrimary,
    primaryContainer = z.primary,
    onPrimaryContainer = z.onPrimary,
    secondary = z.spark,
    onSecondary = z.ink,
    background = z.canvas,
    onBackground = z.ink,
    surface = z.surface,
    onSurface = z.ink,
    surfaceVariant = z.bubbleIn,
    onSurfaceVariant = z.slate,
    outline = z.hairline,
    outlineVariant = z.hairline,
    error = z.danger,
)

private fun materialDark(z: ZapColors) = darkColorScheme(
    primary = z.primary,
    onPrimary = z.onPrimary,
    primaryContainer = z.primary,
    onPrimaryContainer = z.onPrimary,
    secondary = z.spark,
    onSecondary = z.ink,
    background = z.canvas,
    onBackground = z.ink,
    surface = z.surface,
    onSurface = z.ink,
    surfaceVariant = z.bubbleIn,
    onSurfaceVariant = z.slate,
    outline = z.hairline,
    outlineVariant = z.hairline,
    error = z.danger,
)

@Composable
fun ZapLivreTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val zapColors = if (darkTheme) DarkZapColors else LightZapColors
    val colorScheme = if (darkTheme) materialDark(zapColors) else materialLight(zapColors)

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            // Status bar acompanha o fundo da tela (canvas), estilo mensageiro
            // moderno — ícones escuros no light, claros no dark.
            window.statusBarColor = zapColors.canvas.toArgb()
            WindowCompat.getInsetsController(window, view).isAppearanceLightStatusBars = !darkTheme
        }
    }

    CompositionLocalProvider(LocalZapColors provides zapColors) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = Typography,
            shapes = ZapShapes,
            content = content
        )
    }
}
