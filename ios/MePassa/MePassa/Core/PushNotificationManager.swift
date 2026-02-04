//
//  PushNotificationManager.swift
//  MePassa
//
//  Created by MePassa Team
//  Copyright © 2026 MePassa. All rights reserved.
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
        return "https://push.associahub.com.br"
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

        print("🍎 APNs device token: \(tokenString)")

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

    /// Register device token with MePassa push server
    private func registerTokenWithServer(token: String) async {
        guard let url = URL(string: "\(pushServerURL)/api/v1/register") else {
            print("❌ Invalid push server URL")
            return
        }

        // Get peer ID from MePassa core
        let peerId = MePassaCore.shared.localPeerId
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
            request.httpBody = try JSONSerialization.data(withJSONObject: payload)

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

    /// Handle incoming push notification
    func handleNotification(userInfo: [AnyHashable: Any]) {
        print("📨 Received push notification: \(userInfo)")

        // Extract notification data
        if let aps = userInfo["aps"] as? [String: Any] {
            if let alert = aps["alert"] as? [String: String] {
                let title = alert["title"] ?? "New Message"
                let body = alert["body"] ?? ""
                print("   Title: \(title)")
                print("   Body: \(body)")
            }

            if let badge = aps["badge"] as? Int {
                DispatchQueue.main.async {
                    UIApplication.shared.applicationIconBadgeNumber = badge
                }
            }
        }

        // Handle custom data
        if let peerId = userInfo["peer_id"] as? String ?? userInfo["peerId"] as? String {
            print("   From peer: \(peerId)")
            DispatchQueue.main.async { [weak self] in
                self?.appState?.openConversation(peerId: peerId)
            }
        }
    }

    /// Clear badge count
    func clearBadge() {
        UIApplication.shared.applicationIconBadgeNumber = 0
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
        handleNotification(userInfo: response.notification.request.content.userInfo)
        completionHandler()
    }
}
