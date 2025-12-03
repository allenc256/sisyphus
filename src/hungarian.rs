use arrayvec::ArrayVec;

use crate::game::MAX_BOXES;

pub trait Matrix<T> {
    fn get(&self, row: usize, col: usize) -> T;
    fn shape(&self) -> (usize, usize);
}

impl<T: Copy, const N: usize, const M: usize> Matrix<T> for [[T; M]; N] {
    fn get(&self, row: usize, col: usize) -> T {
        self[row][col]
    }

    fn shape(&self) -> (usize, usize) {
        (N, M)
    }
}

pub struct ArrayMatrix<T, const CAP: usize> {
    data: ArrayVec<T, CAP>,
    rows: usize,
    cols: usize,
}

impl<T: Copy, const CAP: usize> ArrayMatrix<T, CAP> {
    pub fn new(rows: usize, cols: usize) -> Self {
        ArrayMatrix {
            data: ArrayVec::new(),
            rows,
            cols,
        }
    }

    pub fn push(&mut self, item: T) {
        debug_assert!(self.data.len() < self.rows * self.cols);
        self.data.push(item);
    }
}

impl<T: Copy, const CAP: usize> Matrix<T> for ArrayMatrix<T, CAP> {
    fn get(&self, row: usize, col: usize) -> T {
        debug_assert!(row < self.rows && col < self.cols);
        self.data[row * self.cols + col]
    }

    fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
}

// Reference: Andrey Lopatin (https://cp-algorithms.com/graph/hungarian-algorithm.html).
pub fn hungarian_algorithm(a: &impl Matrix<u16>) -> u16 {
    const INF: i32 = u16::MAX as i32 + 1;

    let (n, m) = a.shape();
    assert!(n == m);

    // 1-indexed arrays with dummy 0 element
    let mut u = new_buffer::<i32>(n, 0);
    let mut v = new_buffer::<i32>(m, 0);
    let mut p = new_buffer::<usize>(m, 0);
    let mut way = new_buffer::<usize>(m, 0);

    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut minv = new_buffer::<i32>(m, INF);
        let mut used = new_buffer::<bool>(m, false);

        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = INF;
            let mut j1 = 0;

            for j in 1..=m {
                if !used[j] {
                    let cur = a.get(i0 - 1, j - 1) as i32 - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }

            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }

            j0 = j1;

            if p[j0] == 0 {
                break;
            }
        }

        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;

            if j0 == 0 {
                break;
            }
        }
    }

    u16::try_from(-v[0]).unwrap_or(u16::MAX)
}

fn new_buffer<T: Copy>(n: usize, initial_value: T) -> ArrayVec<T, { MAX_BOXES + 1 }> {
    (0..=n).map(|_| initial_value).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hungarian_algorithm() {
        let a = [[8, 4, 7], [5, 2, 3], [9, 4, 8]];
        let cost = hungarian_algorithm(&a);
        assert_eq!(cost, 15);
    }
}
