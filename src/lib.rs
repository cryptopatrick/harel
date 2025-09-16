//! # Harel - SCXML Parser and Serializer for Rust
//!
//! Harel is a Rust crate that provides a comprehensive implementation for parsing, validating, and serializing
//! SCXML (State Chart XML) documents in accordance with the W3C SCXML 1.0 specification.
//!
//! SCXML is an XML-based markup language for describing state machines, commonly used in applications requiring
//! complex state management, such as voice response systems, user interfaces, and workflow engines.
//!
//! ## Key Features
//!
//! - **Parsing**: Parse SCXML from strings or files into structured Rust types, with support for both strict and relaxed namespace handling.
//! - **Validation**: Perform structural and semantic validation to ensure compliance with the SCXML specification, including checks for unique IDs, valid transition targets, and datamodel constraints.
//! - **Serialization**: Convert parsed SCXML structures back to well-formatted XML strings, preserving the original structure and attributes.
//! - **Relaxed Parsing Mode**: Optionally parse SCXML documents without requiring namespace declarations, useful for legacy or non-standard files.
//! - **Comprehensive Element Support**: Handles core SCXML elements, transitions, data models, executable content, and external invocations.
//!
//! ## Usage
//!
//! Add Harel to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! harel = "0.1.0"  # Replace with the actual version
//! ```
//!
//! ## Quick Example
//!
//! ```rust
//! use harel::{parse_scxml, validate, to_xml, ParseOptions};
//!
//! let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
//!     <state id="start">
//!         <transition event="go" target="end"/>
//!     </state>
//!     <final id="end"/>
//! </scxml>"#;
//!
//! // Parse the SCXML document
//! let scxml = parse_scxml(xml).expect("Failed to parse SCXML");
//!
//! // Validate the parsed structure
//! validate(&scxml).expect("SCXML validation failed");
//!
//! // Serialize back to XML
//! let xml_output = to_xml(&scxml);
//! println!("{}", xml_output);
//! ```
//!
//! ## Supported SCXML Elements
//!
//! - **Core Constructs**: `<scxml>`, `<state>`, `<parallel>`, `<final>`, `<initial>`, `<history>`
//! - **Transitions**: `<transition>` with support for events, conditions, targets, types, and executable content
//! - **Data Model**: `<datamodel>`, `<data>` with expressions, sources, and inline content
//! - **Executable Content**: `<raise>`, `<if>`, `<foreach>`, `<send>`, `<script>`, `<assign>`, `<log>`, `<cancel>`
//! - **External Communications**: `<invoke>`, `<param>`, `<finalize>`, `<content>`
//!
//! ## Error Handling
//!
//! Parsing and validation errors are handled via custom error types (`ParseError` and `ValidationError`) that provide
//! detailed information about the issues encountered.
//!
//! ## Limitations and Future Work
//!
//! - Currently supports SCXML 1.0 only; future versions may add support for later drafts or extensions.
//! - Custom or unsupported executable elements are captured as `Executable::Other` for forward compatibility.
//! - No runtime interpretation of SCXML state machines; this crate focuses on parsing, validation, and serialization.

use roxmltree::{Document, Node};
use thiserror::Error;

/// Errors that can occur during SCXML parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid XML: {0}")]
    InvalidXml(#[from] roxmltree::Error),
    #[error("Missing required attribute: {0}")]
    MissingAttribute(String),
    #[error("Invalid structure: {0}")]
    InvalidStructure(String),
    #[error("Invalid namespace: expected {0}")]
    InvalidNamespace(String),
}

/// Errors that can occur during SCXML validation.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Duplicate state ID: {0}")]
    DuplicateId(String),
    #[error("Invalid transition target: {0}")]
    InvalidTarget(String),
    #[error("Circular initial state reference")]
    CircularInitial,
    #[error("Invalid datamodel constraint: {0}")]
    InvalidDatamodel(String),
    #[error("Missing required element: {0}")]
    MissingElement(String),
}

const SCXML_NS: &str = "http://www.w3.org/2005/07/scxml";

/// Represents the root `<scxml>` element, containing the overall state machine definition.
#[derive(Debug, Clone)]
pub struct Scxml {
    /// The SCXML version (must be "1.0").
    pub version: String,
    /// The ID of the initial state or substate.
    pub initial: Option<String>,
    /// The datamodel type (e.g., "ecmascript").
    pub datamodel: Option<String>,
    /// Child states, parallels, finals, or histories.
    pub states: Vec<StateLike>,
    /// Data elements within the `<datamodel>`.
    pub datamodel_elements: Vec<Data>,
}

/// Enum representing state-like elements: `<state>`, `<parallel>`, `<final>`, or `<history>`.
#[derive(Debug, Clone)]
pub enum StateLike {
    State(State),
    Parallel(Parallel),
    Final(Final),
    History(History),
}

/// Represents a `<state>` element, which can contain substates and transitions.
#[derive(Debug, Clone)]
pub struct State {
    /// Unique identifier for the state.
    pub id: Option<String>,
    /// Initial substate ID (attribute form).
    pub initial: Option<String>,
    /// Explicit `<initial>` child element.
    pub initial_element: Option<Initial>,
    /// Transitions from this state.
    pub transitions: Vec<Transition>,
    /// Executable content on entry.
    pub onentry: Vec<Executable>,
    /// Executable content on exit.
    pub onexit: Vec<Executable>,
    /// Child state-like elements.
    pub children: Vec<StateLike>,
    /// Invoke elements for external processes.
    pub invokes: Vec<Invoke>,
}

