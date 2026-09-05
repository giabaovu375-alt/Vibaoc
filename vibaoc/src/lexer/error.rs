// ============================================================
// VIBAO COMPILER (Rust) — lexer/error.rs
// LEXER ERROR: the error type returned when tokenizing fails
// ============================================================

use std::fmt;

// ════════════════════════════════════════════════════════════
// 3. LEXER ERROR
// ════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[lexer] {} ({}:{})",
            self.message, self.line, self.column
        )
    }
}
