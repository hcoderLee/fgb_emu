import 'dart:io';
import 'dart:ffi' as ffi;

import 'native_binding.dart';

class FFIBinding {
  FFIBinding._();

  static NativeBinding? _binding;

  static NativeBinding get binding => _binding ??= NativeBinding(_load());

  /// Load native lib according to different platforms
  static ffi.DynamicLibrary _load() {
    if (Platform.isIOS) {
      return ffi.DynamicLibrary.process();
    }
    const libName = 'libgb_emu.so';
    return ffi.DynamicLibrary.open(libName);
  }
}
