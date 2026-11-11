// Draws Vista's app icon: a deep indigo squircle with three stacked windows,
// the front one showing a little screen. Usage: swift icon.swift <out.png> <size>
import AppKit

let args = CommandLine.arguments
let out = args.count > 1 ? args[1] : "icon.png"
let size = CGFloat(args.count > 2 ? Double(args[2]) ?? 1024 : 1024)

let image = NSImage(size: NSSize(width: size, height: size))
image.lockFocus()
let ctx = NSGraphicsContext.current!.cgContext
let s = size / 1024

// The squircle, inset like every macOS icon.
let tile = CGRect(x: 100 * s, y: 100 * s, width: 824 * s, height: 824 * s)
let shape = CGPath(roundedRect: tile, cornerWidth: 185 * s, cornerHeight: 185 * s, transform: nil)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -10 * s), blur: 28 * s, color: NSColor.black.withAlphaComponent(0.35).cgColor)
ctx.addPath(shape)
ctx.setFillColor(NSColor.black.cgColor)
ctx.fillPath()
ctx.restoreGState()

ctx.saveGState()
ctx.addPath(shape)
ctx.clip()
let colors = [NSColor(srgbRed: 0.36, green: 0.40, blue: 0.97, alpha: 1).cgColor,
              NSColor(srgbRed: 0.16, green: 0.13, blue: 0.48, alpha: 1).cgColor] as CFArray
let gradient = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB), colors: colors, locations: [0, 1])!
ctx.drawLinearGradient(gradient, start: CGPoint(x: 0, y: 924 * s), end: CGPoint(x: 0, y: 100 * s), options: [])
ctx.restoreGState()

// Three windows, back to front.
func window(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, alpha: CGFloat, front: Bool) {
  let rect = CGRect(x: x * s, y: y * s, width: w * s, height: h * s)
  let path = CGPath(roundedRect: rect, cornerWidth: 44 * s, cornerHeight: 44 * s, transform: nil)
  ctx.saveGState()
  if front {
    ctx.setShadow(offset: CGSize(width: 0, height: -14 * s), blur: 40 * s, color: NSColor.black.withAlphaComponent(0.4).cgColor)
  }
  ctx.addPath(path)
  ctx.setFillColor(NSColor.white.withAlphaComponent(alpha).cgColor)
  ctx.fillPath()
  ctx.restoreGState()
  if front {
    // Title bar dots and a screen.
    for (i, c) in [NSColor.systemRed, NSColor.systemYellow, NSColor.systemGreen].enumerated() {
      ctx.setFillColor(c.cgColor)
      ctx.fillEllipse(in: CGRect(x: (x + 36 + CGFloat(i) * 40) * s, y: (y + h - 58) * s, width: 24 * s, height: 24 * s))
    }
    let screen = CGRect(x: (x + 30) * s, y: (y + 30) * s, width: (w - 60) * s, height: (h - 110) * s)
    let screenPath = CGPath(roundedRect: screen, cornerWidth: 22 * s, cornerHeight: 22 * s, transform: nil)
    ctx.saveGState()
    ctx.addPath(screenPath)
    ctx.clip()
    let sky = [NSColor(srgbRed: 0.20, green: 0.24, blue: 0.62, alpha: 1).cgColor,
               NSColor(srgbRed: 0.45, green: 0.55, blue: 1.0, alpha: 1).cgColor] as CFArray
    let fill = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB), colors: sky, locations: [0, 1])!
    ctx.drawLinearGradient(fill, start: CGPoint(x: screen.minX, y: screen.minY), end: CGPoint(x: screen.maxX, y: screen.maxY), options: [])
    ctx.restoreGState()
  }
}
window(330, 470, 440, 300, alpha: 0.28, front: false)
window(280, 400, 480, 320, alpha: 0.5, front: false)
window(220, 250, 560, 390, alpha: 1.0, front: true)

image.unlockFocus()
let rep = NSBitmapImageRep(data: image.tiffRepresentation!)!
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
