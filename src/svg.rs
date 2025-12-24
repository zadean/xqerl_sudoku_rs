/// SVG generation for printable Sudoku puzzles
pub struct SvgRenderer;

impl SvgRenderer {
    /// Generate SVG from a puzzle string (81 characters, 0s for empty cells)
    pub fn render_puzzle(puzzle_str: &str) -> Result<String, String> {
        if puzzle_str.len() != 81 {
            return Err("Puzzle must be exactly 81 characters".to_string());
        }

        let cells: Vec<u8> = puzzle_str
            .chars()
            .map(|c| c.to_digit(10).unwrap_or(0) as u8)
            .collect();

        Self::build_svg(&cells)
    }

    fn build_svg(cells: &[u8]) -> Result<String, String> {
        const CELL_SIZE: f64 = 40.0;
        const GRID_SIZE: f64 = CELL_SIZE * 9.0;
        const BORDER_WIDTH: f64 = 2.0;
        const THIN_LINE_WIDTH: f64 = 0.5;
        const FONT_SIZE: f64 = 28.0;

        let mut svg = String::new();
        svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        svg.push_str(&format!(
            "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\n",
            GRID_SIZE + BORDER_WIDTH * 2.0,
            GRID_SIZE + BORDER_WIDTH * 2.0
        ));

        // Background
        svg.push_str(&format!(
            "  <rect width=\"{}\" height=\"{}\" fill=\"white\"/>\n",
            GRID_SIZE + BORDER_WIDTH * 2.0,
            GRID_SIZE + BORDER_WIDTH * 2.0
        ));

        let offset = BORDER_WIDTH;

        // Draw thin lines (between cells)
        for i in 0..=9 {
            if i % 3 != 0 {
                let pos = offset + i as f64 * CELL_SIZE;
                // Vertical line
                svg.push_str(&format!(
                    "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"{}\"/>\n",
                    pos, offset, pos, offset + GRID_SIZE, THIN_LINE_WIDTH
                ));
                // Horizontal line
                svg.push_str(&format!(
                    "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"{}\"/>\n",
                    offset, pos, offset + GRID_SIZE, pos, THIN_LINE_WIDTH
                ));
            }
        }

        // Draw thick lines (box separators)
        for i in 0..=3 {
            let pos = offset + i as f64 * CELL_SIZE * 3.0;
            // Vertical lines
            svg.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"{}\"/>\n",
                pos, offset, pos, offset + GRID_SIZE, BORDER_WIDTH
            ));
            // Horizontal lines
            svg.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"{}\"/>\n",
                offset, pos, offset + GRID_SIZE, pos, BORDER_WIDTH
            ));
        }

        // Draw numbers
        svg.push_str(&format!(
            "  <style>text {{ font-family: Arial; font-size: {}px; text-anchor: middle; dominant-baseline: middle; font-weight: bold; }}</style>\n",
            FONT_SIZE
        ));

        for (idx, &value) in cells.iter().enumerate() {
            if value != 0 {
                let row = idx / 9;
                let col = idx % 9;
                let x = offset + (col as f64 + 0.5) * CELL_SIZE;
                let y = offset + (row as f64 + 0.5) * CELL_SIZE;

                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\">{}</text>\n",
                    x, y, value
                ));
            }
        }

        svg.push_str("</svg>\n");
        Ok(svg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_puzzle() {
        let puzzle =
            "005300000800600020070002005009004302600080001703500600300800070050007006000009700";
        let result = SvgRenderer::render_puzzle(puzzle);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<text")); // Should have some numbers
    }

    #[test]
    fn test_invalid_length() {
        let puzzle = "12345"; // Too short
        let result = SvgRenderer::render_puzzle(puzzle);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_puzzle() {
        let puzzle = "0".repeat(81);
        let result = SvgRenderer::render_puzzle(&puzzle);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(!svg.contains("<text")); // Should have no numbers
    }
}
