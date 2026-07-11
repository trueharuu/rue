//! Command execution contexts.

/// The execution context passed to a command handler.
///
/// Wraps the argument text and provides helpers for parsing typed
/// arguments and sending replies.
pub struct Context<'a> {
    /// The raw argument text passed to the command.
    args_text: &'a str,
    /// The current offset into the argument text.
    offset: usize,
    /// Channel used to send reply messages back to the source.
    reply_tx: &'a tokio::sync::mpsc::Sender<String>,
}

impl<'a> Context<'a> {
    /// Create a new context from the argument portion of a message.
    #[must_use]
    pub fn new(args_text: &'a str, reply_tx: &'a tokio::sync::mpsc::Sender<String>) -> Self {
        Self {
            args_text,
            offset: 0,
            reply_tx,
        }
    }

    /// Send a reply message back to the source.
    pub async fn reply(&self, message: &str) -> anyhow::Result<()> {
        self.reply_tx
            .send(message.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("failed to send reply: {e}"))
    }

    /// Return the remaining unparsed argument text.
    fn remaining(&self) -> &'a str {
        &self.args_text[self.offset..]
    }

    /// Return the next whitespace-delimited word, or `None` if exhausted.
    pub fn next_word(&mut self) -> Option<&str> {
        let remaining = self.remaining();
        
        let trimmed = remaining.trim_start_matches(char::is_whitespace);
        if trimmed.is_empty() {
            return None;
        }

        let word_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let word = &trimmed[..word_end];

        let whitespace_len = remaining.len() - trimmed.len();
        self.offset += whitespace_len + word_end;
        Some(word)
    }

    /// Consume and return all remaining argument text.
    pub fn rest(&mut self) -> &str {
        let rest = self.remaining().trim();
        self.offset = self.args_text.len();
        rest
    }

    /// Snapshot the current position so it can be restored later.
    #[must_use]
    pub fn save_position(&self) -> usize {
        self.offset
    }

    /// Restore a previously saved position.
    pub fn restore_position(&mut self, saved: usize) {
        self.offset = saved;
    }
}
