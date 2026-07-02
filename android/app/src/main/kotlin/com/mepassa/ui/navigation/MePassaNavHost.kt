package com.mepassa.ui.navigation

import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.mepassa.core.MePassaClientWrapper
import com.mepassa.ui.screens.call.CallScreen
import com.mepassa.ui.screens.call.IncomingCallScreen
import com.mepassa.ui.screens.call.VideoCallScreen
import com.mepassa.ui.screens.chat.ChatScreen
import com.mepassa.ui.screens.conversations.ConversationsScreen
import com.mepassa.ui.screens.group.GroupChatScreen
import com.mepassa.ui.screens.group.GroupInfoScreen
import com.mepassa.ui.screens.group.GroupListScreen
import com.mepassa.ui.screens.onboarding.OnboardingScreen
import com.mepassa.ui.utils.getPermissionDeniedMessage
import com.mepassa.ui.utils.rememberVoipPermissions
import kotlinx.coroutines.launch

/**
 * Rotas de navegação do app
 */
sealed class Screen(val route: String) {
    object Onboarding : Screen("onboarding")
    object Conversations : Screen("conversations")
    object Chat : Screen("chat/{peerId}") {
        fun createRoute(peerId: String) = "chat/$peerId"
    }
    object GroupList : Screen("groups")
    object GroupChat : Screen("group_chat/{groupId}") {
        fun createRoute(groupId: String) = "group_chat/$groupId"
    }
    object GroupInfo : Screen("group_info/{groupId}") {
        fun createRoute(groupId: String) = "group_info/$groupId"
    }
    object IncomingCall : Screen("incoming_call/{callId}/{callerPeerId}") {
        fun createRoute(callId: String, callerPeerId: String) = "incoming_call/$callId/$callerPeerId"
    }
    object ActiveCall : Screen("active_call/{callId}/{remotePeerId}") {
        fun createRoute(callId: String, remotePeerId: String) = "active_call/$callId/$remotePeerId"
    }
    object VideoCall : Screen("video_call/{callId}/{remotePeerId}") {
        fun createRoute(callId: String, remotePeerId: String) = "video_call/$callId/$remotePeerId"
    }
}

/**
 * NavHost principal do app
 *
 * Gerencia navegação entre:
 * - Onboarding (primeira vez)
 * - Conversations (lista de conversas)
 * - Chat (conversa específica)
 */
