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

    Num,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub header: MeshHeader,
    pub submeshes: Vec<Submesh>,
}

impl Mesh {
    pub fn from_data(
        buf: &[u8],
        data_offset: &mut usize,
        unk_2c: i32,
    ) -> Result<Self, VolitionError> {
        let header = MeshHeader::from_data(&buf[*data_offset..])?;
        *data_offset += size_of::<MeshHeader>();

        let num_submeshes = header.num_submeshes as usize;

        if unk_2c != 0 {
            *data_offset += 20;
        }

        align(data_offset, 16);

        let mut submeshes = Vec::with_capacity(num_submeshes);
        for (gdata, cdata) in header.read_submeshes(buf, data_offset)? {
            let gpu = if let Some((surface_header, surfaces)) = gdata {
                align(data_offset, 4);
                let index_header = IndexBufferHeader::from_data(&buf[*data_offset..])?;
                let num_vertex_buffers = index_header.num_vertex_buffers as usize;
                *data_offset += size_of::<IndexBufferHeader>();

                if index_header.mesh_type != 0 {
                    return Err(VolitionError::ExpectedExactValue {
                        field: "MeshData::mesh_type (gpu)",
                        expected: 0,
                        got: index_header.mesh_type as i32,
                    });
                }

                let mut vertex_headers = Vec::with_capacity(num_vertex_buffers);
                for _ in 0..index_header.num_vertex_buffers {
                    vertex_headers.push(VertexBufferHeader::from_data(&buf[*data_offset..])?);
                    *data_offset += size_of::<VertexBufferHeader>();
                }

                Some(SubmeshData {
                    surface_header,
                    surfaces,
                    index_header,
                    vertex_headers,
                })
            } else {
                None
            };

            let cpu = if let Some((surface_header, surfaces)) = cdata {
                align(data_offset, 4);
                let index_header = IndexBufferHeader::from_data(&buf[*data_offset..])?;

                let num_vertex_buffers = index_header.num_vertex_buffers as usize;
                if num_vertex_buffers != 1 {
                    return Err(VolitionError::ExpectedExactValue {
                        field: "IndexBufferHeader::num_vertex_buffers (cpu)",
                        expected: 1,
                        got: num_vertex_buffers as i32,
                    });
                }

                *data_offset += size_of::<IndexBufferHeader>();

                if index_header.mesh_type != 7 {
                    return Err(VolitionError::ExpectedExactValue {
                        field: "MeshData::mesh_type (cpu)",
                        expected: 7,
                        got: index_header.mesh_type as i32,
                    });
                }

                if index_header.num_vertex_buffers != 1 {
                    return Err(VolitionError::ExpectedExactValue {
                        field: "IndexBufferHeader::num_vertex_buffers (cpu)",
                        expected: 1,
                        got: index_header.num_vertex_buffers as i32,
                    });
                }

                let mut vertex_headers = Vec::with_capacity(num_vertex_buffers);
                for _ in 0..index_header.num_vertex_buffers {
                    let vertex_header = VertexBufferHeader::from_data(&buf[*data_offset..])?;

                    if vertex_header.num_uvs != 0 {
                        return Err(VolitionError::ExpectedExactValue {
                            field: "VertexBufferHeader::num_uvs (cpu)",
                            expected: 0,
                            got: vertex_header.num_uvs as i32,
                        });
                    }

                    vertex_headers.push(vertex_header);
                    *data_offset += size_of::<VertexBufferHeader>();
                }

                Some(SubmeshData {
                    surface_header,
                    surfaces,
                    index_header,
                    vertex_headers,
                })
            } else {
                None
            };

            let (cpu_vdata, cpu_idata) = if let Some(submesh) = &cpu {
                let num_indices = submesh.index_header.num_indices as usize;

                align(data_offset, 16);
                let mut len_cpu_vdata = 0;
                for vhead in &submesh.vertex_headers {
                    len_cpu_vdata += vhead.num_vertices as usize * vhead.stride as usize;
                    align(&mut len_cpu_vdata, 16);
                }
                let cpu_vdata = buf[*data_offset..(*data_offset + len_cpu_vdata)].to_vec();
                *data_offset += len_cpu_vdata;

                let len_cpu_idata = num_indices * 2;
                let cpu_idata = buf[*data_offset..(*data_offset + len_cpu_idata)].to_vec();
                *data_offset += len_cpu_idata;

                (cpu_vdata, cpu_idata)
            } else {
                (vec![], vec![])
            };

            submeshes.push(Submesh {
                gpu,
                cpu,
                cpu_vdata,
                cpu_idata,
            });
        }

        Ok(Self { header, submeshes })
    }

