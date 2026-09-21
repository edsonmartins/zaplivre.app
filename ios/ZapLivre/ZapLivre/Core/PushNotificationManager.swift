//
//  PushNotificationManager.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import Foundation
import UserNotifications
import UIKit

/// Manages push notifications registration and handling
class PushNotificationManager: NSObject, ObservableObject {
    @Published var deviceToken: String?
    @Published var isRegistered = false

    weak var appState: AppState?

    private let pushServerURL: String = {
        if let url = Bundle.main.object(forInfoDictionaryKey: "PUSH_SERVER_URL") as? String {
            if !url.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                return url
            }
        }
        return "https://push.zaplivre.app"
    }()

    /// Request push notification permissions
    func requestAuthorization() {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
            if let error = error {
                print("❌ Push notification authorization error: \(error)")
                return
            }

            if granted {
                print("✅ Push notifications authorized")
                DispatchQueue.main.async {
                    UIApplication.shared.registerForRemoteNotifications()
                }
            } else {
                print("⚠️  Push notifications not authorized by user")
            }
        }
    }

    /// Called when device token is successfully registered with APNs
    func didRegisterForRemoteNotifications(deviceToken: Data) {
        // Convert device token to hex string
        let tokenString = deviceToken.map { String(format: "%02.2hhx", $0) }.joined()

        print("🍎 APNs device token received")

        DispatchQueue.main.async {
            self.deviceToken = tokenString
        }

        // Register token with push server
        Task {
            await self.registerTokenWithServer(token: tokenString)
        }
    }

    /// Re-register token after peer ID is available
    func refreshRegistration() {
        guard let token = deviceToken else { return }
        Task {
            await self.registerTokenWithServer(token: token)
        }
    }

    /// Called when registration fails
    func didFailToRegisterForRemoteNotifications(error: Error) {
        print("❌ Failed to register for remote notifications: \(error)")
    }

    /// Register device token with ZapLivre push server
    private func registerTokenWithServer(token: String) async {
        guard let url = URL(string: "\(pushServerURL)/api/v1/register") else {
            print("❌ Invalid push server URL")
            return
        }

        // Get peer ID from ZapLivre core
        let peerId = ZapLivreCore.shared.localPeerId
            ?? UserDefaults.standard.string(forKey: "local_peer_id")
            ?? ""
        if peerId.isEmpty {
            print("⚠️  Push registration skipped: missing peer ID")
            return
        }

        let payload: [String: Any] = [
            "peer_id": peerId,
            "platform": "apns",
            "device_id": UIDevice.current.identifierForVendor?.uuidString ?? UUID().uuidString,
            "token": token,
            "device_name": UIDevice.current.name,
            "app_version": Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.1.0"
        ]

        do {
            var request = URLRequest(url: url)
            request.httpMethod = "POST"
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            let body = try JSONSerialization.data(withJSONObject: payload, options: [.sortedKeys])
            let timestamp = Int64(Date().timeIntervalSince1970)
            let signature = try await ZapLivreCore.shared.signAuthRequest(
                method: "POST",
                path: "/api/v1/register",
                timestamp: timestamp,
                body: body
            )
            request.setValue(peerId, forHTTPHeaderField: "x-zaplivre-peer")
            request.setValue(String(timestamp), forHTTPHeaderField: "x-zaplivre-ts")
            request.setValue(signature, forHTTPHeaderField: "x-zaplivre-sig")
            request.httpBody = body

            let (data, response) = try await URLSession.shared.data(for: request)

            if let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 {
                print("✅ Device token registered with push server")
                DispatchQueue.main.async {
                    self.isRegistered = true
                }
            } else {
                print("❌ Push server registration failed")
                if let responseString = String(data: data, encoding: .utf8) {
                    print("   Response: \(responseString)")
                }
            }
        } catch {
            print("❌ Failed to register token with server: \(error)")
        }
    }

    /// Tell the push server to stop notifying this device. Must run while the
    /// identity still exists: the request is signed with it.
    func unregisterFromServer() async {
        guard let url = URL(string: "\(pushServerURL)/api/v1/unregister"),
              let peerId = ZapLivreCore.shared.localPeerId, !peerId.isEmpty,
              let deviceId = UIDevice.current.identifierForVendor?.uuidString else { return }

        do {
            var request = URLRequest(url: url)
            request.httpMethod = "DELETE"
            request.timeoutInterval = 5
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            let body = try JSONSerialization.data(
                withJSONObject: ["peer_id": peerId, "device_id": deviceId],
                options: [.sortedKeys]
            )
            let timestamp = Int64(Date().timeIntervalSince1970)
            let signature = try await ZapLivreCore.shared.signAuthRequest(
                method: "DELETE",
                path: "/api/v1/unregister",
                timestamp: timestamp,
                body: body
            )
            request.setValue(peerId, forHTTPHeaderField: "x-zaplivre-peer")
            request.setValue(String(timestamp), forHTTPHeaderField: "x-zaplivre-ts")
            request.setValue(signature, forHTTPHeaderField: "x-zaplivre-sig")
            request.httpBody = body
            _ = try await URLSession.shared.data(for: request)
            print("✅ Device token unregistered from push server")
        } catch {
            print("⚠️ Push unregistration failed: \(error)")
        }
    }

    /// Handle incoming push notification
    func handleNotification(userInfo: [AnyHashable: Any], openConversation: Bool = false) {
        print("📨 Push notification received")

        // Extract notification data
        if let aps = userInfo["aps"] as? [String: Any] {
            if let alert = aps["alert"] as? [String: String] {
                let title = alert["title"] ?? "New Message"
                let body = alert["body"] ?? ""
                print("   Title: \(title)")
                _ = body
            }

            if let badge = aps["badge"] as? Int {
                DispatchQueue.main.async {
                    // IOS-12: API nova (a antiga é deprecated no iOS 17)
                    if #available(iOS 16.0, *) {
                        UNUserNotificationCenter.current().setBadgeCount(badge)
                    } else {
                        UIApplication.shared.applicationIconBadgeNumber = badge
                    }
                }
            }
        }

        // The push only says something is waiting: the message itself sits on
        // the message store (the sender fell back to it because we were not
        // reachable). Drain the mailbox; dialing the sender, below, does not
        // bring that message in.
        Task {
            do {
                let fetched = try await ZapLivreCore.shared.fetchOfflineMessages()
                print("📬 Offline mailbox drained after push: \(fetched)")
            } catch {
                print("⚠️ Offline mailbox fetch failed: \(error)")
            }
        }

        // Handle custom data
        // The push target's `peer_id` identifies this device. The sender is
        // carried in the data payload and is the peer we must dial.
        if let peerId = userInfo["sender_peer_id"] as? String
            ?? userInfo["senderPeerId"] as? String
            ?? userInfo["peer_id"] as? String
            ?? userInfo["peerId"] as? String {
            print("   From peer: \(peerId)")
            // A push is also a wake-up signal for the direct P2P transport.
            // Dial in the background/foreground without requiring the user to
            // open the conversation first; the core then updates unread state.
            if let addr = UserDefaults.standard.string(forKey: "zaplivre.multiaddr.\(peerId)") {
                Task {
                    do {
                        try await ZapLivreCore.shared.connectToPeer(peerId: peerId, multiaddr: addr)
                        print("✅ P2P connection started from push wake-up")
                    } catch {
                        print("⚠️ Push wake-up connection failed: \(error)")
                    }
                }
            }
            if openConversation {
                DispatchQueue.main.async { [weak self] in
                    self?.appState?.openConversation(peerId: peerId)
                }
            }
        }
    }

    /// Clear badge count
    func clearBadge() {
        if #available(iOS 16.0, *) {
            UNUserNotificationCenter.current().setBadgeCount(0)
        } else {
            UIApplication.shared.applicationIconBadgeNumber = 0
        }
    }
}

// MARK: - UNUserNotificationCenterDelegate
extension PushNotificationManager: UNUserNotificationCenterDelegate {
    /// Handle notification when app is in foreground
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        print("📨 Notification received in foreground")
        handleNotification(userInfo: notification.request.content.userInfo)

        // Show notification even when app is in foreground
        completionHandler([.banner, .sound, .badge])
    }

    /// Handle notification tap
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        print("📬 User tapped notification")
        handleNotification(userInfo: response.notification.request.content.userInfo, openConversation: true)
        completionHandler()
    }
}
