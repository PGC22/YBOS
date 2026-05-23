use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum XmlError {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Malformed tag at {0}")]
    MalformedTag(usize),
    #[error("Invalid entity at {0}")]
    InvalidEntity(usize),
    #[error("Unclosed tag: {0}")]
    UnclosedTag(String),
    #[error("Mismatched tag: expected {expected}, found {found}")]
    MismatchedTag { expected: String, found: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum XmlToken {
    StartTag { name: String, attrs: HashMap<String, String>, self_closing: bool },
    EndTag { name: String },
    Text(String),
    CData(String),
    Comment,
    ProcessingInstruction,
}

pub struct XmlLexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> XmlLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    pub fn next_token(&mut self) -> Result<Option<XmlToken>, XmlError> {
        loop {
            if self.pos >= self.input.len() {
                return Ok(None);
            }

            if self.starts_with("<!--") {
                self.consume_comment()?;
                return Ok(Some(XmlToken::Comment));
            }

            if self.starts_with("<![CDATA[") {
                return Ok(Some(XmlToken::CData(self.consume_cdata()?)));
            }

            if self.starts_with("<?") {
                self.consume_pi()?;
                return Ok(Some(XmlToken::ProcessingInstruction));
            }

            if self.starts_with("<") {
                if self.input[self.pos + 1..].starts_with('/') {
                    return Ok(Some(XmlToken::EndTag { name: self.consume_end_tag()? }));
                } else {
                    return self.consume_start_tag().map(|t| Some(t));
                }
            }

            let text = self.consume_text()?;
            if !text.is_empty() {
                return Ok(Some(XmlToken::Text(text)));
            }
        }
    }

    fn consume_comment(&mut self) -> Result<(), XmlError> {
        self.pos += 4; // skip <!--
        while !self.starts_with("-->") {
            if self.consume().is_none() {
                return Err(XmlError::UnexpectedEof);
            }
        }
        self.pos += 3; // skip -->
        Ok(())
    }

    fn consume_cdata(&mut self) -> Result<String, XmlError> {
        self.pos += 9; // skip <![CDATA[
        let start = self.pos;
        while !self.starts_with("]]>") {
            if self.consume().is_none() {
                return Err(XmlError::UnexpectedEof);
            }
        }
        let end = self.pos;
        self.pos += 3; // skip ]]>
        Ok(self.input[start..end].to_string())
    }

    fn consume_pi(&mut self) -> Result<(), XmlError> {
        self.pos += 2; // skip <?
        while !self.starts_with("?>") {
            if self.consume().is_none() {
                return Err(XmlError::UnexpectedEof);
            }
        }
        self.pos += 2; // skip ?>
        Ok(())
    }

    fn consume_start_tag(&mut self) -> Result<XmlToken, XmlError> {
        self.pos += 1; // skip <
        let name = self.consume_name()?;
        let mut attrs = HashMap::new();

        loop {
            self.consume_whitespace();
            if self.starts_with(">") {
                self.pos += 1;
                return Ok(XmlToken::StartTag { name, attrs, self_closing: false });
            }
            if self.starts_with("/>") {
                self.pos += 2;
                return Ok(XmlToken::StartTag { name, attrs, self_closing: true });
            }

            let attr_name = self.consume_name()?;
            self.consume_whitespace();
            if self.consume() != Some('=') {
                return Err(XmlError::MalformedTag(self.pos));
            }
            self.consume_whitespace();
            let attr_value = self.consume_quoted_value()?;
            attrs.insert(attr_name, attr_value);
        }
    }

    fn consume_end_tag(&mut self) -> Result<String, XmlError> {
        self.pos += 2; // skip </
        let name = self.consume_name()?;
        self.consume_whitespace();
        if self.consume() != Some('>') {
            return Err(XmlError::MalformedTag(self.pos));
        }
        Ok(name)
    }

    fn consume_name(&mut self) -> Result<String, XmlError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.' {
                self.consume();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(XmlError::MalformedTag(self.pos));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn consume_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.consume();
            } else {
                break;
            }
        }
    }

    fn consume_quoted_value(&mut self) -> Result<String, XmlError> {
        let quote = self.consume().ok_or(XmlError::UnexpectedEof)?;
        if quote != '"' && quote != '\'' {
            return Err(XmlError::MalformedTag(self.pos));
        }
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == quote {
                let end = self.pos;
                self.consume();
                return Ok(decode_entities(&self.input[start..end])?);
            }
            self.consume();
        }
        Err(XmlError::UnexpectedEof)
    }

    fn consume_text(&mut self) -> Result<String, XmlError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '<' {
                break;
            }
            self.consume();
        }
        decode_entities(&self.input[start..self.pos])
    }
}

