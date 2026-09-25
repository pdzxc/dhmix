// Renders the DHMIX app icon: the "DH" badge from the top bar, 256 px, as raw RGBA plus a PNG
// for reference. Run from the repo root on macOS: `swift assets/make_icon.swift`.
import AppKit
import CoreText

let size = 256
let badge = NSColor(red: 0xff / 255.0, green: 0x9b / 255.0, blue: 0x3d / 255.0, alpha: 1) // COLOR_PLAYER (orange)
let ink = NSColor.white

let space = CGColorSpaceCreateDeviceRGB()
let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4, space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
let rect = CGRect(x: 0, y: 0, width: size, height: size).insetBy(dx: 8, dy: 8)
ctx.addPath(CGPath(roundedRect: rect, cornerWidth: 56, cornerHeight: 56, transform: nil))
ctx.setFillColor(badge.cgColor)
ctx.fillPath()

let font = CTFontCreateWithName("HelveticaNeue-Bold" as CFString, 124, nil)
let text = NSAttributedString(string: "DH", attributes: [.font: font, .foregroundColor: ink])
let line = CTLineCreateWithAttributedString(text)
let bounds = CTLineGetBoundsWithOptions(line, [.useGlyphPathBounds])
ctx.textPosition = CGPoint(x: (CGFloat(size) - bounds.width) / 2 - bounds.minX, y: (CGFloat(size) - bounds.height) / 2 - bounds.minY)
CTLineDraw(line, ctx)

let image = ctx.makeImage()!
let png = NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])!
try! png.write(to: URL(fileURLWithPath: "assets/icon-256.png"))

// Un-premultiply into straight RGBA, top row first, as egui's IconData expects.
let data = ctx.data!.bindMemory(to: UInt8.self, capacity: size * size * 4)
var rgba = [UInt8](repeating: 0, count: size * size * 4)
for i in 0..<(size * size) {
    let a = Int(data[i * 4 + 3])
    for c in 0..<3 {
        let v = Int(data[i * 4 + c])
        rgba[i * 4 + c] = a == 0 ? 0 : UInt8(min(255, v * 255 / a))
    }
    rgba[i * 4 + 3] = UInt8(a)
}
try! Data(rgba).write(to: URL(fileURLWithPath: "assets/icon-256.rgba"))
print("wrote assets/icon-256.png and assets/icon-256.rgba")
