use gridcraft_core::{CellRef, RangeRef};

/// Rectangle subtraction stays proportional to the number of areas, including full columns.
pub(super) fn subtract(ranges: &[RangeRef], cut: RangeRef) -> Vec<RangeRef> {
    let mut result = Vec::new();
    for area in ranges {
        let Some(hit) = area.intersection(&cut) else {
            result.push(*area);
            continue;
        };
        if area.start.row < hit.start.row {
            result.push(RangeRef::new(area.start, CellRef::new(hit.start.row - 1, area.end.col)));
        }
        if hit.end.row < area.end.row {
            result.push(RangeRef::new(CellRef::new(hit.end.row + 1, area.start.col), area.end));
        }
        if area.start.col < hit.start.col {
            result.push(RangeRef::new(CellRef::new(hit.start.row, area.start.col), CellRef::new(hit.end.row, hit.start.col - 1)));
        }
        if hit.end.col < area.end.col {
            result.push(RangeRef::new(CellRef::new(hit.start.row, hit.end.col + 1), CellRef::new(hit.end.row, area.end.col)));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rectangle_subtraction_matches_cell_membership_without_overlapping_pieces() {
        let area = RangeRef::parse("A1:E5").unwrap();
        for a in 0..5 {
            for b in a..5 {
                for c in 0..5 {
                    for d in c..5 {
                        let cut = RangeRef::new(CellRef::new(a, c), CellRef::new(b, d));
                        let result = subtract(&[area], cut);
                        assert!(result.len() <= 4);
                        for row in 0..5 {
                            for col in 0..5 {
                                let cell = CellRef::new(row, col);
                                assert_eq!(result.iter().filter(|r| r.contains(cell)).count(), usize::from(!cut.contains(cell)));
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn full_columns_remove_only_the_requested_column_and_preserve_other_areas() {
        let ranges = [RangeRef::cols(0, 2), RangeRef::parse("F4:G5").unwrap()];
        assert_eq!(subtract(&ranges, RangeRef::cols(1, 1)), vec![RangeRef::cols(0, 0), RangeRef::cols(2, 2), ranges[1]]);
        let edge = RangeRef::rows(gridcraft_core::MAX_ROWS - 1, gridcraft_core::MAX_ROWS - 1);
        assert!(subtract(&[edge], edge).is_empty());
    }
}
