// 用 CoreGraphics 画一个简单的纯线条「提交图」图标(暗色 squircle + teal 线条)。
// 用法: swift scripts/GenerateIcon.swift <输出 PNG 路径>
import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

let S = 1024
let cs = CGColorSpaceCreateDeviceRGB()
guard let ctx = CGContext(data: nil, width: S, height: S, bitsPerComponent: 8,
                          bytesPerRow: 0, space: cs,
                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
    fatalError("no context")
}

func color(_ r: Int, _ g: Int, _ b: Int, _ a: CGFloat = 1) -> CGColor {
    CGColor(colorSpace: cs, components: [CGFloat(r) / 255, CGFloat(g) / 255, CGFloat(b) / 255, a])!
}
let bodyColor = color(0x1b, 0x1e, 0x24)
let border    = color(0x2c, 0x2f, 0x36)
let teal      = color(0x3f, 0xb6, 0xa8)

// 暗色 squircle 主体
let inset: CGFloat = 84
let body = CGRect(x: inset, y: inset, width: CGFloat(S) - 2 * inset, height: CGFloat(S) - 2 * inset)
let bodyPath = CGPath(roundedRect: body, cornerWidth: 196, cornerHeight: 196, transform: nil)
ctx.addPath(bodyPath); ctx.setFillColor(bodyColor); ctx.fillPath()
ctx.addPath(bodyPath); ctx.setStrokeColor(border); ctx.setLineWidth(6); ctx.strokePath()

// 提交图线条(纯线条)
ctx.setStrokeColor(teal)
ctx.setLineWidth(26)
ctx.setLineCap(.round)
ctx.setLineJoin(.round)

let lx: CGFloat = 408   // 主干 lane
let rx: CGFloat = 616   // 分支 lane

// 主干竖线
ctx.move(to: CGPoint(x: lx, y: 252))
ctx.addLine(to: CGPoint(x: lx, y: 772))
ctx.strokePath()

// 分支岔出(上)
ctx.move(to: CGPoint(x: lx, y: 636))
ctx.addCurve(to: CGPoint(x: rx, y: 512),
             control1: CGPoint(x: lx, y: 556), control2: CGPoint(x: rx, y: 600))
ctx.strokePath()

// 分支并回(下)
ctx.move(to: CGPoint(x: rx, y: 512))
ctx.addCurve(to: CGPoint(x: lx, y: 388),
             control1: CGPoint(x: rx, y: 424), control2: CGPoint(x: lx, y: 468))
ctx.strokePath()

// 节点(空心圆:先填主体色遮住线,再描边)
func node(_ x: CGFloat, _ y: CGFloat, r: CGFloat = 56) {
    let rect = CGRect(x: x - r, y: y - r, width: 2 * r, height: 2 * r)
    ctx.addEllipse(in: rect); ctx.setFillColor(bodyColor); ctx.fillPath()
    ctx.addEllipse(in: rect); ctx.setStrokeColor(teal); ctx.setLineWidth(26); ctx.strokePath()
}
node(lx, 724)   // 顶部
node(rx, 512)   // 分支
node(lx, 300)   // 底部

// 写 PNG
guard let img = ctx.makeImage() else { fatalError("no image") }
let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "AppIcon.png"
let outURL = URL(fileURLWithPath: outPath)
guard let dest = CGImageDestinationCreateWithURL(outURL as CFURL, UTType.png.identifier as CFString, 1, nil) else {
    fatalError("no dest")
}
CGImageDestinationAddImage(dest, img, nil)
CGImageDestinationFinalize(dest)
print("wrote \(outURL.path)")
