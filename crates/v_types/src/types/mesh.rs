use crate::AABB;
use crate::VolitionError;
use crate::util::*;

pub const MAX_SURFACES: u16 = 100;
pub const MAX_LODS: u32 = 100;

/// Deserialized
#[derive(Debug, Clone)]
pub struct LodMeshData {
    /// Headers for geometry that lives in VRAM
    pub gpu_geometry: Geometry,
    /// Headers for geometry that lives in CPU RAM
    /// Purpose unknown, sometimes not present
    /// Always has exactly one vertex buffer
    /// If exists, number of surfaces matches gpu data
    pub cpu_geometry: Option<Geometry>,
    pub unk_20b: Option<[u8; 20]>,
    /// CPU vertex buffer in raw bytes. Empty if `cpu` == `None`
    /// Format is always 3xf32 coords only
    pub cpu_vdata: Vec<u8>,
    /// CPU index buffer in raw bytes. Empty if `cpu` == `None`
    /// Format is always u16 tri-strip
    pub cpu_idata: Vec<u8>,
}

/// 1:1 from disk
/// Model with multiple lods
#[derive(Debug, Clone)]
#[repr(C)]
pub struct LodMeshHeader {
    pub bbox: AABB,
    pub flags: i32,
    pub num_lods: u32,
    pub ptr_gpu: i32,
    pub ptr_cpu: i32,
}

impl LodMeshHeader {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        let num_lods = read_u32_le(buf, 0x1c);
        if num_lods > MAX_LODS {
            return Err(VolitionError::ValueTooHigh {
                field: "MeshHeader::num_lods",
                max: MAX_LODS as usize,
                got: num_lods as usize,
            });
        }

        if num_lods == 0 {
            return Err(VolitionError::UnexpectedValue {
                desc: "MeshHeader::num_lods cannot be zero",
                got: num_lods as i32,
            });
        }

