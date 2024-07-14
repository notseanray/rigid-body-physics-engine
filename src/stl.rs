// modified from https://github.com/hmeyer/stl_io/blob/master/src/lib.rs
use gxhash::{HashMap, HashMapExt};
use std::io::BufRead;
use std::io::BufWriter;
use std::io::{BufReader, Read, Result, Write};

static DEFAULT_EPSILON: f32 = 1e-6;

#[derive(Default, Debug, Clone, Copy)]
pub struct Vec3<F>([F; 3]);

impl<F> Vec3<F> {
    /// Constructor from array.
    pub fn new(v: [F; 3]) -> Self {
        Self(v)
    }
}

impl<F> From<Vec3<F>> for [F; 3] {
    fn from(v: Vec3<F>) -> Self {
        v.0
    }
}

impl<F> std::ops::Index<usize> for Vec3<F> {
    type Output = F;
    fn index(&self, i: usize) -> &Self::Output {
        assert!(i < 3);
        &self.0[i]
    }
}

// triangle corners
pub type Vertex = Vec3<f32>;
// normal vec for triangle surface
pub type NormalV = Vec3<f32>;

pub struct Triangle {
    /// Normal vector of the Triangle.
    pub normal: NormalV,
    /// The three vertices of the Triangle.
    pub vertices: [Vertex; 3],
}

/*
impl<F> Eq for Vec3<F> {
    fn assert_receiver_is_total_eq(&self) {

    }
}
*/

macro_rules! eq_e {
    ($v1:expr, $v2:expr, $ep:expr) => {
        ($v2 - $v2).abs() < $ep
    };
}

impl PartialEq for Vec3<f32> {
    fn eq(&self, other: &Self) -> bool {
        eq_e!(self[0], other[0], DEFAULT_EPSILON)
            && eq_e!(self[1], other[1], DEFAULT_EPSILON)
            && eq_e!(self[2], other[2], DEFAULT_EPSILON)
    }
}

#[inline(always)]
fn tri_area(a: Vertex, b: Vertex, c: Vertex) -> f32 {
    fn cross(a: Vertex, b: Vertex) -> Vertex {
        let x = a[1] * b[2] - a[2] * b[1];
        let y = a[2] * b[0] - a[0] * b[2];
        let z = a[0] * b[1] - a[1] * b[0];
        Vertex::new([x, y, z])
    }
    fn sub(a: Vertex, b: Vertex) -> Vertex {
        let x = a[0] - b[0];
        let y = a[1] - b[1];
        let z = a[2] - b[2];
        Vertex::new([x, y, z])
    }
    fn length(v: Vertex) -> f32 {
        (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
    }

    length(cross(sub(c, b), sub(a, b))) * 0.5
}

/// STL Triangle in indexed form, consisting of a normal and three indices to vertices in the
/// vertex list.
/// This format is more compact, since in real world Meshes Triangles usually share vertices with
/// other Triangles.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedTriangle {
    /// Normal vector of the Triangle.
    pub normal: NormalV,
    /// The indexed to the three vertices of the Triangle, when this is used in an
    /// [IndexedMesh](struct.IndexedMesh.html).
    pub vertices: [usize; 3],
}

/// STL Mesh in indexed form, consisting of a list of [Vertices](type.Vertex.html) and a list of
/// [indexed Triangles](struct.IndexedTriangle.html).
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedMesh {
    /// List of vertices.
    pub vertices: Vec<Vertex>,
    /// List of triangles..
    pub faces: Vec<IndexedTriangle>,
}

impl IndexedMesh {
    /// Checks that the Mesh has no holes and no zero-area faces.
    /// Also makes sure that all triangles are faced in the same direction.
    pub fn validate(&self) -> Result<()> {
        let mut unconnected_edges: HashMap<(usize, usize), (usize, usize, usize)> = HashMap::new();

        for (fi, face) in self.faces.iter().enumerate() {
            {
                let a = self.vertices[face.vertices[0]];
                let b = self.vertices[face.vertices[1]];
                let c = self.vertices[face.vertices[2]];

                let area = tri_area(a, b, c);

                if area < f32::EPSILON {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("face #{} has a zero-area face", fi),
                    ));
                }
            }

