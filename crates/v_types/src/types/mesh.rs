use crate::AABB;
use crate::VolitionError;
use crate::util::*;

/// SRIV
/// https://www.saintsrowmods.com/forum/threads/crunched-mesh-formats.15962/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[allow(dead_code)]
enum VertexAttributeTypes {
    Invalid = -1,

    // Floating Point Types
    Float1 = 0,
    Float2,
    Float3,
    Float4,

    // Half Float Types
    Half2,
    Half4,

    // Byte Types
    UByte4,
    UByte4N,

    // Short Types
    Short2N,
    Short4N,
    Short2,
    Short4,

    // Compressed Normal Meta Types
    CNormal,
    CTangent,

    // Color Meta Types
    Color,

    // Compressed Position Meta Types
    CPosition,
    XCposition,

    NumRlVertexAttributeTypes,
}

/// A mesh that gets its data from the GPU chunk
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Mesh {
    pub aabb: AABB,
    pub unk_18: i32,
    pub num_submeshes: i16,
    pub unk_1e: i16,
    pub ptr_submesh_a: i32,
    pub ptr_submesh_b: i32,
}

impl Mesh {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            aabb: AABB::from_data(buf)?,
            unk_18: read_i32_le(buf, 0x18),
            num_submeshes: read_i16_le(buf, 0x1c),
            unk_1e: read_i16_le(buf, 0x1e),
            ptr_submesh_a: read_i32_le(buf, 0x20),
            ptr_submesh_b: read_i32_le(buf, 0x24),
        })
    }

    /// # Arguments: buf must be sliced to start after Mesh
    /// Return: (data_a, data_b, len)
    /// len is distance from start of buf to end of data.
    pub fn read_submeshes(
        &self,
        buf: &[u8],
    ) -> Result<
        (
            Vec<(Submesh, Vec<Surface>)>,
            Vec<(Submesh, Vec<Surface>)>,
            usize,
        ),
        VolitionError,
    > {
        let mut data_offset = 0;

        let num_submeshes = self.num_submeshes as usize;
        let mut headers_a = Vec::with_capacity(num_submeshes);
        let mut headers_b = Vec::with_capacity(num_submeshes);

        if self.ptr_submesh_a != 0 {
            for _ in 0..num_submeshes {
                let sm = Submesh::from_data(&buf[data_offset..])?;
                data_offset += size_of::<Submesh>();
                headers_a.push(sm);
            }
        }

        if self.ptr_submesh_b != 0 {
            for _ in 0..num_submeshes {
                let sm = Submesh::from_data(&buf[data_offset..])?;
                data_offset += size_of::<Submesh>();
                headers_b.push(sm);
            }
        }

        let mut submeshes_a = Vec::with_capacity(num_submeshes);
        let mut submeshes_b = Vec::with_capacity(num_submeshes);

        for header in headers_a {
            let (surfaces, len) = header.read_surfaces(&buf[data_offset..])?;
            submeshes_a.push((header, surfaces));
            data_offset += len;
        }

        for header in headers_b {
            let (surfaces, len) = header.read_surfaces(&buf[data_offset..])?;
            submeshes_b.push((header, surfaces));
            data_offset += len;
        }

        Ok((submeshes_a, submeshes_b, data_offset))
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Submesh {
    pub unk_00: i16,
    pub num_surfaces: i16,
    pub unk_04: i32,
    pub unk_08: i32,
    pub unk_0c: i32,
}

impl Submesh {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            unk_00: read_i16_le(buf, 0x0),
            num_surfaces: read_i16_le(buf, 0x2),
            unk_04: read_i32_le(buf, 0x4),
            unk_08: read_i32_le(buf, 0x8),
            unk_0c: read_i32_le(buf, 0xc),
        })
    }

    /// # Arguments: buf must be sliced to start immediately after Submeshes
    /// Return: (data, len)
    /// len is distance from start of buf to end of data.
    pub fn read_surfaces(&self, buf: &[u8]) -> Result<(Vec<Surface>, usize), VolitionError> {
        let mut data_offset = 0;
        let num_surfaces = self.num_surfaces as usize;
        let mut surfaces = Vec::with_capacity(num_surfaces);
        for _ in 0..num_surfaces {
            surfaces.push(Surface::from_data(&buf[data_offset..])?);
            data_offset += size_of::<Surface>();
        }
        Ok((surfaces, data_offset))
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Surface {
    /// Which vertex buffer to use
    pub vbuf: i32,
    /// First index
    pub start_index: i32,
    /// First vertex
    pub start_vertex: i32,
    pub num_indices: i16,
    pub material: i16,
}

impl Surface {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            vbuf: read_i32_le(buf, 0x0),
            start_index: read_i32_le(buf, 0x4),
            start_vertex: read_i32_le(buf, 0x8),
            num_indices: read_i16_le(buf, 0xc),
            material: read_i16_le(buf, 0xe),
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MeshData {
    pub mesh_type: i16,
    pub num_vertex_buffers: u16,
    pub num_indices: u32,
    pub runtime_08: i32,
    pub runtime_0c: i32,
    pub runtime_10: u32,
}

impl MeshData {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let runtime_08 = read_i32_le(buf, 0x8);
        if runtime_08 != -1 {
            return Err(VolitionError::UnexpectedValue {
                field: "MeshData::runtime_08",
                expected: -1,
                got: runtime_08,
            });
        }

        let runtime_0c = read_i32_le(buf, 0xc);
        if runtime_0c != -1 {
            return Err(VolitionError::UnexpectedValue {
                field: "MeshData::runtime_0c",
                expected: -1,
                got: runtime_0c,
            });
        }

        Ok(Self {
            mesh_type: read_i16_le(buf, 0x0),
            num_vertex_buffers: read_u16_le(buf, 0x2),
            num_indices: read_u32_le(buf, 0x4),
            runtime_08,
            runtime_0c,
            runtime_10: read_u32_le(buf, 0x10),
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct VertexBuffer {
    /// Probably
    pub vertex_format: u8,
    pub num_uvs: u8,
    pub stride: u16,
    pub num_vertices: i32,
    pub ptr_render_data: i32,
    pub unk_0c: i32,
}

impl VertexBuffer {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let unk_08 = read_i32_le(buf, 0x8);
        if unk_08 != -1 {
            return Err(VolitionError::UnexpectedValue {
                field: "VertexBuffer::unk_08",
                expected: -1,
                got: unk_08,
            });
        }

        Ok(Self {
            vertex_format: buf[0],
            num_uvs: buf[1],
            stride: read_u16_le(buf, 0x2),
            num_vertices: read_i32_le(buf, 0x4),
            ptr_render_data: unk_08,
            unk_0c: read_i32_le(buf, 0xc),
        })
    }
}
