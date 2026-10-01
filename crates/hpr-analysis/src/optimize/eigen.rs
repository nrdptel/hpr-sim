//! The eigenvalues and eigenvectors of a symmetric matrix, by Jacobi's cyclic method.
//!
//! CMA-ES samples from a normal distribution with covariance `C`, which needs `C = B D² Bᵀ`: the
//! eigenvectors `B` and the square roots `D` of the eigenvalues. The matrices are small (one row
//! per design variable), so the simplest accurate method serves: Jacobi's, which turns `C` into a
//! diagonal matrix by plane rotations, each zeroing one off-diagonal pair. The rotation is the
//! 2-by-2 symmetric Schur decomposition and the sweep order the cyclic one ("Jacobi methods",
//! G. H. Golub and C. F. Van Loan, *Matrix Computations*, 4th ed., 2013, §8.5). A pair is skipped once
//! `|a_pq| ≤ ε √|a_pp a_qq|`, the stopping test of J. Demmel and K. Veselić, "Jacobi's method is
//! more accurate than QR", *SIAM J. Matrix Anal. Appl.* 13(4), 1204–1245 (1992),
//! <https://doi.org/10.1137/0613074>: on a positive definite matrix it keeps each eigenvalue
//! accurate relative to itself, even the smallest of an ill-conditioned covariance.

/// The most sweeps over the off-diagonal pairs. Jacobi's method converges quadratically: a
/// covariance of 20 to 30 rows and a condition number up to 10¹⁶ takes about 12 to 19. The limit
/// only stops a matrix of non-finite numbers from turning forever.
const MAX_SWEEPS: usize = 64;