@Composable
fun MePassaNavHost(
    isClientInitialized: Boolean,
    pendingPeerId: String?,
    onPeerIdConsumed: () -> Unit,
    navController: NavHostController = rememberNavController()
) {
    // Determina tela inicial baseado no estado do client
    val startDestination = if (isClientInitialized) {
        Screen.Conversations.route
    } else {
        Screen.Onboarding.route
    }

    LaunchedEffect(pendingPeerId, isClientInitialized) {
        val peerId = pendingPeerId?.takeIf { it.isNotBlank() } ?: return@LaunchedEffect
        if (!isClientInitialized) return@LaunchedEffect
        navController.navigate(Screen.Chat.createRoute(peerId)) {
            launchSingleTop = true
        }
        onPeerIdConsumed()
    }

    // Chamada recebida (core -> FfiCallEventCallback -> StateFlow): navegar
    // para a tela de IncomingCall assim que o evento chegar
    val incomingCall by MePassaClientWrapper.incomingCall.collectAsState()
    LaunchedEffect(incomingCall) {
        val call = incomingCall ?: return@LaunchedEffect
        navController.navigate(Screen.IncomingCall.createRoute(call.callId, call.callerPeerId)) {
            launchSingleTop = true
        }
        MePassaClientWrapper.consumeIncomingCall()
    }

    NavHost(navController = navController, startDestination = startDestination) {
        // Onboarding
        composable(Screen.Onboarding.route) {
            OnboardingScreen(
                onOnboardingComplete = {
                    // Navegar para Conversations e remover Onboarding da pilha
                    navController.navigate(Screen.Conversations.route) {
                        popUpTo(Screen.Onboarding.route) { inclusive = true }
                    }
                }
            )
        }

        // Lista de conversas
        composable(Screen.Conversations.route) {
            ConversationsScreen(
                onConversationClick = { peerId ->
                    navController.navigate(Screen.Chat.createRoute(peerId))
                },
                onGroupsClick = {
                    navController.navigate(Screen.GroupList.route)
                }
            )
        }

        // Chat (conversa específica)
        composable(
            route = Screen.Chat.route,
            arguments = listOf(
                navArgument("peerId") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val peerId = backStackEntry.arguments?.getString("peerId") ?: return@composable
            val scope = rememberCoroutineScope()
            val snackbarHostState = remember { SnackbarHostState() }

            // Gerenciamento de permissões VoIP
            val voipPermissions = rememberVoipPermissions(
                onPermissionsGranted = {
                    // Permissões concedidas - iniciar chamada
                    scope.launch {
                        val result = MePassaClientWrapper.startCall(peerId)
                        result.onSuccess { callId ->
                            navController.navigate(Screen.ActiveCall.createRoute(callId, peerId))
                        }.onFailure { error ->
                            scope.launch {
                                snackbarHostState.showSnackbar(
                                    message = "Erro ao iniciar chamada: ${error.message}",
                                    duration = SnackbarDuration.Short
                                )
                            }
                            android.util.Log.e("ChatScreen", "Failed to start call: ${error.message}")
                        }
                    }
                },
                onPermissionsDenied = { deniedPermissions ->
                    // Permissões negadas - mostrar mensagem
                    scope.launch {
                        val message = getPermissionDeniedMessage(deniedPermissions)
                        snackbarHostState.showSnackbar(
                            message = message,
                            duration = SnackbarDuration.Long
                        )
                    }
                }
            )

            ChatScreen(
                peerId = peerId,
                onNavigateBack = {
                    navController.popBackStack()
                },
                onStartCall = {
                    // Verificar e solicitar permissões antes de iniciar chamada
                    if (voipPermissions.hasPermissions) {
                        // Já tem permissões - iniciar chamada diretamente
                        scope.launch {
                            val result = MePassaClientWrapper.startCall(peerId)
                            result.onSuccess { callId ->
                                navController.navigate(Screen.ActiveCall.createRoute(callId, peerId))
                            }.onFailure { error ->
                                scope.launch {
                                    snackbarHostState.showSnackbar(
                                        message = "Erro ao iniciar chamada: ${error.message}",
                                        duration = SnackbarDuration.Short
                                    )
                                }
                                android.util.Log.e("ChatScreen", "Failed to start call: ${error.message}")
                            }
                        }
                    } else {
                        // Solicitar permissões primeiro
                        voipPermissions.requestPermissions()
                    }
                }
            )
        }

        // Group List (lista de grupos)
        composable(Screen.GroupList.route) {
            GroupListScreen(
                onGroupClick = { groupId ->
                    navController.navigate(Screen.GroupChat.createRoute(groupId))
                },
                onBack = {
                    navController.popBackStack()
                }
            )
        }

        // Group Chat (conversa em grupo)
        composable(
            route = Screen.GroupChat.route,
            arguments = listOf(
                navArgument("groupId") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val groupId = backStackEntry.arguments?.getString("groupId") ?: return@composable

            GroupChatScreen(
                groupId = groupId,
                onNavigateBack = {
                    navController.popBackStack()
                },
                onGroupInfo = { groupId ->
                    navController.navigate(Screen.GroupInfo.createRoute(groupId))
                }
            )
        }

        // Group Info (informações do grupo)
        composable(
            route = Screen.GroupInfo.route,
            arguments = listOf(
                navArgument("groupId") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val groupId = backStackEntry.arguments?.getString("groupId") ?: return@composable

            GroupInfoScreen(
                groupId = groupId,
                onNavigateBack = {
                    navController.popBackStack()
                }
            )
        }

        // Incoming Call (chamada recebida - fullscreen)
        composable(
            route = Screen.IncomingCall.route,
            arguments = listOf(
                navArgument("callId") { type = NavType.StringType },
                navArgument("callerPeerId") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val callId = backStackEntry.arguments?.getString("callId") ?: return@composable
            val callerPeerId = backStackEntry.arguments?.getString("callerPeerId") ?: return@composable

            IncomingCallScreen(
                callId = callId,
                callerPeerId = callerPeerId,
                onAccept = {
                    // Navegar para ActiveCall e remover IncomingCall da pilha
                    navController.navigate(Screen.ActiveCall.createRoute(callId, callerPeerId)) {
                        popUpTo(Screen.IncomingCall.route) { inclusive = true }
                    }
                },
                onReject = {
                    // Voltar para tela anterior
                    navController.popBackStack()
                }
            )
        }

        // Active Call (chamada ativa)
        composable(
            route = Screen.ActiveCall.route,
            arguments = listOf(
                navArgument("callId") { type = NavType.StringType },
                navArgument("remotePeerId") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val callId = backStackEntry.arguments?.getString("callId") ?: return@composable
            val remotePeerId = backStackEntry.arguments?.getString("remotePeerId") ?: return@composable

            CallScreen(
                callId = callId,
                remotePeerId = remotePeerId,
                onOpenVideo = {
                    navController.navigate(Screen.VideoCall.createRoute(callId, remotePeerId))
                },
                onCallEnded = {
                    // Voltar para Conversations
                    navController.popBackStack(Screen.Conversations.route, inclusive = false)
                }
            )
        }

        // Video Call (video ativa)
        composable(
            route = Screen.VideoCall.route,
            arguments = listOf(
                navArgument("callId") { type = NavType.StringType },
                navArgument("remotePeerId") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val callId = backStackEntry.arguments?.getString("callId") ?: return@composable
            val remotePeerId = backStackEntry.arguments?.getString("remotePeerId") ?: return@composable

            VideoCallScreen(
                callId = callId,
                peerName = remotePeerId.take(16) + "...",
                onHangup = {
                    navController.popBackStack(Screen.Conversations.route, inclusive = false)
                }
            )
        }
    }
}