    pub const fn has_cpu_geometry(&self) -> bool {
        self.header.has_cpu_submeshes()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MeshHeader {
    pub aabb: AABB,
    pub unk_18: i32,
    pub num_submeshes: u16,
    pub unk_1e: i16,
    pub ptr_gpu: i32,
    pub ptr_cpu: i32,
}

impl MeshHeader {
    pub const MAX_SUBMESHES: u16 = 100;

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        let num_submeshes = read_u16_le(buf, 0x1c);
        if num_submeshes > Self::MAX_SUBMESHES {
            return Err(VolitionError::ValueTooHigh {
                field: "MeshHeader::num_submeshes",
                max: Self::MAX_SUBMESHES as usize,
                got: num_submeshes as usize,
            });
        }

        let ptr_gpu = read_i32_le(buf, 0x20);
        if ![0, -1].contains(&ptr_gpu) {
            return Err(VolitionError::UnexpectedValue {
                desc: "MeshData::ptr_gpu_submesh should be either -1 or 0",
                got: ptr_gpu,
            });
        }

        let ptr_cpu = read_i32_le(buf, 0x24);
        if ![0, -1].contains(&ptr_cpu) {
            return Err(VolitionError::UnexpectedValue {
                desc: "MeshData::ptr_cpu_submesh should be either -1 or 0",
                got: ptr_cpu,
            });
        }

        Ok(Self {
            aabb: AABB::from_data(buf)?,
            unk_18: read_i32_le(buf, 0x18),
            num_submeshes,
            unk_1e: read_i16_le(buf, 0x1e),
            ptr_gpu,
            ptr_cpu,
        })
    }

    pub const fn has_gpu_submeshes(&self) -> bool {
        self.ptr_gpu == -1
    }

    pub const fn has_cpu_submeshes(&self) -> bool {
        self.ptr_cpu == -1
    }

    #[allow(clippy::type_complexity)]
    pub fn read_submeshes(
        &self,
        buf: &[u8],
        data_offset: &mut usize,
    ) -> Result<
        Vec<(
            Option<(SurfaceHeader, Vec<Surface>)>,
            Option<(SurfaceHeader, Vec<Surface>)>,
        )>,
        VolitionError,
    > {
        let num_submeshes = self.num_submeshes as usize;

        let gpu_headers = if self.has_gpu_submeshes() {
            let mut headers = Vec::with_capacity(num_submeshes);
            for _ in 0..num_submeshes {
                headers.push(Some(SurfaceHeader::from_data(&buf[*data_offset..])?));
                *data_offset += size_of::<SurfaceHeader>();
            }
            headers
        } else {
            vec![None; num_submeshes]
        };

        let cpu_headers = if self.has_cpu_submeshes() {
            let mut headers = Vec::with_capacity(num_submeshes);
            for _ in 0..num_submeshes {
                headers.push(Some(SurfaceHeader::from_data(&buf[*data_offset..])?));
                *data_offset += size_of::<SurfaceHeader>();
            }
            headers
        } else {
            vec![None; num_submeshes]
        };

        assert_eq!(gpu_headers.len(), num_submeshes);
        assert_eq!(cpu_headers.len(), num_submeshes);

        let mut ret: Vec<(
            Option<(SurfaceHeader, Vec<Surface>)>,
            Option<(SurfaceHeader, Vec<Surface>)>,
        )> = Vec::with_capacity(num_submeshes);
        for (ghead, chead) in gpu_headers.into_iter().zip(cpu_headers) {
            let g = if let Some(header) = ghead {
                let surfaces = header.read_surfaces(buf, data_offset)?;
                Some((header, surfaces))
            } else {
                None
            };
            let c = if let Some(header) = chead {
                let surfaces = header.read_surfaces(buf, data_offset)?;
                for surf in &surfaces {
                    if surf.vbuf != 0 {
                        return Err(VolitionError::ExpectedExactValue {
                            field: "Surface::vbuf (cpu)",
                            expected: 0,
                            got: surf.vbuf as i32,
                        });
                    }
                }
                Some((header, surfaces))
            } else {
                None
            };

            ret.push((g, c));
        }

        Ok(ret)
    }
}

#[derive(Debug, Clone)]
pub struct Submesh {
    /// Headers for geometry that lives in VRAM
    /// Not tested, but probably always present
    pub gpu: Option<SubmeshData>,
    /// Headers for geometry that lives in CPU RAM
    /// Purpose unknown, sometimes not present
    /// No materials or attributes;
    /// Always has exactly one vertex buffer?
    /// If exists, number of surfaces matches gpu data
    pub cpu: Option<SubmeshData>,
    /// CPU vertex buffer in raw bytes. Empty if `cpu` == `None`
    /// Format is always 3xf32 coords only
    pub cpu_vdata: Vec<u8>,
    /// CPU index buffer in raw bytes. Empty if `cpu` == `None`
    /// Format is always u16 tri-strip
    pub cpu_idata: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SubmeshData {
    pub surface_header: SurfaceHeader,
    pub surfaces: Vec<Surface>,
    pub index_header: IndexBufferHeader,
    pub vertex_headers: Vec<VertexBufferHeader>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SurfaceHeader {
    pub unk_00: i16,
    pub num_surfaces: u16,
    pub unk_04: i32,
    pub unk_08: i32,
    pub unk_0c: i32,
}

impl SurfaceHeader {
    pub const MAX_SURFACES: u16 = 100;

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let num_surfaces = read_u16_le(buf, 0x2);
        if num_surfaces > Self::MAX_SURFACES {
            return Err(VolitionError::ValueTooHigh {
                field: "SubmeshHeader::num_surfaces",
                max: Self::MAX_SURFACES as usize,
                got: num_surfaces as usize,
            });
        }

        Ok(Self {
            unk_00: read_i16_le(buf, 0x0),
            num_surfaces,
            unk_04: read_i32_le(buf, 0x4),
            unk_08: read_i32_le(buf, 0x8),
            unk_0c: read_i32_le(buf, 0xc),
        })
    }

    /// # Arguments: buf must be sliced to start immediately after Submeshes
    pub fn read_surfaces(
        &self,
        buf: &[u8],
        data_offset: &mut usize,
    ) -> Result<Vec<Surface>, VolitionError> {
        let num_surfaces = self.num_surfaces as usize;
        let mut surfaces = Vec::with_capacity(num_surfaces);
        for _ in 0..num_surfaces {
            surfaces.push(Surface::from_data(&buf[*data_offset..])?);
            *data_offset += size_of::<Surface>();
        }
        Ok(surfaces)
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Surface {
    /// Which vertex buffer to use
    pub vbuf: u32,
    /// First index
    pub start_index: u32,
    /// First vertex
    pub start_vertex: u32,
    pub num_indices: u16,
    pub material: i16,
}

impl Surface {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            vbuf: read_u32_le(buf, 0x0),
            start_index: read_u32_le(buf, 0x4),
            start_vertex: read_u32_le(buf, 0x8),
            num_indices: read_u16_le(buf, 0xc),
            material: read_i16_le(buf, 0xe),
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct IndexBufferHeader {
    pub mesh_type: i16,
    pub num_vertex_buffers: u16,
    pub num_indices: u32,
    /// Always -1
    pub runtime_08: i32,
    /// Always -1
    pub runtime_0c: i32,
    /// Always 0
    pub runtime_10: u32,
}

impl IndexBufferHeader {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let runtime_08 = read_i32_le(buf, 0x8);
        if runtime_08 != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "MeshData::runtime_08",
                expected: -1,
                got: runtime_08,
            });
        }

        let runtime_0c = read_i32_le(buf, 0xc);
        if runtime_0c != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "MeshData::runtime_0c",
                expected: -1,
                got: runtime_0c,
            });
        }

        let runtime_10 = read_u32_le(buf, 0x10);
        if runtime_10 != 0 {
            return Err(VolitionError::ExpectedExactValue {
                field: "MeshData::runtime_10",
                expected: 0,
                got: runtime_10 as i32,
            });
        }

        Ok(Self {
            mesh_type: read_i16_le(buf, 0x0),
            num_vertex_buffers: read_u16_le(buf, 0x2),
            num_indices: read_u32_le(buf, 0x4),
            runtime_08,
            runtime_0c,
            runtime_10,
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct VertexBufferHeader {
    /// Probably
    pub format: u8,
    pub num_uvs: u8,
    pub stride: u16,
    pub num_vertices: u32,
    /// Always -1
    pub ptr_render_data: i32,
    /// Always 0
    pub unk_0c: i32,
}

impl VertexBufferHeader {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let ptr_render_data = read_i32_le(buf, 0x8);
        if ptr_render_data != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "VertexBuffer::ptr_render_data",
                expected: -1,
                got: ptr_render_data,
            });
        }

        let unk_0c = read_i32_le(buf, 0xc);
        if unk_0c != 0 {
            return Err(VolitionError::ExpectedExactValue {
                field: "VertexBuffer::unk_0c",
                expected: 0,
                got: unk_0c,
            });
        }

        Ok(Self {
            format: buf[0],
            num_uvs: buf[1],
            stride: read_u16_le(buf, 0x2),
            num_vertices: read_u32_le(buf, 0x4),
            ptr_render_data,
            unk_0c,
        })
    }
}