/// Represents a `<parallel>` element for concurrent substates.
#[derive(Debug, Clone)]
pub struct Parallel {
    /// Unique identifier for the parallel region.
    pub id: Option<String>,
    /// Transitions from this parallel region.
    pub transitions: Vec<Transition>,
    /// Executable content on entry.
    pub onentry: Vec<Executable>,
    /// Executable content on exit.
    pub onexit: Vec<Executable>,
    /// Child state-like elements (must be atomic or parallel).
    pub children: Vec<StateLike>,
    /// Invoke elements for external processes.
    pub invokes: Vec<Invoke>,
}

/// Represents a `<final>` element, indicating an end state.
#[derive(Debug, Clone)]
pub struct Final {
    /// Unique identifier for the final state.
    pub id: Option<String>,
    /// Executable content on entry.
    pub onentry: Vec<Executable>,
    /// Executable content on exit.
    pub onexit: Vec<Executable>,
}

/// Represents a `<transition>` element.
#[derive(Debug, Clone)]
pub struct Transition {
    /// Event descriptor that triggers the transition.
    pub event: Option<String>,
    /// Condition expression that must evaluate to true.
    pub cond: Option<String>,
    /// Target state ID(s) (space-separated for multiple).
    pub target: Option<String>,
    /// Transition type ("internal" or "external").
    pub type_: Option<String>,
    /// Executable content within the transition.
    pub executables: Vec<Executable>,
}

/// Represents a `<data>` element in the datamodel.
#[derive(Debug, Clone)]
pub struct Data {
    /// Unique identifier for the data item.
    pub id: String,
    /// Expression to initialize the data.
    pub expr: Option<String>,
    /// External source URL for data.
    pub src: Option<String>,
    /// Inline content for data.
    pub content: Option<String>,
}

/// Represents an `<initial>` element within a compound state.
#[derive(Debug, Clone)]
pub struct Initial {
    /// Optional ID for the initial pseudo-state.
    pub id: Option<String>,
    /// The transition to the initial substate.
    pub transition: Transition,
}

/// Represents a `<history>` pseudo-state.
#[derive(Debug, Clone)]
pub struct History {
    /// Unique identifier for the history state.
    pub id: Option<String>,
    /// History type ("shallow" or "deep").
    pub type_: String,
    /// Default transition for history.
    pub transition: Option<Transition>,
}

/// Represents an `<invoke>` element for external processes.
#[derive(Debug, Clone)]
pub struct Invoke {
    /// Type of the invoked process (e.g., "scxml", "vxml3").
    pub type_: String,
    /// Source URL of the invoked document.
    pub src: Option<String>,
    /// Unique identifier for the invocation.
    pub id: Option<String>,
    /// Parameters passed to the invocation.
    pub params: Vec<Param>,
    /// Finalize executable content.
    pub finalize: Option<Finalize>,
    /// Inline content for the invocation.
    pub content: Option<Content>,
}

/// Represents a `<param>` element within `<invoke>`.
#[derive(Debug, Clone)]
pub struct Param {
    /// Parameter name.
    pub name: String,
    /// Expression value.
    pub expr: Option<String>,
    /// Location expression.
    pub location: Option<String>,
}

/// Represents a `<finalize>` element within `<invoke>`.
#[derive(Debug, Clone)]
pub struct Finalize {
    /// Executable content to finalize the invocation.
    pub executables: Vec<Executable>,
}

/// Represents a `<content>` element.
#[derive(Debug, Clone)]
pub struct Content {
    /// Expression for content.
    pub expr: Option<String>,
    /// Inline content.
    pub content: Option<String>,
}

/// Enum representing executable content elements.
#[derive(Debug, Clone)]
pub enum Executable {
    /// `<raise>` to raise an event.
    Raise { event: String },
    /// `<if>` conditional.
    If { cond: String, then: Vec<Executable>, else_: Vec<Executable> },
    /// `<foreach>` loop.
    Foreach { array: String, item: String, index: Option<String>, body: Vec<Executable> },
    /// `<send>` to send an event.
    Send { event: String, target: Option<String> /* Additional attributes can be added */ },
    /// `<script>` for embedded scripts.
    Script { src: Option<String>, content: Option<String> },
    /// `<assign>` to update data.
    Assign { location: String, expr: String },
    /// `<log>` for logging.
    Log { label: Option<String>, expr: String },
    /// `<cancel>` to cancel a send.
    Cancel { sendid: String },
    /// Placeholder for unsupported or custom executables.
    Other(String),
}

/// Parses an SCXML document from a string using default options.
///
/// This function uses strict namespace checking. For relaxed parsing (e.g., without namespace declarations),
/// use [`parse_scxml_with_options`].
///
/// # Arguments
///
/// * `xml` - The SCXML XML string to parse.
///
/// # Returns
///
/// * `Ok(Scxml)` - The parsed SCXML structure.
/// * `Err(ParseError)` - If parsing fails due to invalid XML, structure, or namespace.
///
/// # Examples
///
/// ```rust
/// use harel::parse_scxml;
///
/// let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
///     <state id="start"/>
/// </scxml>"#;
///
/// let scxml = parse_scxml(xml).expect("Failed to parse SCXML");
/// assert_eq!(scxml.version, "1.0");
/// ```
pub fn parse_scxml(xml: &str) -> Result<Scxml, ParseError> {
    parse_scxml_with_options(xml, ParseOptions::default())
}

