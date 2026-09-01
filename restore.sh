#!/usr/bin/env bash
# restore.sh — Recover the girsa personal layer from GitHub.
#
# The personal/ directory holds the one thing no command can rebuild
# (spec.md §11): shelf arrangement, scans, OCR trained data, vector
# embeddings, editorial corrections, user-added seforim, and exports.
#
# Usage:
#   From a fresh clone of girsa, personal/ is already there:
#
#     git clone https://github.com/SYKhayyat/girsa.git
#     cd girsa
#     ls personal/          # shelf.json, scans.json, tessdata/, works/, etc.
#
#   If personal/ is missing (old clone, or was deleted):
#
#     git fetch origin
#     git checkout origin/main -- personal/
#
#   To pull latest personal changes without touching the rest of the tree:
#
#     git pull origin main -- personal/
#
# What's in here:
#
#   personal/
#   ├── corrections.jsonl    editorial corrections (rare-word fixes)
#   ├── links.jsonl          cross-reference links between seforim
#   ├── scans.json           scan metadata
#   ├── shelf.json           shelf arrangement (which seforim, where)
#   ├── suspects.jsonl       auto-detected rare-word candidates (~5 MB)
#   ├── exports/             generated docx files
#   ├── files/               user-provided PDFs and .ksav files
#   ├── ksav/                ksav session files
#   ├── lane/                vector embeddings and settings
#   ├── tessdata/            Hebrew OCR trained data (3.7 MB)
#   ├── words/               word-level OCR results
#   └── works/               user-added seforim and note works
#
# Total: ~22 MB, 26 files.
#
# This directory was force-added to git on 2026-08-31 because it was the
# one gitignored thing that would be an irreversible loss on delete.
# The .gitignore no longer excludes it — future changes track normally.