            for i in 0..3 {
                let u = face.vertices[i];
                let v = face.vertices[(i + 1) % 3];

                if unconnected_edges.contains_key(&(v, u)) {
                    unconnected_edges.remove(&(v, u));
                } else {
                    unconnected_edges.insert((u, v), (fi, i, (i + 1) % 3));
                }
            }
        }

        if let Option::Some((fi, i1, i2)) = unconnected_edges.values().next() {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "did not find facing edge for face #{}, edge #v{} -> #v{}",
                    fi, i1, i2
                ),
            ))
        } else {
            Ok(())
        }
    }
    // TODO load from mesh here
}

/// Write to std::io::Write as documented in
/// [Wikipedia](https://en.wikipedia.org/wiki/STL_(file_format)#Binary_STL).
///
/// ```
/// use stl_io::{Vertex, Normal};
/// let mesh = [stl_io::Triangle { normal: Normal::new([1.0, 0.0, 0.0]),
///                                vertices: [Vertex::new([0.0, -1.0, 0.0]),
///                                           Vertex::new([0.0, 1.0, 0.0]),
///                                           Vertex::new([0.0, 0.0, 0.5])]}];
/// let mut binary_stl = Vec::<u8>::new();
/// stl_io::write_stl(&mut binary_stl, mesh.iter()).unwrap();
/// ```
pub fn write_stl<T, W, I>(writer: &mut W, mesh: I) -> Result<()>
where
    W: std::io::Write,
    I: std::iter::ExactSizeIterator<Item = T>,
    T: std::borrow::Borrow<Triangle>,
{
    let mut writer = BufWriter::new(writer);

    // Write 80 byte header
    writer.write_all(&[0u8; 80])?;
    writer.write(&u32::to_le_bytes(mesh.len() as u32))?;
    for t in mesh {
        let t = t.borrow();
        for f in &t.normal.0 {
            writer.write(&f32::to_le_bytes(*f as f32))?;
        }
        for &p in &t.vertices {
            for c in &p.0 {
                writer.write(&f32::to_le_bytes(*c as f32))?;
            }
        }
        // Attribute byte count
        writer.write(&u16::to_le_bytes(0))?;
    }
    writer.flush()
}

/// Attempts to read either ascii or binary STL from std::io::Read.
///
/// ```
/// let mut reader = std::io::Cursor::new(
///     b"solid foobar
///       facet normal 0.1 0.2 0.3
///           outer loop
///               vertex 1 2 3
///               vertex 4 5 6e-15
///               vertex 7 8 9.87654321
///           endloop
///       endfacet
///       endsolid foobar".to_vec());
/// let mesh = stl_io::read_stl(&mut reader).unwrap();
/// ```
pub fn read_stl<R>(read: &mut R) -> Result<IndexedMesh>
where
    R: std::io::Read + std::io::Seek,
{
    create_stl_reader(read)?.as_indexed_triangles()
}

/// Attempts to create a [TriangleIterator](trait.TriangleIterator.html) for either ascii or binary
/// STL from std::io::Read.
///
/// ```
/// let mut reader = std::io::Cursor::new(b"solid foobar
/// facet normal 1 2 3
///     outer loop
///         vertex 7 8 9
///         vertex 4 5 6
///         vertex 7 8 9
///     endloop
/// endfacet
/// endsolid foobar".to_vec());
/// let stl = stl_io::create_stl_reader(&mut reader).unwrap();
/// ```
pub fn create_stl_reader<'a, R>(
    read: &'a mut R,
) -> Result<Box<dyn TriangleIterator<Item = Result<Triangle>> + 'a>>
where
    R: std::io::Read + std::io::Seek,
{
    match AsciiStlReader::probe(read) {
        Ok(()) => AsciiStlReader::create_triangle_iterator(read),
        Err(_) => BinaryStlReader::create_triangle_iterator(read),
    }
}

