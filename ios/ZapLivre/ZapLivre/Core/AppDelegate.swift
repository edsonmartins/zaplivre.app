//
//  AppDelegate.swift
//  ZapLivre
//
//  Created by ZapLivre Team
//  Copyright © 2026 ZapLivre. All rights reserved.
//

import UIKit
import UserNotifications

class AppDelegate: NSObject, UIApplicationDelegate {
    // O pushManager é injetado pelo ZapLivreApp DEPOIS do didFinishLaunching,
    // então o delegate de notificações precisa ser (re)atribuído na injeção.
    var pushManager: PushNotificationManager? {
        didSet {
            UNUserNotificationCenter.current().delegate = pushManager
        }
    }

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        print("📱 ZapLivre AppDelegate - didFinishLaunching")
        ZapLivreApp.discardIdentityOrphanedByReinstall()
        return true
    }

    // MARK: - Push Notifications

    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        pushManager?.didRegisterForRemoteNotifications(deviceToken: deviceToken)
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: Error
    ) {
        pushManager?.didFailToRegisterForRemoteNotifications(error: error)
    }

    // Handle content-available pushes while the app is in the background.
    // This wakes the P2P layer so queued messages are delivered without
    // requiring the user to open the conversation.
    func application(
        _ application: UIApplication,
        didReceiveRemoteNotification userInfo: [AnyHashable: Any],
        fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
    ) {
        pushManager?.handleNotification(userInfo: userInfo)
        completionHandler(.newData)
    }
}
