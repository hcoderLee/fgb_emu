import 'package:fgb_emu/src/native/gb_emulator.dart';
import 'package:flutter/material.dart';
import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';

/// Gameboy screen
class GbLCD extends StatefulWidget {
  final GbEmulator emulator;

  const GbLCD({
    Key? key,
    required this.emulator,
  }) : super(key: key);

  @override
  State<GbLCD> createState() => _GbLCDState();
}

class _GbLCDState extends State<GbLCD> with SingleTickerProviderStateMixin {
  late final _WindowBufferUpdater _bufferUpdater;

  @override
  void initState() {
    super.initState();
    _bufferUpdater = _WindowBufferUpdater(this, emulator: widget.emulator);
    _bufferUpdater.start();
  }

  @override
  Widget build(BuildContext context) {
    final width = widget.emulator.windowWidth;
    final height = widget.emulator.windowHeight;
    return SizedBox(
      width: width,
      height: height,
      child: CustomPaint(
        isComplex: true,
        willChange: true,
        painter: _LCD(emulator: widget.emulator),
      ),
    );
  }

  @override
  void dispose() {
    _bufferUpdater.dispose();
    super.dispose();
  }
}

class _LCD extends CustomPainter {
  final GbEmulator emulator;

  _LCD({
    required this.emulator,
  }) : super(repaint: emulator.windowBufferNotifier);

  @override
  void paint(ui.Canvas canvas, ui.Size size) {
    final image = emulator.buffer;
    if (image != null) {
      canvas.drawImage(image, Offset.zero, Paint());
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) {
    return true;
  }
}

/// The timer which notify emulator screen to update
class _WindowBufferUpdater {
  final GbEmulator emulator;
  final TickerProvider _vsync;
  late final Ticker _ticker;

  _WindowBufferUpdater(
    this._vsync, {
    required this.emulator,
  }) {
    _ticker = _vsync.createTicker(_onTick);
  }

  void start() {
    if (!_ticker.isActive) {
      _ticker.start();
    }
  }

  void pause() {
    _ticker.muted = true;
  }

  void resume() {
    _ticker.muted = false;
  }

  void _onTick(Duration elapsed) {
    emulator.updateWindowBuffer();
  }

  void dispose() {
    _ticker.stop();
    _ticker.dispose();
  }
}