/// Struct for binary STL reader.
pub struct BinaryStlReader<'a> {
    reader: Box<dyn std::io::Read + 'a>,
    index: usize,
    size: usize,
}

impl<'a> BinaryStlReader<'a> {
    /// Factory to create a new BinaryStlReader from read.
    pub fn create_triangle_iterator(
        read: &'a mut dyn (std::io::Read),
    ) -> Result<Box<dyn TriangleIterator<Item = Result<Triangle>> + 'a>> {
        let mut reader = Box::new(BufReader::new(read));
        reader.read_exact(&mut [0u8; 80])?;
        let mut f32_buf = [0; 4];
        reader.read(&mut f32_buf)?;
        let num_faces: u32 = u32::from_le_bytes(f32_buf);
        Ok(Box::new(BinaryStlReader {
            reader,
            index: 0,
            size: num_faces as usize,
        })
            as Box<dyn TriangleIterator<Item = Result<Triangle>>>)
    }

    fn next_face(&mut self) -> Result<Triangle> {
        let mut normal = NormalV::default();
        for f in &mut normal.0 {
            let mut f32_buf = [0; 4];
            self.reader.read(&mut f32_buf)?;
            *f = f32::from_le_bytes(f32_buf);
        }
        let mut face = [Vertex::default(); 3];
        for vertex in &mut face {
            for c in vertex.0.iter_mut() {
                let mut f32_buf = [0; 4];
                self.reader.read(&mut f32_buf)?;
                *c = f32::from_le_bytes(f32_buf);
            }
        }
        let mut u16_buf = [0; 4];
        self.reader.read(&mut u16_buf)?;
        Ok(Triangle {
            normal,
            vertices: face,
        })
    }
}

impl<'a> std::iter::Iterator for BinaryStlReader<'a> {
    type Item = Result<Triangle>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.size {
            self.index += 1;
            return Some(self.next_face());
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size - self.index, Some(self.size - self.index))
    }
}

/// Iterates over all Triangles in a STL.
pub trait TriangleIterator: std::iter::Iterator<Item = Result<Triangle>> {
    /// Consumes this iterator and generates an [indexed Mesh](struct.IndexedMesh.html).
    ///
    /// ```
    /// let mut reader = std::io::Cursor::new(b"solid foobar
    /// facet normal 1 2 3
    ///     outer loop
    ///         vertex 7 8 9
    ///         vertex 4 5 6
    ///         vertex 7 8 9
    ///     endloop
    /// endfacet
    /// endsolid foobar".to_vec());
    /// let mut stl = stl_io::create_stl_reader(&mut reader).unwrap();
    /// let indexed_mesh = stl.as_indexed_triangles().unwrap();
    /// ```
    fn as_indexed_triangles(&mut self) -> Result<IndexedMesh> {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let mut vertex_to_index = std::collections::HashMap::new();
        // Do not reserve memory in those structures based on size_hint, because we might have just
        // read bogus data.
        let mut vertex_indices = [0; 3];
        for t in self {
            let t = t?;
            for (i, vertex) in t.vertices.iter().enumerate() {
                // This is ugly, but f32 has no Eq and no Hash.
                let bitpattern = unsafe { std::mem::transmute::<[f32; 3], [u32; 3]>(vertex.0) };
                let index = *vertex_to_index
                    .entry(bitpattern)
                    .or_insert_with(|| vertices.len());
                if index == vertices.len() {
                    vertices.push(*vertex);
                }
                vertex_indices[i] = index;
            }
            triangles.push(IndexedTriangle {
                normal: t.normal,
                vertices: vertex_indices,
            });
        }
        vertices.shrink_to_fit();
        triangles.shrink_to_fit();
        Ok(IndexedMesh {
            vertices,
            faces: triangles,
        })
    }
}

/// Struct for ascii STL reader.
pub struct AsciiStlReader<'a> {
    lines: Box<dyn std::iter::Iterator<Item = Result<Vec<String>>> + 'a>,
}

impl<'a> TriangleIterator for BinaryStlReader<'a> {}
impl<'a> TriangleIterator for AsciiStlReader<'a> {}

