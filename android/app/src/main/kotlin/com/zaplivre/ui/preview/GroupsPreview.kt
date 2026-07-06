package com.zaplivre.ui.preview

import androidx.compose.runtime.Composable
import com.zaplivre.ui.screens.group.GroupListContent
import com.zaplivre.ui.screens.group.GroupUi

/**
 * Harness de validação visual da lista de grupos (dados mock, sem client P2P).
 * Espelha o design preview das demais telas.
 */
@Composable
fun GroupsPreviewContent() {
    GroupListContent(
        groups = listOf(
            GroupUi("g-time", "Time ZapLivre", "Roadmap do trimestre · 8 membros", 8, isAdmin = true),
            GroupUi("g-familia", "Família", "5 membros", 5),
            GroupUi("g-facul", "Amigos da facul", "Reencontro de turma · 12 membros", 12),
            GroupUi("g-vizinhanca", "Vizinhança", "23 membros", 23),
            GroupUi("g-futebol", "Futebol de quinta", "Confirma presença aí · 16 membros", 16),
            GroupUi("g-viagem", "Viagem Chapada", "Planejamento da trilha · 6 membros", 6),
        ),
        onGroupClick = {},
        onCreateGroup = {},
        onBack = {},
    )
}
