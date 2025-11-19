use crate::game::Forward;
use crate::game::Game;
use std::fmt;
use std::fs;
use std::io;

/// Error type for level parsing operations.
#[derive(Debug)]
pub enum LevelError {
    /// IO error when reading from file
    Io(io::Error),
    /// Invalid level content
    InvalidLevel(String),
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LevelError::Io(err) => write!(f, "IO error: {}", err),
            LevelError::InvalidLevel(msg) => write!(f, "Invalid level: {}", msg),
        }
    }
}

impl From<io::Error> for LevelError {
    fn from(err: io::Error) -> Self {
        LevelError::Io(err)
    }
}

impl From<String> for LevelError {
    fn from(err: String) -> Self {
        LevelError::InvalidLevel(err)
    }
}

/// A collection of Sokoban levels in XSB format.
#[derive(Debug)]
pub struct Levels {
    levels: Vec<Game<Forward>>,
}

impl Levels {
    /// Parse XSB-formatted Sokoban levels from a string.
    ///
    /// The XSB format uses:
    /// - Lines starting with `;` as level separators/comments
    /// - Standard Sokoban characters (#, @, $, ., *, +, space)
    /// - Empty lines between levels (optional)
    ///
    /// Parses and validates each level, returning a Levels struct containing Game instances.
    pub fn from_text(contents: &str) -> Result<Self, LevelError> {
        let mut levels = Vec::new();
        let mut current_level = String::new();

        for line in contents.lines() {
            // Skip comment lines (level separators)
            if line.trim_start().starts_with(';') {
                // If we have accumulated a level, parse and save it
                if !current_level.is_empty() {
                    // Remove trailing newline but preserve internal structure
                    let level_str = current_level.trim_end();
                    let game = Game::from_text(level_str)?;
                    levels.push(game);
                    current_level.clear();
                }
                continue;
            }

            // Skip empty lines when we don't have a level started
            if line.is_empty() {
                if !current_level.is_empty() {
                    // Empty line within a level - end of level
                    // Remove trailing newline but preserve internal structure
                    let level_str = current_level.trim_end();
                    let game = Game::from_text(level_str)?;
                    levels.push(game);
                    current_level.clear();
                }
                continue;
            }

            // Add line to current level
            current_level.push_str(line);
            current_level.push('\n');
        }

        // Don't forget the last level if file doesn't end with empty line
        if !current_level.is_empty() {
            // Remove trailing newline but preserve internal structure
            let level_str = current_level.trim_end();
            let game = Game::from_text(level_str)?;
            levels.push(game);
        }

        Ok(Levels { levels })
    }

    /// Parse XSB-formatted Sokoban levels from a text file.
    pub fn from_file(path: &str) -> Result<Self, LevelError> {
        let contents = fs::read_to_string(path)?;
        Self::from_text(&contents)
    }

    /// Get the nth level (0-indexed).
    pub fn get(&self, index: usize) -> Option<&Game<Forward>> {
        self.levels.get(index)
    }

    /// Get the number of levels.
    pub fn len(&self) -> usize {
        self.levels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_text_basic() {
        let level1 = "####
# .#
#  ###
#*@  #
#  $ #
#  ###
####";

        let level2 = "######
#    #
# #@ #
# $* #
# .* #
#    #
######";

        let level3 = "  ####
###  ####
#     $ #
# #  #$ #
# . .#@ #
#########";

        let xsb_content = format!(
            "; 1\n\n{}\n\n; 2\n\n{}\n\n; 3\n\n{}\n",
            level1, level2, level3
        );

        let levels = Levels::from_text(&xsb_content).unwrap();

        assert_eq!(levels.len(), 3);

        // Verify levels match the original strings when formatted back
        assert_eq!(levels.get(0).unwrap().to_string().trim_end(), level1);
        assert_eq!(levels.get(1).unwrap().to_string().trim_end(), level2);
        assert_eq!(levels.get(2).unwrap().to_string().trim_end(), level3);
    }

    #[test]
    fn test_from_text_invalid_level() {
        let xsb_content = "; 1

####
# .#
#@@  #
####
";

        let result = Levels::from_text(xsb_content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LevelError::InvalidLevel(_)));
    }

    #[test]
    fn test_from_file_no_file() {
        let result = Levels::from_file("nonexistent_file.xsb");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LevelError::Io(_)));
    }
}
