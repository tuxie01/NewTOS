# GEMDOS API Specification

This document specifies the GEMDOS (`trap #1`) syscalls implemented by
NewTOS. Functions are listed in numerical order.

Implementation status is tracked in `docs/STATUS.md`, not here. This
document is the **specification**, not the progress report.

For format conventions, see `_template.md`.

---

## Function: `Cconin` (0x01)

### Purpose

Read a single character from the standard input device (CON:), with echo,
blocking until a character is available.

### Trap

- Trap number: 1 (GEMDOS)
- Function ID: 0x01

### Parameters

None.

### Return value

- Type: `LONG`
- Bits 0–7: ASCII value of the character read
- Bits 16–23: keyboard scancode
- Bits 8–15, 24–31: zero
- On Ctrl-C with default settings: returns the character normally; Ctrl-C
  handling is performed by `Cnecin`, not by `Cconin`.

### Error codes

None. The function blocks until input is available.

### m68k ABI mapping

```
move.w   #$01, -(SP)
trap     #1
addq.l   #2, SP
; result in D0.l
```

### AArch64 ABI mapping

```aarch64
mov     x8, #0x01
svc     #1
// result in x0 (32-bit value, zero-extended)
```

### Side effects

- The character is echoed to the standard output device (CON:), unless
  the character is non-printable (e.g., function keys).

### Pre-conditions

- Console input device is initialized.

### Post-conditions

- One character has been consumed from the input stream.
- The character has been echoed to output (if printable).

### Quirks and version differences

- TOS 1.x echoes only ASCII printable range; control characters are not
  echoed. NewTOS follows this.
- The scancode in the upper word is essential for distinguishing keys
  that produce the same ASCII (e.g., Numpad-0 vs main-keyboard 0).

### NewTOS-specific decisions

- The scancode is taken from the USB HID keymap currently in effect (Phase
  TBD). Until USB HID is implemented, scancodes are zero and only the
  ASCII byte from UART input is returned.

### Reference

- Atari Compendium, "Cconin"
- Profibuch, "GEMDOS-Funktion 1"
- EmuTOS source: `bdos/console.c`, (cite stale 2026-10-26: bios/iorec.c, no such file in EmuTOS)

---

## Function: `Cconout` (0x02)

### Purpose

Write a single character to the standard output device (CON:).

### Trap

- Trap number: 1
- Function ID: 0x02

### Parameters

| Name | Type   | Description           |
|------|--------|-----------------------|
| `c`  | `WORD` | Character to write    |

### Return value

- Type: `LONG`
- Always returns 0. (Some sources say it returns the character; Compendium
  says it returns "the function code" — i.e., 2. NewTOS returns 0 to match
  EmuTOS behavior, which Compendium notes as the most common observed
  behavior.)

### Error codes

None directly. Output device errors are silently ignored (this is the TOS
convention; deviating breaks programs that don't check return).

### m68k ABI mapping

```
move.w   #c, -(SP)
move.w   #$02, -(SP)
trap     #1
addq.l   #4, SP
```

### AArch64 ABI mapping

```aarch64
mov     w0, w_char
mov     x8, #0x02
svc     #1
```

### Side effects

- One character written to console output.
- VT52 escape sequence interpretation (cursor positioning, clear, etc.)
  is performed by the console driver per Compendium "VT52 emulation".

### Pre-conditions

- Console output device is initialized.

### Post-conditions

- The character has been written or has begun being written. The function
  may return before the UART has physically transmitted the byte.

### Quirks and version differences

- VT52 emulation mode is the TOS standard; NewTOS implements the documented
  subset (cursor, clear, attribute control). The full set is in Compendium
  appendix.

### NewTOS-specific decisions

- The return value is 0 (matching EmuTOS), not 2 (as some early Compendium
  prints state).

### Reference

- Atari Compendium, "Cconout"
- Profibuch, "GEMDOS-Funktion 2"
- EmuTOS source: `bdos/conout.c:bdos_2()`

---

## Function: `Cconws` (0x09)

### Purpose

Write a zero-terminated string to the standard output device (CON:).

### Trap

- Trap number: 1
- Function ID: 0x09

### Parameters

| Name | Type     | Description                          |
|------|----------|--------------------------------------|
| `s`  | `char *` | Pointer to NUL-terminated string     |

### Return value

- Type: `LONG`
- Number of characters written. (Compendium says "function ID returned",
  but EmuTOS and most TOS versions return the character count.)

### Error codes

None.

### m68k ABI mapping

```
pea      string
move.w   #$09, -(SP)
trap     #1
addq.l   #6, SP
```

### AArch64 ABI mapping

```aarch64
mov     x0, x_string_ptr
mov     x8, #0x09
svc     #1
```

### Side effects

- All characters of the string written via `Cconout` semantics.
- VT52 escape interpretation applies.

### Pre-conditions

- `s` points to a valid NUL-terminated string.
- For m68k callers: `s` is a valid m68k pointer; the marshaling layer
  translates and provides UTF-8/ATARIST mapping.

### Post-conditions

- All characters before the terminating NUL have been written.

### Quirks and version differences

- Behavior on `s == NULL`: Compendium says undefined. NewTOS returns 0
  without writing anything.

### NewTOS-specific decisions

- `s == NULL` is treated as empty string (returns 0).
- Atari character set translation: NewTOS's console outputs the byte as-is
  (no Atari→Unicode translation in this layer; that belongs in the GEM
  font subsystem in Phase 12).

### Reference

- Atari Compendium, "Cconws"
- EmuTOS source: `bdos/conout.c:bdos_9()`

---

<!-- Continue with Cauxin, Cauxout, Cprnout, Crawio, Crawcin, Cnecin,
     Cconrs, Cconis when starting Phase 02 -->
