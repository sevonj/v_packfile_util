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

pub type Submeshes = Vec<(SubmeshHeader, Vec<Surface>)>;

#[derive(Debug, Clone)]
pub struct Mesh {
    pub header: MeshHeader,
    pub submeshes: Vec<Submesh>,
}

impl Mesh {
    pub fn from_data(buf: &[u8], unk_2c: i32) -> Result<(Self, usize), VolitionError> {
        let mut data_offset = 0;

        let header = MeshHeader::from_data(&buf[data_offset..])?;
        data_offset += size_of::<MeshHeader>();

        let num_submeshes = header.num_submeshes as usize;

        if unk_2c != 0 {
            data_offset += 20;
        }

        align_16(&mut data_offset);
        let (sma, smb, sm_len) = header.read_submeshes(&buf[data_offset..])?;
        data_offset += sm_len;

        let mut submeshes = Vec::with_capacity(num_submeshes);

        for i in 0..num_submeshes {
            let has_gpu = !sma.is_empty();
            let has_cpu = !smb.is_empty();

            let (gpu_data, gpu_vbufs) = if has_gpu {
                let gpu_data = MeshData::from_data(&buf[data_offset..])?;
                data_offset += size_of::<MeshData>();
                if gpu_data.mesh_type != 0 {
                    return Err(VolitionError::UnexpectedValue {
                        field: "MeshData::mesh_type (gpu)",
                        expected: 0,
                        got: gpu_data.mesh_type as i32,
                    });
                }
                let num_gpu_vbufs = gpu_data.num_vertex_buffers as usize;

                let mut gpu_vbufs = Vec::with_capacity(num_gpu_vbufs);
                for _ in 0..gpu_data.num_vertex_buffers {
                    gpu_vbufs.push(VertexBuffer::from_data(&buf[data_offset..])?);
                    data_offset += size_of::<VertexBuffer>();
                }
                (Some(gpu_data), gpu_vbufs)
            } else {
                (None, vec![])
            };

            let (cpu_data, cpu_vbufs, cpu_vdata, cpu_idata) = if has_cpu {
                let cpu_data = MeshData::from_data(&buf[data_offset..])?;
                data_offset += size_of::<MeshData>();
                if cpu_data.mesh_type != 7 {
                    return Err(VolitionError::UnexpectedValue {
                        field: "MeshData::mesh_type (cpu)",
                        expected: 7,
                        got: cpu_data.mesh_type as i32,
                    });
                }
                let num_cpu_vbufs = cpu_data.num_vertex_buffers as usize;
                let num_cpu_indices = cpu_data.num_indices as usize;

                let mut cpu_vbufs = Vec::with_capacity(num_cpu_vbufs);
                for _ in 0..cpu_data.num_vertex_buffers {
                    cpu_vbufs.push(VertexBuffer::from_data(&buf[data_offset..])?);
                    data_offset += size_of::<VertexBuffer>();
                }

                align_16(&mut data_offset);
                let mut len_cpu_vdata = 0;
                for vbuf in &cpu_vbufs {
                    len_cpu_vdata += vbuf.num_vertices as usize * vbuf.stride as usize;
                    align_16(&mut len_cpu_vdata);
                }
                let cpu_vdata = buf[data_offset..(data_offset + len_cpu_vdata)].to_vec();
                data_offset += len_cpu_vdata;

                let len_cpu_idata = num_cpu_indices * 2;
                let cpu_idata = buf[data_offset..(data_offset + len_cpu_idata)].to_vec();
                data_offset += len_cpu_idata;

                (Some(cpu_data), cpu_vbufs, cpu_vdata, cpu_idata)
            } else {
                (None, vec![], vec![], vec![])
            };

            submeshes.push(Submesh {
                submesh_a: sma.get(i).cloned(),
                submesh_b: smb.get(i).cloned(),
                gpu_data,
                gpu_vbufs,
                cpu_data,
                cpu_vbufs,
                cpu_vdata,
                cpu_idata,
            });
        }

        Ok((Self { header, submeshes }, data_offset))
    }
}

#[derive(Debug, Clone)]
pub struct Submesh {
    pub submesh_a: Option<(SubmeshHeader, Vec<Surface>)>,
    pub submesh_b: Option<(SubmeshHeader, Vec<Surface>)>,
    pub gpu_data: Option<MeshData>,
    pub gpu_vbufs: Vec<VertexBuffer>,
    pub cpu_data: Option<MeshData>,
    pub cpu_vbufs: Vec<VertexBuffer>,
    pub cpu_vdata: Vec<u8>,
    pub cpu_idata: Vec<u8>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MeshHeader {
    pub aabb: AABB,
    pub unk_18: i32,
    pub num_submeshes: i16,
    pub unk_1e: i16,
    pub ptr_submesh_a: i32,
    pub ptr_submesh_b: i32,
}

impl MeshHeader {
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
            Vec<(SubmeshHeader, Vec<Surface>)>,
            Vec<(SubmeshHeader, Vec<Surface>)>,
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
                let sm = SubmeshHeader::from_data(&buf[data_offset..])?;
                data_offset += size_of::<SubmeshHeader>();
                headers_a.push(sm);
            }
        }

        if self.ptr_submesh_b != 0 {
            for _ in 0..num_submeshes {
                let sm = SubmeshHeader::from_data(&buf[data_offset..])?;
                data_offset += size_of::<SubmeshHeader>();
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
pub struct SubmeshHeader {
    pub unk_00: i16,
    pub num_surfaces: i16,
    pub unk_04: i32,
    pub unk_08: i32,
    pub unk_0c: i32,
}

impl SubmeshHeader {
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
