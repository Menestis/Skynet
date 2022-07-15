use serde::{Serialize};

pub type Message = Vec<MessageComponent>;

use strum_macros::Display;

#[derive(Debug, Serialize)]
pub struct MessageComponent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<Modifiers>,
}

#[derive(Debug, Serialize)]
pub struct Modifiers {
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
}

#[derive(Debug, Clone, Serialize, Display)]
#[serde(into = "String")]
pub enum Color {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LighPurple,
    Yellow,
    White,
    Reset,
    Custom(String),
}

impl From<Color> for String {
    fn from(color: Color) -> Self {
        match color {
            Color::Custom(hex) => hex,
            color => format!("{}", color)
        }
    }
}


pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self {
            message: vec![]
        }
    }

    pub fn component(self, text: String) -> MessageComponentBuilder {
        MessageComponentBuilder {
            message: self,
            component: MessageComponent {
                text,
                color: None,
                font: None,
                modifiers: None,
            },
        }
    }

    pub fn line_break(self) -> MessageBuilder {
        self.component("\n".to_string()).close()
    }

    pub fn close(self) -> Message {
        self.message
    }
}

pub struct MessageComponentBuilder {
    message: MessageBuilder,
    component: MessageComponent,
}

impl MessageComponentBuilder {
    pub fn close(self) -> MessageBuilder {
        let mut message = self.message;
        message.message.push(self.component);
        message
    }

    pub fn with_color(mut self, color: Option<Color>) -> MessageComponentBuilder {
        self.component.color = color;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Option<Modifiers>) -> MessageComponentBuilder {
        self.component.modifiers = modifiers;
        self
    }

    pub fn with_font(mut self, font: Option<String>) -> MessageComponentBuilder {
        self.component.font = font;
        self
    }
}