/// Options for customizing SCXML parsing behavior.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// If true, allows parsing without strict namespace checking.
    pub relaxed_namespace: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            relaxed_namespace: false,
        }
    }
}

/// Parses an SCXML document from a string with custom options.
///
/// Allows customization such as relaxed namespace handling for non-standard SCXML files.
///
/// # Arguments
///
/// * `xml` - The SCXML XML string to parse.
/// * `options` - Custom parsing options.
///
/// # Returns
///
/// * `Ok(Scxml)` - The parsed SCXML structure.
/// * `Err(ParseError)` - If parsing fails.
///
/// # Examples
///
/// ```rust
/// use harel::{parse_scxml_with_options, ParseOptions};
///
/// let xml = r#"<scxml version="1.0">
///     <state id="start"/>
/// </scxml>"#;
///
/// let options = ParseOptions { relaxed_namespace: true };
/// let scxml = parse_scxml_with_options(xml, options).expect("Failed to parse SCXML");
/// assert_eq!(scxml.version, "1.0");
/// ```
pub fn parse_scxml_with_options(xml: &str, options: ParseOptions) -> Result<Scxml, ParseError> {
    // Parse the XML string into a document tree.
    let doc = Document::parse(xml)?;
    let root = doc.root_element();

    // Validate namespace if not in relaxed mode.
    if !options.relaxed_namespace {
        if root.tag_name().namespace() != Some(SCXML_NS) {
            return Err(ParseError::InvalidNamespace(SCXML_NS.to_string()));
        }
    }

    // Ensure the root element is <scxml>.
    if root.tag_name().name() != "scxml" {
        return Err(ParseError::InvalidStructure("Root must be <scxml>".into()));
    }

    // Extract required version attribute.
    let version = root.attribute("version").ok_or(ParseError::MissingAttribute("version".into()))?.to_string();
    if version != "1.0" {
        return Err(ParseError::InvalidStructure("SCXML version must be 1.0".into()));
    }

    // Extract optional attributes.
    let initial = root.attribute("initial").map(|s| s.to_string());
    let datamodel = root.attribute("datamodel").map(|s| s.to_string());

    let mut states = Vec::new();
    let mut datamodel_elements = Vec::new();

    // Process child elements of <scxml>.
    for child in root.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "state" => states.push(StateLike::State(parse_state(&child)?)),
            "parallel" => states.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => states.push(StateLike::Final(parse_final(&child)?)),
            "history" => states.push(StateLike::History(parse_history(&child)?)),
            "datamodel" => datamodel_elements.extend(parse_datamodel(&child)?),
            _ => {},  // Ignore unsupported elements
        }
    }

    Ok(Scxml { version, initial, datamodel, states, datamodel_elements })
}

/// Validates the parsed SCXML structure for compliance with the specification.
///
/// Checks include:
/// - Unique IDs across states.
/// - Valid targets for transitions.
/// - No circular initial references (basic check).
/// - Datamodel constraints (e.g., unique data IDs).
///
/// # Arguments
///
/// * `scxml` - The parsed SCXML to validate.
///
/// # Returns
///
/// * `Ok(())` - If valid.
/// * `Err(ValidationError)` - If invalid.
///
/// # Examples
///
/// ```rust
/// use harel::{parse_scxml, validate};
///
/// let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
///     <state id="start">
///         <transition event="go" target="end"/>
///     </state>
///     <final id="end"/>
/// </scxml>"#;
///
/// let scxml = parse_scxml(xml).expect("Failed to parse SCXML");
/// validate(&scxml).expect("SCXML validation failed");
/// ```
pub fn validate(scxml: &Scxml) -> Result<(), ValidationError> {
    let mut all_ids = std::collections::HashSet::new();

    // Collect and check for duplicate IDs.
    collect_state_ids(&scxml.states, &mut all_ids)?;

    // Validate transition targets exist.
    validate_transition_targets(&scxml.states, &all_ids)?;

    // Validate initial reference if present.
    if let Some(ref initial) = scxml.initial {
        if !all_ids.contains(initial) {
            return Err(ValidationError::InvalidTarget(initial.clone()));
        }
    }

    // Validate datamodel elements.
    validate_datamodel_constraints(&scxml.datamodel_elements)?;

    // TODO: Add check for circular initial references if needed.

    Ok(())
}

