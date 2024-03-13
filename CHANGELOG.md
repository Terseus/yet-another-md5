# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v2.0.0] - 2024-03-13

### Changed

- Replace the old `Md5Hasher` API methods `add_chunk` and `compute` with the more widespread `update` and `finalize`.

### Removed

- Remove simplelog dependency.
- Remove serial_test and criterion dev dependencies.

## [v1.0.0] - 2024-02-17

### Added

- First public version.
