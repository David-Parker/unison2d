//! 2D Math utilities for FEM simulation
//! All 2x2 matrices are stored in column-major order: [col0.x, col0.y, col1.x, col1.y]

/// 2x2 matrix in column-major order
pub type Mat2 = [f32; 4];


/// Create a 2x2 matrix from column vectors
#[inline]
pub fn mat2_create(a: f32, b: f32, c: f32, d: f32) -> Mat2 {
    [a, b, c, d]
}

/// Create a 2x2 identity matrix
#[inline]
pub fn mat2_identity() -> Mat2 {
    [1.0, 0.0, 0.0, 1.0]
}

/// Compute determinant of 2x2 matrix
#[inline]
pub fn mat2_det(m: &Mat2) -> f32 {
    m[0] * m[3] - m[2] * m[1]
}

/// Compute inverse of 2x2 matrix
#[inline]
pub fn mat2_inv(m: &Mat2) -> Mat2 {
    let d = mat2_det(m);
    if d.abs() < 1e-10 {
        return mat2_identity();
    }
    [m[3] / d, -m[1] / d, -m[2] / d, m[0] / d]
}

/// Transpose 2x2 matrix
#[inline]
pub fn mat2_transpose(m: &Mat2) -> Mat2 {
    [m[0], m[2], m[1], m[3]]
}

/// Compute inverse transpose of 2x2 matrix (F^{-T})
#[inline]
pub fn mat2_inv_transpose(m: &Mat2) -> Mat2 {
    let d = mat2_det(m);
    if d.abs() < 1e-10 {
        return mat2_identity();
    }
    [m[3] / d, -m[2] / d, -m[1] / d, m[0] / d]
}

/// Multiply two 2x2 matrices: A * B
#[inline]
pub fn mat2_mul(a: &Mat2, b: &Mat2) -> Mat2 {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
    ]
}

/// Multiply 2x2 matrix by 2D vector
#[inline]
pub fn mat2_mul_vec(m: &Mat2, v: &[f32; 2]) -> [f32; 2] {
    [m[0] * v[0] + m[2] * v[1], m[1] * v[0] + m[3] * v[1]]
}

/// Add two 2x2 matrices
#[inline]
pub fn mat2_add(a: &Mat2, b: &Mat2) -> Mat2 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

/// Subtract two 2x2 matrices: A - B
#[inline]
pub fn mat2_sub(a: &Mat2, b: &Mat2) -> Mat2 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}

/// Scale 2x2 matrix by scalar
#[inline]
pub fn mat2_scale(m: &Mat2, s: f32) -> Mat2 {
    [m[0] * s, m[1] * s, m[2] * s, m[3] * s]
}

/// Compute trace of 2x2 matrix (sum of diagonal)
#[inline]
pub fn mat2_trace(m: &Mat2) -> f32 {
    m[0] + m[3]
}

/// Compute Frobenius norm squared of 2x2 matrix
#[inline]
pub fn mat2_frobenius_norm_sq(m: &Mat2) -> f32 {
    m[0] * m[0] + m[1] * m[1] + m[2] * m[2] + m[3] * m[3]
}


#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    fn assert_close(actual: f32, expected: f32, msg: &str) {
        assert!((actual - expected).abs() < EPSILON, "{}: expected {}, got {}", msg, expected, actual);
    }

    fn assert_mat2_close(actual: &Mat2, expected: &Mat2, msg: &str) {
        for i in 0..4 {
            assert_close(actual[i], expected[i], &format!("{}[{}]", msg, i));
        }
    }

    #[test]
    fn test_mat2_identity() {
        assert_mat2_close(&mat2_identity(), &[1.0, 0.0, 0.0, 1.0], "identity");
    }

    #[test]
    fn test_mat2_det() {
        assert_close(mat2_det(&[1.0, 0.0, 0.0, 1.0]), 1.0, "det(I)");
        assert_close(mat2_det(&[2.0, 0.0, 0.0, 3.0]), 6.0, "det(scale)");
    }

    #[test]
    fn test_mat2_inv() {
        let a = [3.0, 1.0, 2.0, 4.0];
        let a_inv = mat2_inv(&a);
        let product = mat2_mul(&a, &a_inv);
        assert_mat2_close(&product, &mat2_identity(), "A * A^-1 = I");
    }

    #[test]
    fn test_mat2_mul() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let i = mat2_identity();
        assert_mat2_close(&mat2_mul(&i, &a), &a, "I * A = A");
        assert_mat2_close(&mat2_mul(&a, &i), &a, "A * I = A");
    }

    #[test]
    fn test_mat2_transpose() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let at = mat2_transpose(&a);
        assert_mat2_close(&at, &[1.0, 3.0, 2.0, 4.0], "transpose");
        // Double transpose = original
        assert_mat2_close(&mat2_transpose(&at), &a, "double transpose");
    }

    #[test]
    fn test_mat2_inv_transpose() {
        let a = [3.0, 1.0, 2.0, 4.0];
        let a_inv_t = mat2_inv_transpose(&a);
        // inv_transpose = transpose(inv) = inv(transpose)
        let a_inv = mat2_inv(&a);
        let a_inv_then_t = mat2_transpose(&a_inv);
        assert_mat2_close(&a_inv_t, &a_inv_then_t, "inv_transpose = transpose(inv)");
    }

    #[test]
    fn test_mat2_add_sub() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [5.0, 6.0, 7.0, 8.0];
        assert_mat2_close(&mat2_add(&a, &b), &[6.0, 8.0, 10.0, 12.0], "add");
        assert_mat2_close(&mat2_sub(&a, &b), &[-4.0, -4.0, -4.0, -4.0], "sub");
    }

    #[test]
    fn test_mat2_scale() {
        let a = [1.0, 2.0, 3.0, 4.0];
        assert_mat2_close(&mat2_scale(&a, 2.0), &[2.0, 4.0, 6.0, 8.0], "scale");
    }

    #[test]
    fn test_mat2_trace() {
        assert_close(mat2_trace(&mat2_identity()), 2.0, "trace(I)");
        assert_close(mat2_trace(&[3.0, 1.0, 2.0, 5.0]), 8.0, "trace");
    }

    #[test]
    fn test_mat2_frobenius_norm_sq() {
        assert_close(mat2_frobenius_norm_sq(&mat2_identity()), 2.0, "frobenius(I)");
        assert_close(mat2_frobenius_norm_sq(&[1.0, 2.0, 3.0, 4.0]), 30.0, "frobenius");
    }

    #[test]
    fn test_mat2_mul_vec() {
        let m = [1.0, 2.0, 3.0, 4.0];
        let v = [1.0, 1.0];
        let result = mat2_mul_vec(&m, &v);
        assert_close(result[0], 4.0, "mul_vec[0]");
        assert_close(result[1], 6.0, "mul_vec[1]");
    }

}