        let ptr_gpu = read_i32_le(buf, 0x20);
        if ptr_gpu != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "MeshHeader::ptr_gpu",
                expected: -1,
                got: ptr_gpu,
            });
        }

        let ptr_cpu = read_i32_le(buf, 0x24);
        if ![0, -1].contains(&ptr_cpu) {
            return Err(VolitionError::UnexpectedValue {
                desc: "MeshHeader::ptr_cpu should be either -1 or 0",
                got: ptr_cpu,
            });
        }

        Ok(Self {
            bbox: AABB::from_data(buf)?,
            flags: read_i32_le(buf, 0x18),
            num_lods,
            ptr_gpu,
            ptr_cpu,
        })
    }

    pub const fn has_cpu_geometry(&self) -> bool {
        self.ptr_cpu == -1
    }

    pub(crate) fn read_data(
        &self,
        buf: &[u8],
        data_offset: &mut usize,
        unk_2c: i32,
    ) -> Result<Vec<LodMeshData>, VolitionError> {
        let num_lods = self.num_lods as usize;

        let unk_20b = if unk_2c != 0 {
            let u = read_bytes(buf, *data_offset);
            *data_offset += 20;
            Some(u)
        } else {
            None
        };

        align(data_offset, 16);

        let mut meshes: Vec<LodMeshData> = Vec::with_capacity(num_lods);

        let num_lods = self.num_lods as usize;

        let mut gpu_headers = Vec::with_capacity(num_lods);
        for _ in 0..num_lods {
            gpu_headers.push(MeshHeader::from_data(&buf[*data_offset..])?);
            *data_offset += size_of::<MeshHeader>();
        }

        let cpu_headers = if self.has_cpu_geometry() {
            let mut headers = Vec::with_capacity(num_lods);
            for _ in 0..num_lods {
                headers.push(Some(MeshHeader::from_data(&buf[*data_offset..])?));
                *data_offset += size_of::<MeshHeader>();
            }
            headers
        } else {
            vec![None; num_lods]
        };

        assert_eq!(gpu_headers.len(), num_lods);
        assert_eq!(cpu_headers.len(), num_lods);

        #[allow(clippy::type_complexity)]
        let mut ret: Vec<(
            (MeshHeader, Vec<Surface>),
            Option<(MeshHeader, Vec<Surface>)>,
        )> = Vec::with_capacity(num_lods);

        for (ghead, chead) in gpu_headers.into_iter().zip(cpu_headers) {
            let g_surfs = ghead.read_surfaces(buf, data_offset)?;
            let g = (ghead, g_surfs);

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

        for (gdata, cdata) in ret {
            let gpu = {
                let (surface_header, surfaces) = gdata;

                align(data_offset, 4);
                let index_header = IndexBuffer::from_data(&buf[*data_offset..])?;
                let num_vertex_buffers = index_header.num_vertex_buffers as usize;
                *data_offset += size_of::<IndexBuffer>();

                if index_header.mesh_type != 0 {
                    return Err(VolitionError::ExpectedExactValue {
                        field: "MeshData::mesh_type (gpu)",
                        expected: 0,
                        got: index_header.mesh_type as i32,
                    });
                }

                let mut vertex_headers = Vec::with_capacity(num_vertex_buffers);
                for _ in 0..index_header.num_vertex_buffers {
                    vertex_headers.push(VertexBuffer::from_data(&buf[*data_offset..])?);
                    *data_offset += size_of::<VertexBuffer>();
                }

                Geometry {
                    surface_header,
                    surfaces,
                    index_header,
                    vertex_headers,
                }
            };

            let cpu = if let Some((surface_header, surfaces)) = cdata {
                align(data_offset, 4);
                let index_header = IndexBuffer::from_data(&buf[*data_offset..])?;

                let num_vertex_buffers = index_header.num_vertex_buffers as usize;
                if num_vertex_buffers != 1 {
                    return Err(VolitionError::ExpectedExactValue {
                        field: "IndexBufferHeader::num_vertex_buffers (cpu)",
                        expected: 1,
                        got: num_vertex_buffers as i32,
                    });
                }

                *data_offset += size_of::<IndexBuffer>();

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
                    let vertex_header = VertexBuffer::from_data(&buf[*data_offset..])?;
                    vertex_headers.push(vertex_header);
                    *data_offset += size_of::<VertexBuffer>();
                }

                Some(Geometry {
                    surface_header,
                    surfaces,
                    index_header,
                    vertex_headers,
                })
            } else {
                None
            };

            let (cpu_vdata, cpu_idata) = if let Some(geom) = &cpu {
                let num_indices = geom.index_header.num_indices as usize;

                align(data_offset, 16);
                let mut len_cpu_vdata = 0;
                for vhead in &geom.vertex_headers {
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

            meshes.push(LodMeshData {
                gpu_geometry: gpu,
                cpu_geometry: cpu,
                unk_20b,
                cpu_vdata,
                cpu_idata,
            });
        }
        Ok(meshes)
    }
}

/// Deserialized
#[derive(Debug, Clone)]
pub struct Geometry {
    pub surface_header: MeshHeader,
    pub surfaces: Vec<Surface>,
    pub index_header: IndexBuffer,
    pub vertex_headers: Vec<VertexBuffer>,
}

/// 1:1 from disk
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MeshHeader {
    pub unk_00: i16,
    pub num_surfaces: u16,
    /// Always -1
    pub ptr_04: i32,
    /// Always -1
    pub ptr_08: i32,
    /// Always 0
    pub unk_0c: i32,
}

impl MeshHeader {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let num_surfaces = read_u16_le(buf, 0x2);
        if num_surfaces > MAX_SURFACES {
            return Err(VolitionError::ValueTooHigh {
                field: "GeometryHeader::num_surfaces",
                max: MAX_SURFACES as usize,
                got: num_surfaces as usize,
            });
        }

        let ptr_04 = read_i32_le(buf, 0x4);
        if ptr_04 != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "GeometryHeader::unk_04",
                expected: -1,
                got: ptr_04,
            });
        }

        let ptr_08 = read_i32_le(buf, 0x8);
        if ptr_08 != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "GeometryHeader::unk_08",
                expected: -1,
                got: ptr_08,
            });
        }

        let unk_0c = read_i32_le(buf, 0xc);
        if unk_0c != 0 {
            return Err(VolitionError::ExpectedExactValue {
                field: "GeometryHeader::unk_0c",
                expected: 0,
                got: unk_0c,
            });
        }

        Ok(Self {
            unk_00: read_i16_le(buf, 0x0),
            num_surfaces,
            ptr_04,
            ptr_08,
            unk_0c,
        })
    }

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

/// 1:1 from disk
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
    pub material: u16,
}

impl Surface {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            vbuf: read_u32_le(buf, 0x0),
            start_index: read_u32_le(buf, 0x4),
            start_vertex: read_u32_le(buf, 0x8),
            num_indices: read_u16_le(buf, 0xc),
            material: read_u16_le(buf, 0xe),
        })
    }
}

/// 1:1 from disk
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IndexBuffer {
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

impl IndexBuffer {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let runtime_08 = read_i32_le(buf, 0x8);
        if runtime_08 != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "IndexBuffer::runtime_08",
                expected: -1,
                got: runtime_08,
            });
        }

        let runtime_0c = read_i32_le(buf, 0xc);
        if runtime_0c != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "IndexBuffer::runtime_0c",
                expected: -1,
                got: runtime_0c,
            });
        }

        let runtime_10 = read_u32_le(buf, 0x10);
        if runtime_10 != 0 {
            return Err(VolitionError::ExpectedExactValue {
                field: "IndexBuffer::runtime_10",
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

/// 1:1 from disk
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VertexBuffer {
    /// Probably
    pub format: i16,
    pub stride: u16,
    pub num_vertices: u32,
    /// Always -1
    pub ptr_render_data: i32,
    /// Always 0
    pub unk_0c: i32,
}

impl VertexBuffer {
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
            format: read_i16_le(buf, 0x0),
            stride: read_u16_le(buf, 0x2),
            num_vertices: read_u32_le(buf, 0x4),
            ptr_render_data,
            unk_0c,
        })
    }
}
