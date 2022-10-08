import 'dart:async';
import 'dart:ffi' as ffi;
import 'dart:io';
import 'dart:ui' as ui;

import 'package:ffi/ffi.dart';
import 'package:flutter/material.dart';

import 'gb_native_binding.dart';

typedef _NativeEmulator = ffi.Pointer<Emulator_C>;

const int gbWindowWidth = 160;
const int gbWindowHeight = 144;

enum GbButton {
  left(GbBtn.LEFT),
  up(GbBtn.UP),
  right(GbBtn.RIGHT),
  down(GbBtn.DOWN),
  A(GbBtn.A),
  B(GbBtn.B),
  select(GbBtn.SELECT),
  start(GbBtn.START);

  final int val;

  const GbButton(this.val);
}

class GbEmulator {
  final String gamePath;

  /// Native implemented emulator
  late final _NativeEmulator _emulator;

  /// The width of emulator's screen
  late final double _windowWidth;

  late final GbWindowBufferNotifier _notifier;

  late final ffi.Pointer<WindowConfig> _windowConfig;

  GbWindowBufferNotifier get windowBufferNotifier => _notifier;

  double get windowWidth => _windowWidth;

  /// The height of emulator's screen
  late final double _windowHeight;

  double get windowHeight => _windowHeight;

  /// The emulator screen scale factor according to original size
  late final int _windowScaleFactor;

  late final GbNativeBinding _binding;

  /// Emulator screen pixels data, will get update each frame
  ui.Image? _buffer;

  ui.Image? get buffer => _buffer;

  GbEmulator({
    required this.gamePath,
  }) {
    _notifier = GbWindowBufferNotifier();
    // Load native lib
    _binding = GbNativeBinding(_load());
    // Calculate window scale factor, width and height
    final screenWidth =
        MediaQueryData.fromWindow(WidgetsBinding.instance.window).size.width;
    _windowScaleFactor = (screenWidth / gbWindowWidth).floor();
    _windowWidth = gbWindowWidth.toDouble() * _windowScaleFactor;
    _windowHeight = gbWindowHeight.toDouble() * _windowScaleFactor;
    _windowConfig = malloc<WindowConfig>();
    _windowConfig.ref.scale_factor = _windowScaleFactor.toDouble();
    // Create emulator
    _emulator = _binding.create_emulator(_windowConfig);
  }

  /// Load native lib according to different platforms
  ffi.DynamicLibrary _load() {
    if (Platform.isIOS) {
      return ffi.DynamicLibrary.process();
    }
    const libName = 'libgb_emu.so';
    return ffi.DynamicLibrary.open(libName);
  }

  void run() {
    _binding.run_emulator(
      _emulator,
      gamePath.toNativeUtf8().cast(),
    );
  }

  void pause() {
    _binding.pause_emulator(_emulator);
  }

  void resume() {
    _binding.resume_emulator(_emulator);
  }

  void exit() {
    _binding.exit_emulator(_emulator);
    _buffer?.dispose();
    malloc.free(_windowConfig);
  }

  void pressButton(GbButton button) {
    _binding.press_button(_emulator, button.val);
  }

  void releaseButton(GbButton button) {
    _binding.release_button(_emulator, button.val);
  }

  void updateWindowBuffer() async {
    final c = Completer<ui.Image>();
    final bufferSize = (_windowWidth * _windowHeight).toInt();
    final pixels =
        _binding.get_window_buffer(_emulator).asTypedList(bufferSize);
    void decodeCallback(ui.Image image) {
      c.complete(image);
    }

    ui.decodeImageFromPixels(
      pixels.buffer.asUint8List(),
      _windowWidth.toInt(),
      _windowHeight.toInt(),
      ui.PixelFormat.rgba8888,
      decodeCallback,
    );
    final newBuffer = await c.future;
    _buffer = newBuffer;
    _notifier.notifyBufferUpdate();
  }
}

/// Used to notifiy window buffer change
class GbWindowBufferNotifier extends ChangeNotifier {
  void notifyBufferUpdate() {
    notifyListeners();
  }
}
