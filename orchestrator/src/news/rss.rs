use crate::news::xml::{self, XmlNode, XmlError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RssError {
    #[error("XML error: {0}")]
    Xml(#[from] XmlError),
    #[error("Not an RSS feed")]
    NotRss,
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[derive(Debug, Clone)]
pub struct RssChannel {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Vec<RssItem>,
}

#[derive(Debug, Clone)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub description: String,
    pub pub_date: Option<String>,
    pub guid: Option<String>,
}

pub fn parse_rss(xml_str: &str) -> Result<RssChannel, RssError> {
    let root = xml::parse(xml_str)?;

    if let XmlNode::Element { name, children, .. } = root {
        if name == "rss" {
            let channel_node = children.iter().find(|n| {
                if let XmlNode::Element { name, .. } = n {
                    name == "channel"
                } else {
                    false
                }
            }).ok_or(RssError::NotRss)?;

            return parse_channel(channel_node);
        } else if name == "feed" {
            // Minimal Atom 1.0 support
            return parse_atom_feed(&children);
        }
    }

    Err(RssError::NotRss)
}

fn parse_channel(node: &XmlNode) -> Result<RssChannel, RssError> {
    if let XmlNode::Element { children, .. } = node {
        let mut title = None;
        let mut link = None;
        let mut description = None;
        let mut items = Vec::new();

        for child in children {
            if let XmlNode::Element { name, children: element_children, .. } = child {
                match name.as_str() {
                    "title" => title = get_text(element_children),
                    "link" => link = get_text(element_children),
                    "description" => description = get_text(element_children),
                    "item" => items.push(parse_item(child)?),
                    _ => {}
                }
            }
        }

        Ok(RssChannel {
            title: title.unwrap_or_default(),
            link: link.unwrap_or_default(),
            description: description.unwrap_or_default(),
            items,
        })
    } else {
        Err(RssError::NotRss)
    }
}

fn parse_item(node: &XmlNode) -> Result<RssItem, RssError> {
    if let XmlNode::Element { children, .. } = node {
        let mut title = None;
        let mut link = None;
        let mut description = None;
        let mut pub_date = None;
        let mut guid = None;

        for child in children {
            if let XmlNode::Element { name, children: element_children, .. } = child {
                match name.as_str() {
                    "title" => title = get_text(element_children),
                    "link" => link = get_text(element_children),
                    "description" => description = get_text(element_children),
                    "pubDate" => pub_date = get_text(element_children),
                    "guid" => guid = get_text(element_children),
                    _ => {}
                }
            }
        }

        Ok(RssItem {
            title: title.unwrap_or_default(),
            link: link.unwrap_or_default(),
            description: description.unwrap_or_default(),
            pub_date,
            guid,
        })
    } else {
        Err(RssError::NotRss)
    }
}

fn parse_atom_feed(children: &[XmlNode]) -> Result<RssChannel, RssError> {
    let mut title = None;
    let mut link = None;
    let mut description = None;
    let mut items = Vec::new();

    for child in children {
        if let XmlNode::Element { name, children: element_children, attrs, .. } = child {
            match name.as_str() {
                "title" => title = get_text(element_children),
                "link" => {
                    // Atom links often use href attribute
                    if let Some(href) = attrs.get("href") {
                        link = Some(href.clone());
                    } else {
                        link = get_text(element_children);
                    }
                },
                "subtitle" => description = get_text(element_children),
                "entry" => items.push(parse_atom_entry(child)?),
                _ => {}
            }
        }
    }

    Ok(RssChannel {
        title: title.unwrap_or_default(),
        link: link.unwrap_or_default(),
        description: description.unwrap_or_default(),
        items,
    })
}

fn parse_atom_entry(node: &XmlNode) -> Result<RssItem, RssError> {
    if let XmlNode::Element { children, .. } = node {
        let mut title = None;
        let mut link = None;
        let mut description = None;
        let mut pub_date = None;
        let mut guid = None;

        for child in children {
            if let XmlNode::Element { name, children: element_children, attrs, .. } = child {
                match name.as_str() {
                    "title" => title = get_text(element_children),
                    "link" => {
                        if let Some(href) = attrs.get("href") {
                            link = Some(href.clone());
                        } else {
                            link = get_text(element_children);
                        }
                    },
                    "summary" | "content" => description = get_text(element_children),
                    "published" | "updated" => pub_date = get_text(element_children),
                    "id" => guid = get_text(element_children),
                    _ => {}
                }
            }
        }

        Ok(RssItem {
            title: title.unwrap_or_default(),
            link: link.unwrap_or_default(),
            description: description.unwrap_or_default(),
            pub_date,
            guid,
        })
    } else {
        Err(RssError::NotRss)
    }
}

fn get_text(children: &[XmlNode]) -> Option<String> {
    let mut text = String::new();
    for child in children {
        match child {
            XmlNode::Text(t) => text.push_str(t),
            XmlNode::CData(t) => text.push_str(t),
            _ => {}
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rss_2_0() {
        let xml = r#"
            <rss version="2.0">
                <channel>
                    <title>Test Feed</title>
                    <link>http://example.com</link>
                    <description>A test RSS 2.0 feed</description>
                    <item>
                        <title>Item 1</title>
                        <link>http://example.com/1</link>
                        <description>Description 1</description>
                        <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>
                        <guid>1</guid>
                    </item>
                </channel>
            </rss>
        "#;
        let channel = parse_rss(xml).unwrap();
        assert_eq!(channel.title, "Test Feed");
        assert_eq!(channel.items.len(), 1);
        assert_eq!(channel.items[0].title, "Item 1");
    }

    #[test]
    fn test_parse_atom_1_0() {
        let xml = r#"
            <feed xmlns="http://www.w3.org/2005/Atom">
                <title>Atom Feed</title>
                <subtitle>A test Atom feed</subtitle>
                <link href="http://example.com"/>
                <entry>
                    <title>Entry 1</title>
                    <link href="http://example.com/1"/>
                    <summary>Summary 1</summary>
                    <published>2024-01-01T00:00:00Z</published>
                    <id>1</id>
                </entry>
            </feed>
        "#;
        let channel = parse_rss(xml).unwrap();
        assert_eq!(channel.title, "Atom Feed");
        assert_eq!(channel.items.len(), 1);
        assert_eq!(channel.items[0].title, "Entry 1");
        assert_eq!(channel.items[0].link, "http://example.com/1");
    }
}