impl<'a> std::iter::Iterator for AsciiStlReader<'a> {
    type Item = Result<Triangle>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_face() {
            Ok(None) => None,
            Ok(Some(t)) => Some(Ok(t)),
            Err(e) => Some(Err(e)),
        }
    }
}

impl<'a> AsciiStlReader<'a> {
    /// Test whether or not read is an ascii STL file.
    pub fn probe<F: std::io::Read + std::io::Seek>(read: &mut F) -> Result<()> {
        let mut header = String::new();
        let maybe_read_error = BufReader::new(&mut *read).read_line(&mut header);
        // Try to seek back to start before evaluating potential read errors.
        read.seek(std::io::SeekFrom::Start(0))?;
        maybe_read_error?;
        if !header.starts_with("solid ") {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ascii STL does not start with \"solid \"",
            ))
        } else {
            Ok(())
        }
    }
    /// Factory to create a new ascii STL Reader from read.
    pub fn create_triangle_iterator(
        read: &'a mut dyn (std::io::Read),
    ) -> Result<Box<dyn TriangleIterator<Item = Result<Triangle>> + 'a>> {
        let mut lines = BufReader::new(read).lines();
        match lines.next() {
            Some(Err(e)) => return Err(e),
            Some(Ok(ref line)) if !line.starts_with("solid ") => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "ascii STL does not start with \"solid \"",
                ))
            }
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "empty file?",
                ))
            }
            _ => {}
        }
        let lines = lines
            .map(|result| {
                result.map(|l| {
                    // Make lines into iterator over vectors of tokens
                    l.split_whitespace()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                })
            })
            // filter empty lines.
            .filter(|result| result.is_err() || (!result.as_ref().unwrap().is_empty()));
        Ok(Box::new(AsciiStlReader {
            lines: Box::new(lines),
        })
            as Box<dyn TriangleIterator<Item = Result<Triangle>>>)
    }
    // Tries to read a triangle.
    fn next_face(&mut self) -> Result<Option<Triangle>> {
        let face_header: Option<Result<Vec<String>>> = self.lines.next();
        if face_header.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "EOF while expecting facet or endsolid.",
            ));
        }
        let face_header = face_header.unwrap()?;
        if !face_header.is_empty() && face_header[0] == "endsolid" {
            return Ok(None);
        }
        if face_header.len() != 5 || face_header[0] != "facet" || face_header[1] != "normal" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid facet header: {:?}", face_header),
            ));
        }
        let mut result_normal = NormalV::default();
        AsciiStlReader::tokens_to_f32(&face_header[2..5], &mut result_normal.0[0..3])?;
        self.expect_static(&["outer", "loop"])?;
        let mut result_vertices = [Vertex::default(); 3];
        for vertex_result in &mut result_vertices {
            if let Some(line) = self.lines.next() {
                let line = line?;
                if line.len() != 4 || line[0] != "vertex" {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("vertex f32 f32 f32, got {:?}", line),
                    ));
                }
                AsciiStlReader::tokens_to_f32(&line[1..4], &mut vertex_result.0[0..3])?;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "EOF while expecting vertex",
                ));
            }
        }
        self.expect_static(&["endloop"])?;
        self.expect_static(&["endfacet"])?;
        Ok(Some(Triangle {
            normal: result_normal,
            vertices: result_vertices,
        }))
    }
    fn tokens_to_f32(tokens: &[String], output: &mut [f32]) -> Result<()> {
        assert_eq!(tokens.len(), output.len());
        for i in 0..tokens.len() {
            let f = tokens[i]
                .parse::<f32>()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
            if !f.is_finite() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("expected finite f32, got {} which is {:?}", f, f.classify()),
                ));
            }
            output[i] = f;
        }
        Ok(())
    }
    fn expect_static(&mut self, expectation: &[&str]) -> Result<()> {
        if let Some(line) = self.lines.next() {
            let line = line?;
            if line != expectation {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("expected {:?}, got {:?}", expectation, line),
                ));
            }
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("EOF while expecting {:?}", expectation),
            ));
        }
        Ok(())
    }
}
