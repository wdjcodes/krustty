# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/wdjcodes/krustty/compare/v0.1.0...v0.1.1) - 2026-04-30

### Added

- handle scroll events and render scrollback buffer
- load character glyph on cache miss

### Fixed

- align glyphs when rasterizing to atlas

### Other

- application owns glyph cache and atlas texture
- create a single reference to the gpu for the application

## [0.1.0](https://github.com/wdjcodes/krustty/compare/v0.0.0...v0.1.0) - 2026-04-15

### Added

- add supprot for arrow keys, and key repetition
- tracks cursor position during resize and handles A properly ([#40](https://github.com/wdjcodes/krustty/pull/40))
- render cursor ([#38](https://github.com/wdjcodes/krustty/pull/38))
- grid resizes and reflows lines with window resize ([#36](https://github.com/wdjcodes/krustty/pull/36))
- set terminal grid size based on initial window size
- add support for standard ansi colors
- expand support for ansi crusor movement ([#29](https://github.com/wdjcodes/krustty/pull/29))
- read basic keyboard input from winit window event ([#28](https://github.com/wdjcodes/krustty/pull/28))
- add basic ansi parsing
- add basics for rendering from grid ([#26](https://github.com/wdjcodes/krustty/pull/26))
- add dynamic font loading
- add basic font rendering
- add basic Grid structs ([#18](https://github.com/wdjcodes/krustty/pull/18))
- add ui window to krustty ([#12](https://github.com/wdjcodes/krustty/pull/12))
- krustty spawns tty and sends input and reads output

### Fixed

- prevent grid and pty resize unless number of cells changes
- bug in move cursor down routine
- reflow now trims empty whitespace in wrapped lines
- zsh PROMPT_SP by deferring soft line wraps
- fix bug causing resize crash on shrinking window

### Other

- refactor to help reduce flickering when backspacing
