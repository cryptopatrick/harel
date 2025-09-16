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
    pub initial_attr: Option<String>,  // 'initial' attribute
    pub initial_elem: Option<Initial>, // <initial> element
    pub transitions: Vec<Transition>,
    pub onentry: Vec<Executable>,
    pub onexit: Vec<Executable>,
    pub invokes: Vec<Invoke>,
    pub children: Vec<StateLike>,
    // Add history, etc.
}

#[derive(Debug, Clone)]
pub struct Parallel {
    pub id: Option<String>,
    pub initial_attr: Option<String>,
    pub initial_elem: Option<Initial>,
    pub transitions: Vec<Transition>,
    pub onentry: Vec<Executable>,
    pub onexit: Vec<Executable>,
    pub invokes: Vec<Invoke>,
    pub children: Vec<StateLike>,
}

#[derive(Debug, Clone)]
pub struct Final {
    pub id: Option<String>,
    pub onentry: Vec<Executable>,
    pub onexit: Vec<Executable>,
}

#[derive(Debug, Clone)]
pub struct Initial {
    pub id: Option<String>,
    pub transition: Transition,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub event: Option<String>,
    pub cond: Option<String>,
    pub target: Option<String>,
    pub executables: Vec<Executable>,
}

#[derive(Debug, Clone)]
pub struct Invoke {
    pub type_: Option<String>,
    pub src: Option<String>,
    pub id: Option<String>,
    pub autoforward: Option<bool>,
    // Add more: srcexpr, idlocation, namelist, typeexpr
    pub params: Vec<Param>,
    pub finalize: Option<Finalize>,
    // Add content if needed
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub expr: Option<String>,
    // Add location if needed
}

#[derive(Debug, Clone)]
pub struct Finalize {
    pub executables: Vec<Executable>,
}

#[derive(Debug, Clone)]
pub struct Data {
    pub id: String,
    pub expr: Option<String>,
    pub src: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Executable {
    Raise { event: String },
    If { cond: String, then: Vec<Executable>, r#else: Vec<Executable> },
    Foreach { array: String, item: String, index: Option<String>, body: Vec<Executable> },
    Send { event: String, target: Option<String> /* Add more */ },
    Script { src: Option<String>, content: Option<String> },
    Assign { location: String, expr: String },
    Log { label: Option<String>, expr: String },
    Cancel { sendid: String },
    Other(String),
}

pub fn parse_scxml(xml: &str) -> Result<Scxml, ParseError> {
    let doc = Document::parse(xml)?;
    let root = doc.root_element();

    // Relaxed NS check: Allow no NS or correct SCXML NS
    let ns = root.tag_name().namespace();
    if ns.is_some() && ns != Some(SCXML_NS) {
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
            _ => {},
        }
    }

    Ok(Scxml { version, initial, datamodel, states, datamodel_elements })
}

fn parse_state(node: &Node) -> Result<State, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let initial_attr = node.attribute("initial").map(|s| s.to_string());

    let mut initial_elem = None;
    let mut transitions = Vec::new();
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();
    let mut invokes = Vec::new();
    let mut children = Vec::new();

    for child in node.children() {
        if !child.is_element() { continue; }
        let tag = child.tag_name().name();
        match tag {
            "transition" => transitions.push(parse_transition(&child)?),
            "onentry" | "onenter" => onentry.extend(parse_executables(&child)?),  // Handle both (spec is "onentry")
            "onexit" => onexit.extend(parse_executables(&child)?),
            "initial" => initial_elem = Some(parse_initial(&child)?),
            "state" => children.push(StateLike::State(parse_state(&child)?)),
            "parallel" => children.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => children.push(StateLike::Final(parse_final(&child)?)),
            "invoke" => invokes.push(parse_invoke(&child)?),
            _ => {},
        }
    }

    Ok(State { id, initial_attr, initial_elem, transitions, onentry, onexit, invokes, children })
}

fn parse_parallel(node: &Node) -> Result<Parallel, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let initial_attr = node.attribute("initial").map(|s| s.to_string());

    let mut initial_elem = None;
    let mut transitions = Vec::new();
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();
    let mut invokes = Vec::new();
    let mut children = Vec::new();

    for child in node.children() {
        if !child.is_element() { continue; }
        let tag = child.tag_name().name();
        match tag {
            "transition" => transitions.push(parse_transition(&child)?),
            "onentry" | "onenter" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            "initial" => initial_elem = Some(parse_initial(&child)?),
            "state" => children.push(StateLike::State(parse_state(&child)?)),
            "parallel" => children.push(StateLike::Parallel(parse_parallel(&child)?)),
            "final" => children.push(StateLike::Final(parse_final(&child)?)),
            "invoke" => invokes.push(parse_invoke(&child)?),
            _ => {},
        }
    }

    Ok(Parallel { id, initial_attr, initial_elem, transitions, onentry, onexit, invokes, children })
}

fn parse_final(node: &Node) -> Result<Final, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let mut onentry = Vec::new();
    let mut onexit = Vec::new();

    for child in node.children() {
        if !child.is_element() { continue; }
        let tag = child.tag_name().name();
        match tag {
            "onentry" | "onenter" => onentry.extend(parse_executables(&child)?),
            "onexit" => onexit.extend(parse_executables(&child)?),
            _ => {},
        }
    }

    Ok(Final { id, onentry, onexit })
}

