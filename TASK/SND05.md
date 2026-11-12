# SND05 — Sound: DMA ring, PWM jack backend and XBIOS 0x80-0x8D

Status: OPEN (2026-11-12) | Priority: P2

Type: developer. From [SND01](SND01.md), decisions 2026-11-12. Starts after [SND03](SND03.md) (measured clock,
FIFO, cache, channels) and [SND04](SND04.md) (oracle semantics); Q-N5 (`_SND`) and Q-N6 (rates) are decided from
SND03 first.

## Facts

- Spec: `docs/api/xbios/_dmasound-group.md` (incl. §9.1 end-of-buffer without MFP); hardware
  `docs/hardware/pwm-audio.md` (its `Setbuffer` -> `SOURCE_AD` mapping is wrong: the DMA source is a CPU-converted
  staging ring of duty-cycle words; DMA-lite channels move at most 65535 bytes per control block).
- Decisions: D-tick transport first; `Setinterrupt` recorded, never called, warning; native `SndBufPtr` layout;
  big-endian 16-bit samples by default, a switch for native little-endian samples (form settled in the spec first,
  SND01 Q-N4).

## Work

1. DMA driver (BCM2711 datasheet chapter 4): channel from the SND03 H4 result, control-block ring, cache handling
   per H3.
2. PWM jack backend: clock per H1, GPIO 40/41 ALT0, conversion of 8/16-bit big-endian samples into duty-cycle words
   in the 200 Hz tick.
3. XBIOS 0x80-0x8D per spec; `_SND` cookie per the ADR-012 amendment (owner decision on the draft in SND01).
4. Host tests per function (documented return, documented error); oracle cases of SND04 equal or listed in a
   COMPATIBILITY entry.

## Acceptance

- A program plays a buffer audibly at the jack on the real Pi (owner); SND04 cases equal or accepted; `bios-r1`
  230-235 equal or accepted; `cargo xtask test` green; warnings delta 0; doc-check 0 failures.
