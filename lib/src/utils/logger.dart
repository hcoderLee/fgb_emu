import 'dart:ffi';
import 'dart:ffi' as ffi;
import 'dart:isolate';

import 'package:fgb_emu/src/ffi/ffi_binding.dart';
import 'package:flutter/foundation.dart';

/// Rust code could send log message to flutter, this class will config how to
/// handle log message
class FLogger {
  static final RawReceivePort _receivePort = RawReceivePort();
  static bool _hasInitialized = false;

  /// It should be called only once, before running native emulator function
  static void init() {
    if (kReleaseMode || _hasInitialized) {
      return;
    }
    // Bind handler function to print. When native code sent log message, it will
    // print to console
    _receivePort.handler = debugPrint;
    // Call native init_logger function
    FFIBinding.binding.init_logger(
      _receivePort.sendPort.nativePort,
      NativeApi.postCObject.cast(),
    );
    _hasInitialized = true;
  }
}