fn parse_initial(node: &Node) -> Result<Initial, ParseError> {
    let id = node.attribute("id").map(|s| s.to_string());
    let mut transition = None;
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "transition" {
            transition = Some(parse_transition(&child)?);
        }
    }
    let transition = transition.ok_or(ParseError::InvalidStructure("<initial> must contain <transition>".into()))?;
    Ok(Initial { id, transition })
}

fn parse_transition(node: &Node) -> Result<Transition, ParseError> {
    let event = node.attribute("event").map(|s| s.to_string());
    let cond = node.attribute("cond").map(|s| s.to_string());
    let target = node.attribute("target").map(|s| s.to_string());
    let executables = parse_executables(node)?;
    Ok(Transition { event, cond, target, executables })
}

fn parse_invoke(node: &Node) -> Result<Invoke, ParseError> {
    let type_ = node.attribute("type").map(|s| s.to_string());
    let src = node.attribute("src").map(|s| s.to_string());
    let id = node.attribute("id").map(|s| s.to_string());
    let autoforward = node.attribute("autoforward").map(|b| b == "true");

    let mut params = Vec::new();
    let mut finalize = None;

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "param" => {
                let name = child.attribute("name").ok_or(ParseError::MissingAttribute("param name".into()))?.to_string();
                let expr = child.attribute("expr").map(|s| s.to_string());
                params.push(Param { name, expr });
            }
            "finalize" => finalize = Some(Finalize { executables: parse_executables(&child)? }),
            // Add "content" if needed
            _ => {},
        }
    }

    Ok(Invoke { type_, src, id, autoforward, params, finalize })
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
        let tag = child.tag_name().name();
        match tag {
            "raise" => execs.push(Executable::Raise {
                event: child.attribute("event").unwrap_or("").to_string(),
            }),
            "if" => {
                let cond = child.attribute("cond").unwrap_or("").to_string();
                let mut then = Vec::new();
                let mut r#else = Vec::new();
                let mut in_else = false;
                for subchild in child.children() {
                    if subchild.is_element() {
                        if subchild.tag_name().name() == "else" {
                            in_else = true;
                            continue;
                        }
                        let sub_execs = parse_executables(&subchild)?;
                        if in_else {
                            r#else.extend(sub_execs);
                        } else {
                            then.extend(sub_execs);
                        }
                    }
                }
                execs.push(Executable::If { cond, then, r#else });
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
                content: child.text().map(|t| t.trim().to_string()),
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
            _ => execs.push(Executable::Other(tag.to_string())),
        }
    }
    Ok(execs)

    println!("{:#?}", scxml);.
}

// Test with your example file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example() {
        let xml = r#"
        <?xml version="1.0"?>
        <?access-control allow="*"?>
        <scxml version="1.0" datamodel="ecmascript" initial="master"> 
          <state id="master">
            <initial id="init1">
              <transition target="_home"/>
            </initial>
            <transition event="new_dealer" target="NewDealer"/>
            <transition event="mumble" target="_home"/> 
            <transition event="silence" target="_home"/> 
            <state id="_home">
              <onenter>
                <script>
                _data = {};
                </script>
              </onenter>
              <invoke src="datamodel.v3#InitDataModel" type="vxml3">
                <finalize>
                  <script>
                  var n;
                  for (n in event) {
                      _data[n] = event[n];
                  }
                  </script>
                </finalize>
              </invoke>
              <transition event="success" target="Welcome"/>
            </state>

            <state id="Welcome">
              <invoke src="dialog.vxml#Welcome" type="vxml3">
                <param name="skinpath" expr="skinpath"/>
              </invoke>
              <transition event="success" target="Intro2"/>
            </state>

            <!-- Additional states omitted for brevity in test, but parser handles them -->
          </state>
        </scxml>
        "#;
        let scxml = parse_scxml(xml).unwrap();
        assert_eq!(scxml.datamodel, Some("ecmascript".to_string()));
        if let StateLike::State(master) = &scxml.states[0] {
            assert_eq!(master.id, Some("master".to_string()));
            assert_eq!(master.initial_elem.as_ref().unwrap().id, Some("init1".to_string()));
            assert_eq!(master.initial_elem.as_ref().unwrap().transition.target, Some("_home".to_string()));
            if let StateLike::State(home) = &master.children[0] {
                assert_eq!(home.id, Some("_home".to_string()));
                if let Executable::Script { content, .. } = &home.onentry[0] {
                    assert_eq!(content.as_ref().unwrap(), "_data = {};");
                }
                let invoke = &home.invokes[0];
                assert_eq!(invoke.src, Some("datamodel.v3#InitDataModel".to_string()));
                assert_eq!(invoke.type_, Some("vxml3".to_string()));
                let finalize_script = &invoke.finalize.as_ref().unwrap().executables[0];
                if let Executable::Script { content, .. } = finalize_script {
                    assert!(content.as_ref().unwrap().contains("for (n in event)"));
                }
            }
        }
    }
}