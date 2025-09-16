use roxmltree::{Document, Node};
use thiserror::Error;

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

const SCXML_NS: &str = "http://www.w3.org/2005/07/scxml";

// Core SCXML struct
#[derive(Debug, Clone)]
pub struct Scxml {
    pub version: String,
    pub initial: Option<String>,
    pub datamodel: Option<String>,
    pub states: Vec<StateLike>,
    pub datamodel_elements: Vec<Data>,
}

// State-like elements: <state>, <parallel>, <final>
#[derive(Debug, Clone)]
pub enum StateLike {
    State(State),
    Parallel(Parallel),
    Final(Final),
}

#[derive(Debug, Clone)]
pub struct State {
    pub id: Option<String>,
    pub initial: Option<String>,
    pub transitions: Vec<Transition>,
    pub onentry: Vec<Executable>,
    pub onexit: Vec<Executable>,
    pub children: Vec<StateLike>,
    // Add history, invoke, etc.
}

#[derive(Debug, Clone)]
pub struct Parallel {
    pub id: Option<String>,
    pub transitions: Vec<Transition>,
    pub onentry: Vec<Executable>,
    pub onexit: Vec<Executable>,
    pub children: Vec<StateLike>,
}

#[derive(Debug, Clone)]
pub struct Final {
    pub id: Option<String>,
    pub onentry: Vec<Executable>,
    pub onexit: Vec<Executable>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub event: Option<String>,
    pub cond: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Data {
    pub id: String,
    pub expr: Option<String>,
    pub src: Option<String>,
    // Add content for inline data
}

#[derive(Debug, Clone)]
pub enum Executable {
    Raise { event: String },
    If { cond: String, then: Vec<Executable>, else_: Vec<Executable> },
    Foreach { array: String, item: String, index: Option<String>, body: Vec<Executable> },
    Send { event: String, target: Option<String> /* Add more */ },
    Script { src: Option<String>, content: Option<String> },
    Assign { location: String, expr: String },
    Log { label: Option<String>, expr: String },
    Cancel { sendid: String },
    Other(String), // For unsupported or custom elements
}

pub fn parse_scxml(xml: &str) -> Result<Scxml, ParseError> {
    let doc = Document::parse(xml)?;
    let root = doc.root_element();
    
    // Check namespace
    if root.tag_name().namespace() != Some(SCXML_NS) {
        return Err(ParseError::InvalidNamespace(SCXML_NS.to_string()));
    }
    if root.tag_name().name() != "scxml" {
        return Err(ParseError::InvalidStructure("Root must be <scxml>".into()));
    }

    let version = root.attribute("version").ok_or(ParseError::MissingAttribute("version".into()))?.to_string();
    if version != "1.0" {
        return Err(ParseError::InvalidStructure("SCXML version must be 1.0".into()));
    }
    let initial = root.attribute("initial").map(|s| s.to_string());
    let datamodel = root.attribute("datamodel").map(|s| s.to_string());

    let mut states = Vec::new();
    let mut datamodel_elements = Vec::new();

    for child in root.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "state" => states.push(StateLike::State(parse_state(&child)?)),
            "parallel" => states.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => states.push(StateLike::Final(parse_final(&child)?)),
            "datamodel" => datamodel_elements.extend(parse_datamodel(&child)?),
            // Add more: history, invoke
            _ => {},
        }
    }

    Ok(Scxml { version, initial, datamodel, states, datamodel_elements })
}

fn parse_state(node: &Node) -> Result<State, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let initial = node.attribute("initial").map(|s| s.to_string());

    let mut transitions = Vec::new();
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();
    let mut children = Vec::new();

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "transition" => transitions.push(parse_transition(&child)?),
            "onentry" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            "state" => children.push(StateLike::State(parse_state(&child)?)),
            "parallel" => children.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => children.push(StateLike::Final(parse_final(&child)?)),
            // Add history, invoke
            _ => {},
        }
    }

    Ok(State { id, initial, transitions, onentry, onexit, children })
}

fn parse_parallel(node: &Node) -> Result<Parallel, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let mut transitions = Vec::new();
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();
    let mut children = Vec::new();

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "transition" => transitions.push(parse_transition(&child)?),
            "onentry" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            "state" => children.push(StateLike::State(parse_state(&child)?)),
            "parallel" => children.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => children.push(StateLike::Final(parse_final(&child)?)),
            _ => {},
        }
    }

    Ok(Parallel { id, transitions, onentry, onexit, children })
}

fn parse_final(node: &Node) -> Result<Final, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "onentry" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            _ => {},
        }
    }

    Ok(Final { id, onentry, onexit })
}

fn parse_transition(node: &Node) -> Result<Transition, ParseError> {
    Ok(Transition {
        event: node.attribute("event").map(|s| s.to_string()),
        cond: node.attribute("cond").map(|s| s.to_string()),
        target: node.attribute("target").map(|s| s.to_string()),
    })
}

fn parse_datamodel(node: &Node) -> Result<Vec<Data>, ParseError> {
    let mut data_elements = Vec::new();
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "data" {
            let id = child.attribute("id").ok_or(ParseError::MissingAttribute("data id".into()))?.to_string();
            let expr = child.attribute("expr").map(|s| s.to_string());
            let src = child.attribute("src").map(|s| s.to_string());
            data_elements.push(Data { id, expr, src });
        }
    }
    Ok(data_elements)
}

fn parse_executables(node: &Node) -> Result<Vec<Executable>, ParseError> {
    let mut execs = Vec::new();
    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "raise" => execs.push(Executable::Raise {
                event: child.attribute("event").unwrap_or("").to_string(),
            }),
            "if" => {
                let cond = child.attribute("cond").unwrap_or("").to_string();
                let mut then = Vec::new();
                let mut else_ = Vec::new();
                let mut in_else = false;
                for subchild in child.children() {
                    if subchild.is_element() && subchild.tag_name().name() == "else" {
                        in_else = true;
                        continue;
                    }
                    if subchild.is_element() {
                        let sub_execs = parse_executables(&subchild)?;
                        if in_else {
                            else_.extend(sub_execs);
                        } else {
                            then.extend(sub_execs);
                        }
                    }
                }
                execs.push(Executable::If { cond, then, else_ });
            }
            "foreach" => {
                let array = child.attribute("array").unwrap_or("").to_string();
                let item = child.attribute("item").unwrap_or("").to_string();
                let index = child.attribute("index").map(|s| s.to_string());
                let body = parse_executables(&child)?;
                execs.push(Executable::Foreach { array, item, index, body });
            }
            "send" => execs.push(Executable::Send {
                event: child.attribute("event").unwrap_or("").to_string(),
                target: child.attribute("target").map(|s| s.to_string()),
            }),
            "script" => execs.push(Executable::Script {
                src: child.attribute("src").map(|s| s.to_string()),
                content: child.text().map(|s| s.to_string()),
            }),
            "assign" => execs.push(Executable::Assign {
                location: child.attribute("location").unwrap_or("").to_string(),
                expr: child.attribute("expr").unwrap_or("").to_string(),
            }),
            "log" => execs.push(Executable::Log {
                label: child.attribute("label").map(|s| s.to_string()),
                expr: child.attribute("expr").unwrap_or("").to_string(),
            }),
            "cancel" => execs.push(Executable::Cancel {
                sendid: child.attribute("sendid").unwrap_or("").to_string(),
            }),
            _ => execs.push(Executable::Other(child.tag_name().name().to_string())),
        }
    }
    Ok(execs)
}

// Extended tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_parse() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="start">
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
        </scxml>
        "#;
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
}