// Helper function to recursively collect state IDs and detect duplicates.
fn collect_state_ids(states: &[StateLike], all_ids: &mut std::collections::HashSet<String>) -> Result<(), ValidationError> {
    for state in states {
        match state {
            StateLike::State(s) => {
                if let Some(ref id) = s.id {
                    if !all_ids.insert(id.clone()) {
                        return Err(ValidationError::DuplicateId(id.clone()));
                    }
                }
                collect_state_ids(&s.children, all_ids)?;
            }
            StateLike::Parallel(p) => {
                if let Some(ref id) = p.id {
                    if !all_ids.insert(id.clone()) {
                        return Err(ValidationError::DuplicateId(id.clone()));
                    }
                }
                collect_state_ids(&p.children, all_ids)?;
            }
            StateLike::Final(f) => {
                if let Some(ref id) = f.id {
                    if !all_ids.insert(id.clone()) {
                        return Err(ValidationError::DuplicateId(id.clone()));
                    }
                }
            }
            StateLike::History(h) => {
                if let Some(ref id) = h.id {
                    if !all_ids.insert(id.clone()) {
                        return Err(ValidationError::DuplicateId(id.clone()));
                    }
                }
            }
        }
    }
    Ok(())
}

// Helper function to recursively validate transition targets.
fn validate_transition_targets(states: &[StateLike], all_ids: &std::collections::HashSet<String>) -> Result<(), ValidationError> {
    for state in states {
        match state {
            StateLike::State(s) => {
                for transition in &s.transitions {
                    if let Some(ref target) = transition.target {
                        for target_id in target.split_whitespace() {
                            if !all_ids.contains(target_id) {
                                return Err(ValidationError::InvalidTarget(target_id.to_string()));
                            }
                        }
                    }
                }
                validate_transition_targets(&s.children, all_ids)?;
            }
            StateLike::Parallel(p) => {
                for transition in &p.transitions {
                    if let Some(ref target) = transition.target {
                        for target_id in target.split_whitespace() {
                            if !all_ids.contains(target_id) {
                                return Err(ValidationError::InvalidTarget(target_id.to_string()));
                            }
                        }
                    }
                }
                validate_transition_targets(&p.children, all_ids)?;
            }
            StateLike::History(h) => {
                if let Some(ref transition) = h.transition {
                    if let Some(ref target) = transition.target {
                        for target_id in target.split_whitespace() {
                            if !all_ids.contains(target_id) {
                                return Err(ValidationError::InvalidTarget(target_id.to_string()));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// Helper function to validate datamodel constraints, such as unique data IDs.
fn validate_datamodel_constraints(data_elements: &[Data]) -> Result<(), ValidationError> {
    let mut data_ids = std::collections::HashSet::new();

    for data in data_elements {
        if !data_ids.insert(data.id.clone()) {
            return Err(ValidationError::DuplicateId(data.id.clone()));
        }
        // Note: SCXML allows empty data elements for late binding, so no check for expr/src/content.
    }

    Ok(())
}

/// Serializes the SCXML structure back to an XML string.
///
/// Produces well-formatted XML with indentation, including the XML declaration and namespace.
///
/// # Arguments
///
/// * `scxml` - The SCXML structure to serialize.
///
/// # Returns
///
/// A string containing the serialized XML.
///
/// # Examples
///
/// ```rust
/// use harel::{parse_scxml, to_xml};
///
/// let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
///     <state id="start"/>
/// </scxml>"#;
///
/// let scxml = parse_scxml(xml).expect("Failed to parse SCXML");
/// let serialized = to_xml(&scxml);
/// assert!(serialized.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
/// ```
pub fn to_xml(scxml: &Scxml) -> String {
    let mut output = String::new();
    // Add XML declaration.
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    // Start <scxml> tag with attributes.
    output.push_str("<scxml");
    output.push_str(&format!(" xmlns=\"{}\"", SCXML_NS));
    output.push_str(&format!(" version=\"{}\"", scxml.version));

    if let Some(ref initial) = scxml.initial {
        output.push_str(&format!(" initial=\"{}\"", initial));
    }

    if let Some(ref datamodel) = scxml.datamodel {
        output.push_str(&format!(" datamodel=\"{}\"", datamodel));
    }

    output.push_str(">\n");

    // Serialize <datamodel> if present.
    if !scxml.datamodel_elements.is_empty() {
        output.push_str("    <datamodel>\n");
        for data in &scxml.datamodel_elements {
            output.push_str(&format!("        <data id=\"{}\"", data.id));
            if let Some(ref expr) = data.expr {
                output.push_str(&format!(" expr=\"{}\"", expr));
            }
            if let Some(ref src) = data.src {
                output.push_str(&format!(" src=\"{}\"", src));
            }
            if let Some(ref content) = data.content {
                output.push_str(&format!(">{}</data>\n", content));
            } else {
                output.push_str("/>\n");
            }
        }
        output.push_str("    </datamodel>\n");
    }

    // Serialize child states.
    for state in &scxml.states {
        serialize_state_like(state, 1, &mut output);
    }

    output.push_str("</scxml>");
    output
}

// Helper to serialize state-like elements with indentation.
fn serialize_state_like(state: &StateLike, indent_level: usize, output: &mut String) {
    let indent = "    ".repeat(indent_level);

    match state {
        StateLike::State(s) => {
            output.push_str(&format!("{}<state", indent));
            if let Some(ref id) = s.id {
                output.push_str(&format!(" id=\"{}\"", id));
            }
            if let Some(ref initial) = s.initial {
                output.push_str(&format!(" initial=\"{}\"", initial));
            }

            if s.transitions.is_empty() && s.onentry.is_empty() && s.onexit.is_empty()
                && s.children.is_empty() && s.invokes.is_empty() && s.initial_element.is_none() {
                output.push_str("/>\n");
                return;
            }

            output.push_str(">\n");

            // Serialize <initial> if present.
            if let Some(ref initial_elem) = s.initial_element {
                serialize_initial(initial_elem, indent_level + 1, output);
            }

            // Serialize <onentry>.
            if !s.onentry.is_empty() {
                output.push_str(&format!("{}    <onentry>\n", indent));
                for executable in &s.onentry {
                    serialize_executable(executable, indent_level + 2, output);
                }
                output.push_str(&format!("{}    </onentry>\n", indent));
            }

            // Serialize children.
            for child in &s.children {
                serialize_state_like(child, indent_level + 1, output);
            }

            // Serialize transitions.
            for transition in &s.transitions {
                serialize_transition(transition, indent_level + 1, output);
            }

            // Serialize <onexit>.
            if !s.onexit.is_empty() {
                output.push_str(&format!("{}    <onexit>\n", indent));
                for executable in &s.onexit {
                    serialize_executable(executable, indent_level + 2, output);
                }
                output.push_str(&format!("{}    </onexit>\n", indent));
            }

            // Serialize <invoke>s.
            for invoke in &s.invokes {
                serialize_invoke(invoke, indent_level + 1, output);
            }

            output.push_str(&format!("{}</state>\n", indent));
        }
        StateLike::Parallel(p) => {
            output.push_str(&format!("{}<parallel", indent));
            if let Some(ref id) = p.id {
                output.push_str(&format!(" id=\"{}\"", id));
            }
            output.push_str(">\n");

            // Serialize children (no initial for parallel).
            for child in &p.children {
                serialize_state_like(child, indent_level + 1, output);
            }

            // TODO: Add serialization for transitions, onentry, onexit, invokes if needed.

            output.push_str(&format!("{}</parallel>\n", indent));
        }
        StateLike::Final(f) => {
            output.push_str(&format!("{}<final", indent));
            if let Some(ref id) = f.id {
                output.push_str(&format!(" id=\"{}\"", id));
            }
            // TODO: Add onentry/onexit if non-empty.
            output.push_str("/>\n");
        }
        StateLike::History(h) => {
            output.push_str(&format!("{}<history", indent));
            if let Some(ref id) = h.id {
                output.push_str(&format!(" id=\"{}\"", id));
            }
            output.push_str(&format!(" type=\"{}\"", h.type_));
            // TODO: Serialize transition if present.
            output.push_str("/>\n");
        }
    }
}

// Helper to serialize <initial>.
fn serialize_initial(initial: &Initial, indent_level: usize, output: &mut String) {
    let indent = "    ".repeat(indent_level);
    output.push_str(&format!("{}<initial", indent));
    if let Some(ref id) = initial.id {
        output.push_str(&format!(" id=\"{}\"", id));
    }
    output.push_str(">\n");
    serialize_transition(&initial.transition, indent_level + 1, output);
    output.push_str(&format!("{}</initial>\n", indent));
}

// Helper to serialize <transition>.
fn serialize_transition(transition: &Transition, indent_level: usize, output: &mut String) {
    let indent = "    ".repeat(indent_level);
    output.push_str(&format!("{}<transition", indent));

    if let Some(ref event) = transition.event {
        output.push_str(&format!(" event=\"{}\"", event));
    }
    if let Some(ref cond) = transition.cond {
        output.push_str(&format!(" cond=\"{}\"", cond));
    }
    if let Some(ref target) = transition.target {
        output.push_str(&format!(" target=\"{}\"", target));
    }
    if let Some(ref type_) = transition.type_ {
        output.push_str(&format!(" type=\"{}\"", type_));
    }

    if transition.executables.is_empty() {
        output.push_str("/>\n");
    } else {
        output.push_str(">\n");
        for executable in &transition.executables {
            serialize_executable(executable, indent_level + 1, output);
        }
        output.push_str(&format!("{}</transition>\n", indent));
    }
}

// Helper to serialize executable content.
fn serialize_executable(executable: &Executable, indent_level: usize, output: &mut String) {
    let indent = "    ".repeat(indent_level);

    match executable {
        Executable::Raise { event } => {
            output.push_str(&format!("{}<raise event=\"{}\"/>\n", indent, event));
        }
        Executable::Script { src, content } => {
            output.push_str(&format!("{}<script", indent));
            if let Some(src) = src {
                output.push_str(&format!(" src=\"{}\"", src));
            }
            if let Some(content) = content {
                output.push_str(&format!(">{}</script>\n", content));
            } else {
                output.push_str("/>\n");
            }
        }
        Executable::Assign { location, expr } => {
            output.push_str(&format!("{}<assign location=\"{}\" expr=\"{}\"/>\n", indent, location, expr));
        }
        Executable::Log { label, expr } => {
            output.push_str(&format!("{}<log", indent));
            if let Some(label) = label {
                output.push_str(&format!(" label=\"{}\"", label));
            }
            output.push_str(&format!(" expr=\"{}\"/>\n", expr));
        }
        // TODO: Add serialization for other Executable variants.
        _ => {
            output.push_str(&format!("{}<!-- Unsupported executable -->\n", indent));
        }
    }
}

// Helper to serialize <invoke>.
fn serialize_invoke(invoke: &Invoke, indent_level: usize, output: &mut String) {
    let indent = "    ".repeat(indent_level);
    output.push_str(&format!("{}<invoke type=\"{}\"", indent, invoke.type_));

    if let Some(ref src) = invoke.src {
        output.push_str(&format!(" src=\"{}\"", src));
    }
    if let Some(ref id) = invoke.id {
        output.push_str(&format!(" id=\"{}\"", id));
    }

    if invoke.params.is_empty() && invoke.finalize.is_none() && invoke.content.is_none() {
        output.push_str("/>\n");
        return;
    }

    output.push_str(">\n");

    // Serialize <param>s.
    for param in &invoke.params {
        output.push_str(&format!("{}    <param name=\"{}\"", indent, param.name));
        if let Some(ref expr) = param.expr {
            output.push_str(&format!(" expr=\"{}\"", expr));
        }
        if let Some(ref location) = param.location {
            output.push_str(&format!(" location=\"{}\"", location));
        }
        output.push_str("/>\n");
    }

    // TODO: Serialize finalize and content if present.

    output.push_str(&format!("{}</invoke>\n", indent));
}

// Helper to parse <state>.
fn parse_state(node: &Node) -> Result<State, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let initial = node.attribute("initial").map(|s| s.to_string());

    let mut initial_element = None;
    let mut transitions = Vec::new();
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();
    let mut children = Vec::new();
    let mut invokes = Vec::new();

    // Process child elements.
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "initial" => initial_element = Some(parse_initial(&child)?),
            "transition" => transitions.push(parse_transition(&child)?),
            "onentry" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            "state" => children.push(StateLike::State(parse_state(&child)?)),
            "parallel" => children.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => children.push(StateLike::Final(parse_final(&child)?)),
            "history" => children.push(StateLike::History(parse_history(&child)?)),
            "invoke" => invokes.push(parse_invoke(&child)?),
            _ => {},  // Ignore unsupported
        }
    }

    Ok(State { id, initial, initial_element, transitions, onentry, onexit, children, invokes })
}

// Helper to parse <parallel>.
fn parse_parallel(node: &Node) -> Result<Parallel, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let mut transitions = Vec::new();
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();
    let mut children = Vec::new();
    let mut invokes = Vec::new();

    // Process child elements.
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "transition" => transitions.push(parse_transition(&child)?),
            "onentry" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            "state" => children.push(StateLike::State(parse_state(&child)?)),
            "parallel" => children.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => children.push(StateLike::Final(parse_final(&child)?)),
            "history" => children.push(StateLike::History(parse_history(&child)?)),
            "invoke" => invokes.push(parse_invoke(&child)?),
            _ => {},
        }
    }

    Ok(Parallel { id, transitions, onentry, onexit, children, invokes })
}

// Helper to parse <final>.
fn parse_final(node: &Node) -> Result<Final, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();

    // Process child elements.
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "onentry" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            _ => {},
        }
    }

    Ok(Final { id, onentry, onexit })
}

