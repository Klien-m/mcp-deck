// 用 macOS 自带的 AppKit 生成 Retina 背景；执行方法见 README。
#import <AppKit/AppKit.h>

static const CGFloat Width = 660;
static const CGFloat Height = 420;

static NSColor *Gray(CGFloat value) {
    return [NSColor colorWithSRGBRed:value green:value blue:value alpha:1];
}

static void Text(NSString *value, CGFloat top, CGFloat size, NSFontWeight weight, NSColor *color) {
    NSMutableParagraphStyle *paragraph = [[NSMutableParagraphStyle alloc] init];
    paragraph.alignment = NSTextAlignmentCenter;
    [value drawInRect:NSMakeRect(32, Height - top - size * 1.6, Width - 64, size * 1.6)
        withAttributes:@{
            NSFontAttributeName: [NSFont systemFontOfSize:size weight:weight],
            NSForegroundColorAttributeName: color,
            NSParagraphStyleAttributeName: paragraph,
        }];
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) {
            fprintf(stderr, "Usage: generate-dmg-background <output.png>\n");
            return 1;
        }
        NSBitmapImageRep *bitmap = [[NSBitmapImageRep alloc]
            initWithBitmapDataPlanes:NULL pixelsWide:Width * 2 pixelsHigh:Height * 2
            bitsPerSample:8 samplesPerPixel:4 hasAlpha:YES isPlanar:NO
            colorSpaceName:NSDeviceRGBColorSpace bytesPerRow:0 bitsPerPixel:0];
        NSGraphicsContext *graphics = [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
        if (!bitmap || !graphics) {
            fprintf(stderr, "Cannot create DMG background canvas\n");
            return 1;
        }
        // 写入逻辑尺寸：1320 × 840 像素对应 660 × 420 点，Finder 不会放大背景。
        bitmap.size = NSMakeSize(Width, Height);
        [NSGraphicsContext saveGraphicsState];
        NSGraphicsContext.currentContext = graphics;
        CGContextScaleCTM(graphics.CGContext, 2, 2);

        NSGradient *gradient = [[NSGradient alloc] initWithStartingColor:Gray(0.985) endingColor:Gray(0.945)];
        [gradient drawInRect:NSMakeRect(0, 0, Width, Height) angle:-90];
        Text(@"安装 MCP Deck", 45, 30, NSFontWeightSemibold, Gray(0.14));
        Text(@"将左侧图标拖入「应用程序」即可完成安装", 94, 14, NSFontWeightRegular, Gray(0.43));

        // 原生图标由 Finder 绘制；箭头与 tauri.conf.json 中的图标中心对齐。
        CGFloat arrowY = Height - 230;
        NSBezierPath *arrow = [NSBezierPath bezierPath];
        arrow.lineWidth = 2.5;
        arrow.lineCapStyle = NSLineCapStyleRound;
        arrow.lineJoinStyle = NSLineJoinStyleRound;
        [arrow moveToPoint:NSMakePoint(315, arrowY + 12)];
        [arrow lineToPoint:NSMakePoint(327, arrowY)];
        [arrow lineToPoint:NSMakePoint(315, arrowY - 12)];
        [arrow moveToPoint:NSMakePoint(333, arrowY + 12)];
        [arrow lineToPoint:NSMakePoint(345, arrowY)];
        [arrow lineToPoint:NSMakePoint(333, arrowY - 12)];
        [Gray(0.62) setStroke];
        [arrow stroke];

        NSBezierPath *divider = [NSBezierPath bezierPath];
        divider.lineWidth = 0.5;
        [divider moveToPoint:NSMakePoint(48, Height - 344)];
        [divider lineToPoint:NSMakePoint(Width - 48, Height - 344)];
        [Gray(0.84) setStroke];
        [divider stroke];
        Text(@"安装完成后，请从「应用程序」中打开 MCP Deck", 365, 12, NSFontWeightRegular, Gray(0.46));
        [NSGraphicsContext restoreGraphicsState];

        NSData *png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
        NSError *error = nil;
        if (!png || ![png writeToFile:[NSString stringWithUTF8String:argv[1]] options:NSDataWritingAtomic error:&error]) {
            fprintf(stderr, "Cannot write DMG background: %s\n", error.localizedDescription.UTF8String ?: "PNG encoding failed");
            return 1;
        }
        printf("Generated %s (1320 x 840 px, 660 x 420 pt)\n", argv[1]);
    }
    return 0;
}