/// The eigen-decomposition of the symmetric `n × n` matrix `a` (row-major; only the symmetric
/// part matters, as each rotation reads both triangles but keeps them equal).
///
/// Returns the eigenvalues, unsorted, and the eigenvectors as the columns of an `n × n`
/// row-major matrix `v`: `v[i * n + k]` is component `i` of the eigenvector of eigenvalue `k`.
/// So `a = v diag(values) vᵀ`, with `v` orthogonal to rounding.
pub(crate) fn symmetric_eigen(a: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut a = a.to_vec();
    let mut v = vec![0.0; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    for _ in 0..MAX_SWEEPS {
        let mut rotated = false;
        for p in 0..n {
            for q in p + 1..n {
                let apq = a[p * n + q];
                let app = a[p * n + p];
                let aqq = a[q * n + q];
                // Demmel and Veselić's test: the pair is already zero to the diagonal's accuracy.
                if apq.abs() <= f64::EPSILON * (app * aqq).abs().sqrt() {
                    continue;
                }
                rotated = true;
                // Golub and Van Loan's symmetric Schur decomposition: the rotation (c, s) that
                // zeroes a_pq, with t = tan θ the smaller root of t² + 2τt − 1 = 0.
                let tau = (aqq - app) / (2.0 * apq);
                let t = tau.signum() / (tau.abs() + tau.hypot(1.0));
                let c = 1.0 / t.hypot(1.0);
                let s = t * c;
                // A ← A J, then A ← Jᵀ A, with J the identity but for J_pp = J_qq = c,
                // J_pq = s, J_qp = −s; and V ← V J.
                for k in 0..n {
                    let akp = a[k * n + p];
                    let akq = a[k * n + q];
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let apk = a[p * n + k];
                    let aqk = a[q * n + k];
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let vkp = v[k * n + p];
                    let vkq = v[k * n + q];
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
        if !rotated {
            break;
        }
    }
    let values = (0..n).map(|i| a[i * n + i]).collect();
    (values, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `v diag(values) vᵀ`.
    fn rebuild(values: &[f64], v: &[f64], n: usize) -> Vec<f64> {
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                a[i * n + j] = (0..n)
                    .map(|k| v[i * n + k] * values[k] * v[j * n + k])
                    .sum();
            }
        }
        a
    }

    fn assert_orthogonal(v: &[f64], n: usize, tolerance: f64) {
        for i in 0..n {
            for j in 0..n {
                let dot: f64 = (0..n).map(|k| v[k * n + i] * v[k * n + j]).sum();
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (dot - expected).abs() <= tolerance,
                    "columns {i} and {j}: {dot}"
                );
            }
        }
    }

    /// The second-difference matrix (2 on the diagonal, −1 beside it) has the closed-form
    /// eigenvalues `2 − 2 cos(kπ/(n + 1))`, `k = 1 … n`.
    #[test]
    fn second_difference_matrix_has_its_closed_form_eigenvalues() {
        for n in [1, 2, 3, 7, 20] {
            let mut a = vec![0.0; n * n];
            for i in 0..n {
                a[i * n + i] = 2.0;
                if i + 1 < n {
                    a[i * n + i + 1] = -1.0;
                    a[(i + 1) * n + i] = -1.0;
                }
            }
            let (mut values, v) = symmetric_eigen(&a, n);
            values.sort_by(f64::total_cmp);
            for (k, value) in values.iter().enumerate() {
                let exact =
                    2.0 - 2.0 * ((k + 1) as f64 * std::f64::consts::PI / (n + 1) as f64).cos();
                assert!(
                    (value - exact).abs() <= 1e-14,
                    "n {n}, k {k}: {value} vs {exact}"
                );
            }
            assert_orthogonal(&v, n, 1e-14);
        }
    }

    /// A graded covariance, `diag(d) R diag(d)` with `dᵢ` from 1 to 10⁻⁶ and `R` all 0.2 off its
    /// unit diagonal, so its eigenvalues run from about 1 to 10⁻¹². Their product is
    /// `det = Π dᵢ² (1 − r)ⁿ⁻¹ (1 + (n − 1) r)` in closed form, so it holds every eigenvalue,
    /// the smallest included, to rounding relative to itself, as Demmel and Veselić's test
    /// promises. (An absolute residual `‖A x − λ x‖` can't: computing `A x` in floating point
    /// errs by about `ε ‖A‖`, far more than the smallest `λ`.)
    #[test]
    fn graded_covariance_keeps_small_eigenvalues_relatively_accurate() {
        let n = 6;
        let r = 0.2;
        let d: Vec<f64> = (0..n).map(|i| 10f64.powf(-1.2 * i as f64)).collect();
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let rij = if i == j { 1.0 } else { r };
                a[i * n + j] = d[i] * rij * d[j];
            }
        }
        let (values, v) = symmetric_eigen(&a, n);
        assert!(values.iter().all(|&x| x > 0.0));
        assert!(values.iter().copied().fold(f64::INFINITY, f64::min) < 1e-12);
        assert_orthogonal(&v, n, 1e-14);
        let product: f64 = values.iter().product();
        let det = d.iter().map(|di| di * di).product::<f64>()
            * (1.0 - r).powi(n as i32 - 1)
            * (1.0 + (n as f64 - 1.0) * r);
        assert!(
            (product / det - 1.0).abs() <= 1e-13,
            "{product} vs {det}: {}",
            product / det - 1.0
        );
        let trace: f64 = (0..n).map(|i| a[i * n + i]).sum();
        assert!((values.iter().sum::<f64>() - trace).abs() <= 1e-15 * trace);
    }

    /// A random symmetric matrix is rebuilt from its decomposition to rounding.
    #[test]
    fn random_symmetric_matrix_is_rebuilt() {
        let n = 9;
        let mut rng = hpr_core::random::SeededRng::seed_from_u64(7);
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            for j in i..n {
                let x = rng.standard_normal();
                a[i * n + j] = x;
                a[j * n + i] = x;
            }
        }
        let (values, v) = symmetric_eigen(&a, n);
        assert_orthogonal(&v, n, 1e-14);
        for (x, y) in rebuild(&values, &v, n).iter().zip(&a) {
            assert!((x - y).abs() <= 1e-13, "{x} vs {y}");
        }
    }

    /// A diagonal matrix needs no rotation: it comes back exactly, with the identity.
    #[test]
    fn diagonal_matrix_is_returned_exactly() {
        let a = [3.0, 0.0, 0.0, 0.0, 1e-9, 0.0, 0.0, 0.0, 7.0];
        let (values, v) = symmetric_eigen(&a, 3);
        assert_eq!(values, vec![3.0, 1e-9, 7.0]);
        assert_eq!(v, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    }
}