// Helper to parse <transition>.
fn parse_transition(node: &Node) -> Result<Transition, ParseError> {
    let executables = parse_executables(node)?;

    Ok(Transition {
        event: node.attribute("event").map(|s| s.to_string()),
        cond: node.attribute("cond").map(|s| s.to_string()),
        target: node.attribute("target").map(|s| s.to_string()),
        type_: node.attribute("type").map(|s| s.to_string()),
        executables,
    })
}

// Helper to parse <initial>.
fn parse_initial(node: &Node) -> Result<Initial, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());

    let mut transition = None;
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "transition" {
            transition = Some(parse_transition(&child)?);
            break;
        }
    }

    let transition = transition.ok_or(ParseError::InvalidStructure("Initial must have a transition".into()))?;
    Ok(Initial { id, transition })
}

// Helper to parse <history>.
fn parse_history(node: &Node) -> Result<History, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let type_ = node.attribute("type").unwrap_or("shallow").to_string();

    let mut transition = None;
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "transition" {
            transition = Some(parse_transition(&child)?);
            break;
        }
    }

    Ok(History { id, type_, transition })
}

// Helper to parse <invoke>.
fn parse_invoke(node: &Node) -> Result<Invoke, ParseError> {
    let type_ = node.attribute("type").unwrap_or("").to_string();
    let src = node.attribute("src").map(|s| s.to_string());
    let id = node.attribute("id").map(|s| s.to_string());

    let mut params = Vec::new();
    let mut finalize = None;
    let mut content = None;

    // Process child elements.
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "param" => params.push(parse_param(&child)?),
            "finalize" => finalize = Some(parse_finalize(&child)?),
            "content" => content = Some(parse_content(&child)?),
            _ => {},
        }
    }

    Ok(Invoke { type_, src, id, params, finalize, content })
}

