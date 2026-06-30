import SwiftUI

@main
struct SimpleGitClientApp: App {
    @State private var appViewModel = AppViewModel()

    var body: some Scene {
        WindowGroup {
            MainLayout()
                .environment(appViewModel)
                .environment(ToastCenter.shared)
                .environment(ActivityCenter.shared)
                .frame(minWidth: 1100, minHeight: 700)
                .background(Theme.bgApp)
                .preferredColorScheme(.dark)
        }
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1400, height: 900)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open Repository...") {
                    appViewModel.showOpenDialog()
                }
                .keyboardShortcut("o", modifiers: .command)

                Divider()

                Button("New Tab") {
                    appViewModel.showOpenDialog()
                }
                .keyboardShortcut("t", modifiers: .command)
            }

            CommandGroup(after: .windowArrangement) {
                Button("Close Tab") {
                    if let id = appViewModel.activeTabId {
                        appViewModel.closeTab(id)
                    }
                }
                .keyboardShortcut("w", modifiers: .command)
            }
        }
    }
}
