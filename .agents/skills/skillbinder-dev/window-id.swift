// Prints the CoreGraphics ID of a process's largest normal window, for `screencapture -l<id>`.
// Usage: swift window-id.swift <pid>
import CoreGraphics
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

guard CommandLine.arguments.count == 2, let pid = Int(CommandLine.arguments[1]) else {
    fail("usage: swift window-id.swift <pid>")
}
let windows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
func area(_ window: [String: Any]) -> Double {
    let bounds = window[kCGWindowBounds as String] as? [String: Double] ?? [:]
    return (bounds["Width"] ?? 0) * (bounds["Height"] ?? 0)
}
let largest = windows
    .filter { $0[kCGWindowOwnerPID as String] as? Int == pid && $0[kCGWindowLayer as String] as? Int == 0 }
    .max { area($0) < area($1) }
guard let id = largest?[kCGWindowNumber as String] as? Int else {
    fail("no window for process \(pid)")
}
print(id)