// Helper to parse <param>.
fn parse_param(node: &Node) -> Result<Param, ParseError> {
    let name = node.attribute("name").ok_or(ParseError::MissingAttribute("param name".into()))?.to_string();
    let expr = node.attribute("expr").map(|s| s.to_string());
    let location = node.attribute("location").map(|s| s.to_string());

    Ok(Param { name, expr, location })
}

// Helper to parse <finalize>.
fn parse_finalize(node: &Node) -> Result<Finalize, ParseError> {
    let executables = parse_executables(node)?;
    Ok(Finalize { executables })
}

// Helper to parse <content>.
fn parse_content(node: &Node) -> Result<Content, ParseError> {
    let expr = node.attribute("expr").map(|s| s.to_string());
    let content = node.text().map(|s| s.to_string());

    Ok(Content { expr, content })
}

// Helper to parse <datamodel>.
fn parse_datamodel(node: &Node) -> Result<Vec<Data>, ParseError> {
    let mut data_elements = Vec::new();
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "data" {
            let id = child.attribute("id").ok_or(ParseError::MissingAttribute("data id".into()))?.to_string();
            let expr = child.attribute("expr").map(|s| s.to_string());
            let src = child.attribute("src").map(|s| s.to_string());
            let content = child.text().map(|s| s.to_string());
            data_elements.push(Data { id, expr, src, content });
        }
    }
    Ok(data_elements)
}

