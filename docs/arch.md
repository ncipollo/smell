# Architecture

The code is broken down into three layers:

## cli

The view layer, built with clap. The `cli` module contains a top-level router, which `main` calls. Each command gets its own file and is called from the router. No actual logic lives in this layer — commands call through to the feature layer.

## feature

Where all domain logic lives. The cli layer calls through to feature.

## code

Interaction with the tree-sitter libraries. Parser setup and related concerns are organized by language type.
