#include <stdint.h>

typedef struct Emulator Emulator_C;

typedef enum
{
    LEFT = 0x01,
    UP = 0x02,
    RIGHT = 0x04,
    DOWN = 0x08,
    A = 0x10,
    B = 0x20,
    START = 0x40,
    SELECT = 0x80,
} GbBtn;

typedef struct
{
    float scale_factor;
} WindowConfig;

Emulator_C *create_emulator(WindowConfig *win_config);

void run_emulator(Emulator_C *emulator, char *rom_path);

uint32_t *get_window_buffer(Emulator_C *emulator);

void press_button(Emulator_C *emulator, GbBtn btn);

void release_button(Emulator_C *emulator, GbBtn btn);

void pause_emulator(Emulator_C *emulator);

void resume_emulator(Emulator_C *emulator);

void exit_emulator(Emulator_C *emulator);