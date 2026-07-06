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
}
