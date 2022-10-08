#import "FgbEmuPlugin.h"
#if __has_include(<fgb_emu/fgb_emu-Swift.h>)
#import <fgb_emu/fgb_emu-Swift.h>
#else
// Support project import fallback if the generated compatibility header
// is not copied when this plugin is created as a library.
// https://forums.swift.org/t/swift-static-libraries-dont-copy-generated-objective-c-header/19816
#import "fgb_emu-Swift.h"
#endif

@implementation FgbEmuPlugin
+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar>*)registrar {
  [SwiftFgbEmuPlugin registerWithRegistrar:registrar];
}
@end
