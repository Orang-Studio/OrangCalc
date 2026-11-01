// minimal element/attribute tree for settings and definition files
use std::fmt::Write;
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub text: String,
}
impl Node {
    pub fn new(name: &str) -> Node {
        Node { name: name.into(), ..Default::default() }
    }
    pub fn attr(&self, k: &str) -> &str {
        self.attrs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str()).unwrap_or("")
    }
    pub fn set(&mut self, k: &str, v: impl Into<String>) -> &mut Node {
        let v = v.into();
        match self.attrs.iter_mut().find(|(a, _)| a == k) {
            Some(e) => e.1 = v,
            None => self.attrs.push((k.into(), v)),
        }
        self
    }
    pub fn with(mut self, k: &str, v: impl Into<String>) -> Node {
        self.set(k, v);
        self
    }
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
    pub fn push(&mut self, n: Node) {
        self.children.push(n);
    }
    pub fn to_string_doc(&self) -> String {
        let mut s = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        self.write(&mut s, 0);
        s
    }
    fn write(&self, s: &mut String, depth: usize) {
        let pad = "  ".repeat(depth);
        let _ = write!(s, "{}<{}", pad, self.name);
        for (k, v) in &self.attrs {
            let _ = write!(s, " {}=\"{}\"", k, escape(v));
        }
        if self.children.is_empty() && self.text.is_empty() {
            s.push_str(" />\n");
            return;
        }
        s.push('>');
        if !self.text.is_empty() {
            s.push_str(&escape(&self.text));
        }
        if !self.children.is_empty() {
            s.push('\n');
            for c in &self.children {
                c.write(s, depth + 1);
            }
            s.push_str(&pad);
        }
        let _ = writeln!(s, "</{}>", self.name);
    }
    pub fn parse(text: &str) -> Result<Node, String> {
        let doc = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
        Ok(convert(doc.root_element()))
    }
}
fn convert(e: roxmltree::Node) -> Node {
    Node {
        name: e.tag_name().name().to_string(),
        attrs: e.attributes().map(|a| (a.name().to_string(), a.value().to_string())).collect(),
        children: e.children().filter(|c| c.is_element()).map(convert).collect(),
        text: e.children().filter(|c| c.is_text()).filter_map(|c| c.text()).collect::<String>().trim().to_string(),
    }
}
fn escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\n' => o.push_str("&#10;"),
            '\r' => o.push_str("&#13;"),
            '\t' => o.push_str("&#9;"),
            _ => o.push(ch),
        }
    }
    o
}