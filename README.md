<h1 align="center">
  <br>
    <img 
      src="https://github.com/cryptopatrick/factory/blob/master/img/100days/harel_scxml.png" 
      alt="Title" 
      width="200"
    />
  <br>
  HAREL
  <br>
</h1>

<h4 align="center">
  Rust implementation of 
  <a href="https://en.wikipedia.org/wiki/State_diagram#Harel_statechart" target="_blank">
    Harel Statecharts</a> parser.</h4>

<p align="center">
  <a href="https://crates.io/crates/harel" target="_blank">
    <img src="https://img.shields.io/crates/v/harel" alt="Crates.io"/>
  </a>
  <a href="https://crates.io/crates/harel" target="_blank">
    <img src="https://img.shields.io/crates/d/harel" alt="Downloads"/>
  </a>
  <a href="https://docs.rs/harel" target="_blank">
    <img src="https://docs.rs/harel/badge.svg" alt="Documentation"/>
  </a>
  <a href="LICENSE" target="_blank">
    <img src="https://img.shields.io/github/license/cp/harel.svg" alt="GitHub license"/>
  </a>
</p>

<p align="center">
  <a href="#-what-is-harel">What is Harel</a> •
  <a href="#-features">Features</a> •
  <a href="#-how-to-use">How To Use</a> •
  <a href="#-documentation">Documentation</a> •
  <a href="#-license">License</a>
</p>

<!-- TABLE OF CONTENTS -->
<h2 id="table-of-contents"> :pushpin: Table of Contents</h2>

<details open="open">
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#-what-is-harel"> What is Harel</a></li>
    <li><a href="#-features"> Features</a></li>
    <li><a href="#-how-to-use"> How to Use</a></li>
    <li><a href="#-documentation"> Documentation</a></li>
    <li><a href="#-license">License</a></li>
  </ol>
</details>

## 🤔 What is Harel

`harel` is a Rust library that provides a parser for Harel Statecharts, a powerful extension of finite state machines that supports hierarchical (nested) states, concurrent states, and complex state transitions. Harel Statecharts were introduced by David Harel in 1987 as a visual formalism for describing the behavior of complex reactive systems.

### Use Cases

- **Embedded Systems**: Model complex device behaviors and state machines
- **Game Development**: Implement AI behaviors and game state management
- **Protocol Implementation**: Design and validate communication protocols
- **Workflow Systems**: Model business processes and workflow logic
- **Reactive Systems**: Handle event-driven system behaviors

### Architecture

The library provides:

1. **Parser**: Converts textual statechart descriptions into structured data
2. **AST Representation**: Abstract syntax tree for statechart elements
3. **Validation**: Ensures statechart correctness and well-formedness
4. **Traversal**: Navigate and analyze statechart structures

## 📷 Features

### Core Parsing
- **Hierarchical States**: Support for nested state structures
- **Concurrent States**: Parallel state execution modeling
- **Transitions**: State-to-state transitions with guards and actions
- **Events**: Event-driven state machine behavior

### Statechart Elements
- **States**: Simple states, composite states, and concurrent regions
- **Transitions**: Internal and external transitions with conditions
- **Guards**: Boolean conditions for transition enablement
- **Actions**: Entry, exit, and transition actions

### Validation & Analysis
- **Syntax Validation**: Ensures proper statechart syntax
- **Semantic Checks**: Validates statechart semantics and consistency
- **Structure Analysis**: Analyzes state hierarchy and relationships

## 🚙 How to Use

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
harel = "0.1"
```

Or install with cargo:

```bash
cargo add harel
```

### Example

```rust
use harel::*;

// Parse a simple statechart
let statechart_text = r#"
statechart TrafficLight {
    state Red {
        on timer -> Green
    }
    state Green {
        on timer -> Yellow
    }
    state Yellow {
        on timer -> Red
    }
}
"#;

// Parse the statechart
let parser = HarelParser::new();
let statechart = parser.parse(statechart_text)?;

// Analyze the parsed statechart
println!("Statechart name: {}", statechart.name());
println!("Number of states: {}", statechart.states().len());
```

## 📚 Documentation

Comprehensive documentation is available at [docs.rs/harel](https://docs.rs/harel), including:
- API reference for all public types and functions
- Tutorial on parsing statecharts
- Examples of different statechart patterns
- Best practices for statechart design

## 🗄 License
This project is licensed under MIT. See [LICENSE](LICENSE) for details.