fn decode_entities(input: &str) -> Result<String, XmlError> {
    let mut output = String::new();
    let mut chars = input.chars().enumerate();
    while let Some((i, c)) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            let mut found_semi = false;
            while let Some((_, ec)) = chars.next() {
                if ec == ';' {
                    found_semi = true;
                    break;
                }
                entity.push(ec);
            }
            if !found_semi {
                return Err(XmlError::InvalidEntity(i));
            }
            match entity.as_str() {
                "amp" => output.push('&'),
                "lt" => output.push('<'),
                "gt" => output.push('>'),
                "quot" => output.push('"'),
                "apos" => output.push('\''),
                _ if entity.starts_with("#x") => {
                    let hex = &entity[2..];
                    let code = u32::from_str_radix(hex, 16).map_err(|_| XmlError::InvalidEntity(i))?;
                    output.push(std::char::from_u32(code).ok_or(XmlError::InvalidEntity(i))?);
                }
                _ if entity.starts_with("#") => {
                    let dec = &entity[1..];
                    let code = dec.parse::<u32>().map_err(|_| XmlError::InvalidEntity(i))?;
                    output.push(std::char::from_u32(code).ok_or(XmlError::InvalidEntity(i))?);
                }
                _ => return Err(XmlError::InvalidEntity(i)),
            }
        } else {
            output.push(c);
        }
    }
    Ok(output)
}

#[derive(Debug, Clone, PartialEq)]
pub enum XmlNode {
    Element {
        name: String,
        attrs: HashMap<String, String>,
        children: Vec<XmlNode>,
    },
    Text(String),
    CData(String),
}

pub fn parse(input: &str) -> Result<XmlNode, XmlError> {
    let mut lexer = XmlLexer::new(input);
    let mut stack: Vec<XmlNode> = Vec::new();

    while let Some(token) = lexer.next_token()? {
        match token {
            XmlToken::StartTag { name, attrs, self_closing } => {
                let node = XmlNode::Element {
                    name,
                    attrs,
                    children: Vec::new(),
                };
                if self_closing {
                    if let Some(XmlNode::Element { children, .. }) = stack.last_mut() {
                        children.push(node);
                    } else if stack.is_empty() {
                        return Ok(node);
                    }
                } else {
                    stack.push(node);
                }
            }
            XmlToken::EndTag { name } => {
                let finished_node = stack.pop().ok_or(XmlError::MismatchedTag {
                    expected: "None".to_string(),
                    found: name.clone(),
                })?;

                if let XmlNode::Element { name: ref start_name, .. } = finished_node {
                    if start_name != &name {
                        return Err(XmlError::MismatchedTag {
                            expected: start_name.clone(),
                            found: name,
                        });
                    }
                }

                if stack.is_empty() {
                    return Ok(finished_node);
                } else {
                    if let Some(XmlNode::Element { children, .. }) = stack.last_mut() {
                        children.push(finished_node);
                    }
                }
            }
            XmlToken::Text(t) => {
                if !t.trim().is_empty() {
                    if let Some(XmlNode::Element { children, .. }) = stack.last_mut() {
                        children.push(XmlNode::Text(t));
                    }
                }
            }
            XmlToken::CData(t) => {
                if let Some(XmlNode::Element { children, .. }) = stack.last_mut() {
                    children.push(XmlNode::CData(t));
                }
            }
            XmlToken::Comment | XmlToken::ProcessingInstruction => {}
        }
    }

    Err(XmlError::UnexpectedEof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let xml = r#"<root attr="val">text</root>"#;
        let mut lexer = XmlLexer::new(xml);
        assert!(matches!(lexer.next_token().unwrap(), Some(XmlToken::StartTag { .. })));
        assert!(matches!(lexer.next_token().unwrap(), Some(XmlToken::Text(..))));
        assert!(matches!(lexer.next_token().unwrap(), Some(XmlToken::EndTag { .. })));
        assert!(matches!(lexer.next_token().unwrap(), None));
    }

    #[test]
    fn test_lexer_entities() {
        let xml = r#"<t>&amp;&lt;&gt;&quot;&apos;&#65;&#x42;</t>"#;
        let mut lexer = XmlLexer::new(xml);
        lexer.next_token().unwrap();
        if let Some(XmlToken::Text(t)) = lexer.next_token().unwrap() {
            assert_eq!(t, r#"&<>"'AB"#);
        } else {
            panic!("Expected text");
        }
    }

    #[test]
    fn test_lexer_cdata() {
        let xml = r#"<t><![CDATA[<not a tag>]]></t>"#;
        let mut lexer = XmlLexer::new(xml);
        lexer.next_token().unwrap();
        if let Some(XmlToken::CData(t)) = lexer.next_token().unwrap() {
            assert_eq!(t, "<not a tag>");
        } else {
            panic!("Expected CDATA");
        }
    }

    #[test]
    fn test_parser_basic() {
        let xml = r#"<root><child>text</child></root>"#;
        let node = parse(xml).unwrap();
        if let XmlNode::Element { name, children, .. } = node {
            assert_eq!(name, "root");
            assert_eq!(children.len(), 1);
            if let XmlNode::Element { name, children, .. } = &children[0] {
                assert_eq!(name, "child");
                assert_eq!(children.len(), 1);
                if let XmlNode::Text(t) = &children[0] {
                    assert_eq!(t, "text");
                }
            }
        }
    }

    #[test]
    fn test_parser_self_closing() {
        let xml = r#"<root><child/></root>"#;
        let node = parse(xml).unwrap();
        if let XmlNode::Element { children, .. } = node {
            assert_eq!(children.len(), 1);
            if let XmlNode::Element { name, .. } = &children[0] {
                assert_eq!(name, "child");
            }
        }
    }

    #[test]
    fn test_parser_mismatched() {
        let xml = r#"<root><child></root>"#;
        let err = parse(xml).unwrap_err();
        assert!(matches!(err, XmlError::MismatchedTag { .. }));
    }
}
