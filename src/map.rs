#[allow(unused_imports)]
use std::io::Write;

const MAP_WIDTH: usize = 200;
const MAP_HEIGHT: usize = 200;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Cell {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub g: Option<String>,
}

pub struct MapState {
    width: usize,
    height: usize,
    cells: Vec<Option<String>>,
}

impl MapState {
    pub fn new() -> Self {
        Self {
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            cells: vec![None; MAP_WIDTH * MAP_HEIGHT],
        }
    }

    pub async fn update_map(&mut self, cells: &[Cell], logger: &crate::logger::Logger) {
        logger.log("updateMap\n").await;

        let origin_x = (self.width / 2) as i32;
        let origin_y = (self.height / 2) as i32;
        let mut map_index: i32 = 0;

        for cell in cells {
            if let (Some(x), Some(y)) = (cell.x, cell.y) {
                map_index = origin_x + x + (self.width as i32) * (origin_y + y);
            } else {
                map_index += 1;
            }

            if let Some(g) = &cell.g {
                if map_index >= 0 && (map_index as usize) < self.cells.len() {
                    self.cells[map_index as usize] = Some(g.clone());
                }
            }
        }
    }

    pub fn print_map<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut min_x = self.width;
        let mut max_x = 0;
        let mut min_y = self.height;
        let mut max_y = 0;

        for y in 0..self.height {
            for x in 0..self.width {
                let i = x + y * self.width;
                if self.cells[i].is_some() {
                    if x < min_x {
                        min_x = x;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }

        if min_x > max_x || min_y > max_y {
            writeln!(writer, "Map is empty")?;
            return Ok(());
        }

        writeln!(writer, "{},{} - {},{}", min_x, min_y, max_x, max_y)?;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let i = x + y * self.width;
                match &self.cells[i] {
                    None => {
                        write!(writer, " ")?;
                    }
                    Some(g) => {
                        write!(writer, "{}", g)?;
                    }
                }
            }
            writeln!(writer)?;
        }
        writer.flush()?;
        Ok(())
    }
}