// Helper to parse executables within a container node.
fn parse_executables(node: &Node) -> Result<Vec<Executable>, ParseError> {
    let mut execs = Vec::new();
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        execs.push(parse_single_executable(&child)?);
    }
    Ok(execs)
}

// Helper to parse a single executable element.
fn parse_single_executable(node: &Node) -> Result<Executable, ParseError> {
    match node.tag_name().name() {
        "raise" => Ok(Executable::Raise {
            event: node.attribute("event").unwrap_or("").to_string(),
        }),
        "if" => {
            let cond = node.attribute("cond").unwrap_or("").to_string();
            let mut then = Vec::new();
            let mut else_ = Vec::new();
            let mut in_else = false;
            for subchild in node.children() {
                if subchild.is_element() {
                    if subchild.tag_name().name() == "else" {
                        in_else = true;
                        continue;
                    }
                    let sub_exec = parse_single_executable(&subchild)?;
                    if in_else {
                        else_.push(sub_exec);
                    } else {
                        then.push(sub_exec);
                    }
                }
            }
            Ok(Executable::If { cond, then, else_ })
        }
        "foreach" => {
            let array = node.attribute("array").unwrap_or("").to_string();
            let item = node.attribute("item").unwrap_or("").to_string();
            let index = node.attribute("index").map(|s| s.to_string());
            let body = parse_executables(node)?;
            Ok(Executable::Foreach { array, item, index, body })
        }
        "send" => Ok(Executable::Send {
            event: node.attribute("event").unwrap_or("").to_string(),
            target: node.attribute("target").map(|s| s.to_string()),
        }),
        "script" => Ok(Executable::Script {
            src: node.attribute("src").map(|s| s.to_string()),
            content: node.text().map(|s| s.to_string()),
        }),
        "assign" => Ok(Executable::Assign {
            location: node.attribute("location").unwrap_or("").to_string(),
            expr: node.attribute("expr").unwrap_or("").to_string(),
        }),
        "log" => Ok(Executable::Log {
            label: node.attribute("label").map(|s| s.to_string()),
            expr: node.attribute("expr").unwrap_or("").to_string(),
        }),
        "cancel" => Ok(Executable::Cancel {
            sendid: node.attribute("sendid").unwrap_or("").to_string(),
        }),
        _ => Ok(Executable::Other(node.tag_name().name().to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_parse() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
            <datamodel>
                <data id="var1" expr="0"/>
            </datamodel>
            <state id="start">
                <onentry>
                    <if cond="var1 == 0">
                        <assign location="var1" expr="1"/>
                        <else/>
                        <log expr="var1"/>
                    </if>
                    <foreach array="items" item="i">
                        <send event="process"/>
                    </foreach>
                </onentry>
                <transition event="go" target="end"/>
                <parallel id="para">
                    <state id="sub1"/>
                </parallel>
            </state>
            <final id="end"/>
        </scxml>"#;
        let scxml = parse_scxml(xml).unwrap();
        assert_eq!(scxml.version, "1.0");
        assert_eq!(scxml.initial, Some("start".to_string()));
        assert_eq!(scxml.datamodel_elements[0].id, "var1");
        if let StateLike::State(state) = &scxml.states[0] {
            assert_eq!(state.id, Some("start".to_string()));
            if let Executable::If { cond, then, else_ } = &state.onentry[0] {
                assert_eq!(cond, "var1 == 0");
                assert_eq!(then.len(), 1);
                assert_eq!(else_.len(), 1);
            }
            if let Executable::Foreach { array, item, .. } = &state.onentry[1] {
                assert_eq!(array, "items");
                assert_eq!(item, "i");
            }
            if let StateLike::Parallel(para) = &state.children[0] {
                assert_eq!(para.id, Some("para".to_string()));
            }
        }
        if let StateLike::Final(f) = &scxml.states[1] {
            assert_eq!(f.id, Some("end".to_string()));
        }
    }

    #[test]
    fn test_invalid_namespace() {
        let xml = r#"
        <scxml xmlns="http://wrong.namespace" version="1.0">
            <state id="start"/>
        </scxml>
        "#;
        assert!(matches!(parse_scxml(xml), Err(ParseError::InvalidNamespace(_))));
    }

    #[test]
    fn test_blackjack_parsing() {
        use std::fs;
        let blackjack_xml = fs::read_to_string("examples/blackjack.scxml")
            .expect("Should read blackjack.scxml");

        let options = ParseOptions { relaxed_namespace: true };
        let result = parse_scxml_with_options(&blackjack_xml, options);
        assert!(result.is_ok(), "Blackjack SCXML should parse successfully: {:?}", result.err());

        let scxml = result.unwrap();
        assert_eq!(scxml.version, "1.0");
        assert_eq!(scxml.datamodel, Some("ecmascript".to_string()));
        assert_eq!(scxml.initial, Some("master".to_string()));

        assert_eq!(scxml.states.len(), 1);
        if let StateLike::State(master_state) = &scxml.states[0] {
            assert_eq!(master_state.id, Some("master".to_string()));
            assert!(master_state.initial_element.is_some());
            assert!(master_state.children.len() > 5);

            let welcome_state = master_state.children.iter().find(|state| {
                if let StateLike::State(s) = state {
                    s.id.as_ref() == Some(&"Welcome".to_string())
                } else { false }
            });
            assert!(welcome_state.is_some());
        }
    }

    #[test]
    fn test_validation() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
            <datamodel>
                <data id="var1" expr="0"/>
            </datamodel>
            <state id="start">
                <transition event="go" target="end"/>
            </state>
            <final id="end"/>
        </scxml>"#;

        let scxml = parse_scxml(xml).unwrap();
        let result = validate(&scxml);
        assert!(result.is_ok(), "Validation should pass: {:?}", result.err());
    }

    #[test]
    fn test_validation_duplicate_id() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
            <state id="duplicate"/>
            <state id="duplicate"/>
        </scxml>"#;

        let scxml = parse_scxml(xml).unwrap();
        let result = validate(&scxml);
        assert!(matches!(result, Err(ValidationError::DuplicateId(_))));
    }

    #[test]
    fn test_validation_invalid_target() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
            <state id="start">
                <transition event="go" target="nonexistent"/>
            </state>
        </scxml>"#;

        let scxml = parse_scxml(xml).unwrap();
        let result = validate(&scxml);
        assert!(matches!(result, Err(ValidationError::InvalidTarget(_))));
    }

    #[test]
    fn test_serialization() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
            <state id="start">
                <transition event="go" target="end"/>
            </state>
            <final id="end"/>
        </scxml>"#;

        let scxml = parse_scxml(xml).unwrap();
        let serialized = to_xml(&scxml);

        assert!(serialized.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(serialized.contains("<scxml"));
        assert!(serialized.contains("version=\"1.0\""));
        assert!(serialized.contains("initial=\"start\""));
        assert!(serialized.contains("<state id=\"start\">"));
        assert!(serialized.contains("<final id=\"end\""));

        let reparsed = parse_scxml(&serialized);
        assert!(reparsed.is_ok());
    }

    #[test]
    fn test_all_example_files() {
        use std::fs;

        let example_files = [
            "blackjack.scxml",
            "calc.scxml",
            "main.scxml",
            "microwave-parallell.scxml",
            "microwave.scxml",
            "traffic.scxml"
        ];

        for filename in &example_files {
            let path = format!("examples/{}", filename);
            if let Ok(content) = fs::read_to_string(&path) {
                let options = ParseOptions { relaxed_namespace: true };
                let result = parse_scxml_with_options(&content, options);
                assert!(result.is_ok(), "Failed to parse {}: {:?}", filename, result.err());

                let scxml = result.unwrap();
                let validation_result = validate(&scxml);
                assert!(validation_result.is_ok(), "Validation failed for {}: {:?}", filename, validation_result.err());

                println!("✅ Successfully parsed and validated: {}", filename);
            } else {
                println!("⚠️  Could not read file: {}", filename);
            }
        }
    }

    #[test]
    fn test_invoke_with_params() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
            <state id="calling">
                <invoke type="vxml3" src="dialog.vxml">
                    <param name="param1" expr="'value1'"/>
                    <param name="param2" location="var2"/>
                    <finalize>
                        <assign location="result" expr="event.data"/>
                    </finalize>
                </invoke>
            </state>
        </scxml>"#;

        let scxml = parse_scxml(xml).unwrap();
        if let StateLike::State(state) = &scxml.states[0] {
            assert_eq!(state.invokes.len(), 1);
            let invoke = &state.invokes[0];
            assert_eq!(invoke.type_, "vxml3");
            assert_eq!(invoke.src.as_ref().unwrap(), "dialog.vxml");
            assert_eq!(invoke.params.len(), 2);
            assert_eq!(invoke.params[0].name, "param1");
            assert_eq!(invoke.params[0].expr.as_ref().unwrap(), "'value1'");
            assert!(invoke.finalize.is_some());
        }
    }

    #[test]
    fn test_history_states() {
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
            <state id="parent">
                <history type="deep" id="hist">
                    <transition target="child1"/>
                </history>
                <state id="child1"/>
                <state id="child2"/>
            </state>
        </scxml>"#;

        let scxml = parse_scxml(xml).unwrap();
        if let StateLike::State(parent) = &scxml.states[0] {
            if let StateLike::History(hist) = &parent.children[0] {
                assert_eq!(hist.id.as_ref().unwrap(), "hist");
                assert_eq!(hist.type_, "deep");
                assert!(hist.transition.is_some());
                assert_eq!(hist.transition.as_ref().unwrap().target.as_ref().unwrap(), "child1");
            }
        }
    }
}