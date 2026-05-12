use crate::AABB;
use crate::VolitionError;
use crate::util::*;

pub const MAX_SURFACES: u16 = 100;
pub const MAX_LODS: u32 = 100;
pub const MAX_UVS: u8 = 4;
pub const V_ATTR_FLAG_BONES: u8 = 1;
pub const V_ATTR_FLAG_NORMAL: u8 = 2;
pub const V_ATTR_FLAG_UNK: u8 = 4; // maybe morph

/// Deserialized
#[derive(Debug, Clone)]
pub struct LodMeshData {
    /// Headers for geometry that lives in VRAM
    pub gpu_geometry: Mesh,
    /// Headers for geometry that lives in CPU RAM
    /// Purpose unknown, sometimes not present
    /// Always has exactly one vertex buffer
    /// Never has UV channels. Only possible extra attribute is bones.
    /// If exists, number of surfaces matches gpu data
    pub cpu_geometry: Option<Mesh>,
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
    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
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
            bbox: AABB::from_le_bytes(buf)?,
            flags: read_i32_le(buf, 0x18),
            num_lods,
            ptr_gpu,
            ptr_cpu,
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x00..0x18].copy_from_slice(&self.bbox.to_le_bytes());
        bytes[0x18..0x1c].copy_from_slice(&self.flags.to_le_bytes());
        bytes[0x1c..0x20].copy_from_slice(&self.num_lods.to_le_bytes());
        bytes[0x20..0x24].copy_from_slice(&self.ptr_gpu.to_le_bytes());
        bytes[0x24..0x28].copy_from_slice(&self.ptr_cpu.to_le_bytes());
        bytes
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
            gpu_headers.push(MeshHeader::from_le_bytes(&buf[*data_offset..])?);
            *data_offset += size_of::<MeshHeader>();
        }

        let cpu_headers = if self.has_cpu_geometry() {
            let mut headers = Vec::with_capacity(num_lods);
            for _ in 0..num_lods {
                headers.push(Some(MeshHeader::from_le_bytes(&buf[*data_offset..])?));
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
                let index_header = IndexBuffer::from_le_bytes(&buf[*data_offset..])?;
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
                    vertex_headers.push(VertexBuffer::from_le_bytes(&buf[*data_offset..])?);
                    *data_offset += size_of::<VertexBuffer>();
                }

                Mesh {
                    surface_header,
                    surfaces,
                    index_header,
                    vertex_headers,
                }
            };

            let cpu = if let Some((surface_header, surfaces)) = cdata {
                align(data_offset, 4);
                let index_header = IndexBuffer::from_le_bytes(&buf[*data_offset..])?;

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
                    let vertex_header = VertexBuffer::from_le_bytes(&buf[*data_offset..])?;

                    if vertex_header.has_normals() {
                        return Err(VolitionError::UnexpectedValue {
                            desc: "VertexBufferHeader::attributes shouldn't have normals (cpu)",
                            got: vertex_header.attributes as i32,
                        });
                    }

                    if vertex_header.has_unk_attr() {
                        return Err(VolitionError::UnexpectedValue {
                            desc: "VertexBufferHeader::attributes shouldn't have unk_attr (cpu)",
                            got: vertex_header.attributes as i32,
                        });
                    }

                    if vertex_header.num_uv_channels != 0 {
                        return Err(VolitionError::ExpectedExactValue {
                            field: "VertexBufferHeader::num_uvs (cpu)",
                            expected: 0,
                            got: vertex_header.num_uv_channels as i32,
                        });
                    }

                    vertex_headers.push(vertex_header);
                    *data_offset += size_of::<VertexBuffer>();
                }

                Some(Mesh {
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
pub struct Mesh {
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
    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
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

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x00..0x02].copy_from_slice(&self.unk_00.to_le_bytes());
        bytes[0x02..0x04].copy_from_slice(&self.num_surfaces.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&(-1_i32).to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&(-1_i32).to_le_bytes());
        bytes[0x0c..0x10].copy_from_slice(&0_i32.to_le_bytes());
        bytes
    }

    pub fn read_surfaces(
        &self,
        buf: &[u8],
        data_offset: &mut usize,
    ) -> Result<Vec<Surface>, VolitionError> {
        let num_surfaces = self.num_surfaces as usize;
        let mut surfaces = Vec::with_capacity(num_surfaces);
        for _ in 0..num_surfaces {
            surfaces.push(Surface::from_le_bytes(&buf[*data_offset..])?);
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
    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            vbuf: read_u32_le(buf, 0x0),
            start_index: read_u32_le(buf, 0x4),
            start_vertex: read_u32_le(buf, 0x8),
            num_indices: read_u16_le(buf, 0xc),
            material: read_u16_le(buf, 0xe),
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x00..0x04].copy_from_slice(&self.vbuf.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&self.start_index.to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&self.start_vertex.to_le_bytes());
        bytes[0x0c..0x0e].copy_from_slice(&self.num_indices.to_le_bytes());
        bytes[0x0e..0x10].copy_from_slice(&self.material.to_le_bytes());
        bytes
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
    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
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

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x00..0x02].copy_from_slice(&self.mesh_type.to_le_bytes());
        bytes[0x02..0x04].copy_from_slice(&self.num_vertex_buffers.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&self.num_indices.to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&(-1_i32).to_le_bytes());
        bytes[0x0c..0x10].copy_from_slice(&(-1_i32).to_le_bytes());
        bytes[0x10..0x14].copy_from_slice(&0_i32.to_le_bytes());
        bytes
    }
}

/// 1:1 from disk
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VertexBuffer {
    pub attributes: u8,
    pub num_uv_channels: u8,
    pub stride: u16,
    pub num_vertices: u32,
    /// Always -1
    pub ptr_render_data: i32,
    /// Always 0
    pub unk_0c: i32,
}

impl VertexBuffer {
    pub const fn has_bones(&self) -> bool {
        self.attributes & V_ATTR_FLAG_BONES != 0
    }

    pub const fn has_normals(&self) -> bool {
        self.attributes & V_ATTR_FLAG_NORMAL != 0
    }

    /// maybe morph
    pub const fn has_unk_attr(&self) -> bool {
        self.attributes & V_ATTR_FLAG_UNK != 0
    }

    pub const fn off_normal(&self) -> usize {
        if self.has_bones() { 12 + 8 } else { 12 }
    }

    pub const fn off_uv(&self) -> usize {
        let mut off = 12;
        if self.has_bones() {
            off += 8
        };
        if self.has_normals() {
            off += 4
        };
        if self.has_unk_attr() {
            off += 8
        };
        off
    }

    pub const fn attr_len(&self) -> usize {
        vertex_attr_len(self.attributes)
    }

    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let attributes = buf[0];
        if attributes > 5 {
            return Err(VolitionError::UnexpectedValue {
                desc: "VertexBuffer::attr unknown type",
                got: attributes as i32,
            });
        }

        let num_uv_channels = buf[1];

        if num_uv_channels > MAX_UVS {
            return Err(VolitionError::UnexpectedValue {
                desc: "MeshHeader::num_uvs higher than expected",
                got: num_uv_channels as i32,
            });
        }

        let stride = read_u16_le(buf, 0x2);

        let uv_len = (num_uv_channels as usize) * 4;
        let expected_stride = 12 + vertex_attr_len(attributes) + uv_len;

        if stride as usize != expected_stride {
            return Err(VolitionError::ExpectedExactValue {
                field: "VertexBuffer::stride (calculated from format)",
                expected: expected_stride as i32,
                got: expected_stride as i32 - stride as i32,
            });
        }

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
            attributes,
            num_uv_channels,
            stride,
            num_vertices: read_u32_le(buf, 0x4),
            ptr_render_data,
            unk_0c,
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0] = self.attributes;
        bytes[1] = self.num_uv_channels;
        bytes[0x02..0x04].copy_from_slice(&self.stride.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&self.num_vertices.to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&(-1_i32).to_le_bytes());
        bytes[0x0c..0x10].copy_from_slice(&0_i32.to_le_bytes());
        bytes
    }
}

const fn vertex_attr_len(attr: u8) -> usize {
    let mut attr_len = 0;
    if attr & V_ATTR_FLAG_BONES != 0 {
        attr_len += 8;
    }
    if attr & V_ATTR_FLAG_NORMAL != 0 {
        attr_len += 4;
    }
    if attr & V_ATTR_FLAG_UNK != 0 {
        attr_len += 8;
    }
    attr_len
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_lod_mesh_header_cycle_bytes() {
        let mut buf = vec![];

        buf.extend_from_slice(&3.0_f32.to_le_bytes()); // aabb
        buf.extend_from_slice(&4.0_f32.to_le_bytes());
        buf.extend_from_slice(&5.0_f32.to_le_bytes());
        buf.extend_from_slice(&7.0_f32.to_le_bytes());
        buf.extend_from_slice(&8.0_f32.to_le_bytes());
        buf.extend_from_slice(&9.0_f32.to_le_bytes());

        buf.extend_from_slice(&0_i32.to_le_bytes()); // flags
        buf.extend_from_slice(&4_i32.to_le_bytes()); // num_lods
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // ptr_gpu
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // ptr_cpu

        let hed = LodMeshHeader::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, hed.to_le_bytes());
    }

    #[test]
    fn test_mesh_header_cycle_bytes() {
        let mut buf = vec![];

        buf.extend_from_slice(&9_i16.to_le_bytes()); // unk_00
        buf.extend_from_slice(&9_u16.to_le_bytes()); // num_surfaces
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // ptr_04
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // ptr_08
        buf.extend_from_slice(&0_i32.to_le_bytes()); // unk_0c

        let hed = MeshHeader::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, hed.to_le_bytes());
    }

    #[test]
    fn test_surface_cycle_bytes() {
        let mut buf = vec![];

        buf.extend_from_slice(&3_u32.to_le_bytes()); // vbuf
        buf.extend_from_slice(&4_u32.to_le_bytes()); // start_index
        buf.extend_from_slice(&100_i32.to_le_bytes()); // start_vertex
        buf.extend_from_slice(&52_u16.to_le_bytes()); // num_indices
        buf.extend_from_slice(&7_i16.to_le_bytes()); // material

        let hed = Surface::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, hed.to_le_bytes());
    }

    #[test]
    fn test_index_buffer_cycle_bytes() {
        let mut buf = vec![];

        buf.extend_from_slice(&4_i16.to_le_bytes()); // mesh_type
        buf.extend_from_slice(&100_u16.to_le_bytes()); // num_vertex_buffers
        buf.extend_from_slice(&52_u32.to_le_bytes()); // num_indices
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // runtime_08
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // runtime_0c
        buf.extend_from_slice(&0_i32.to_le_bytes()); // runtime_10

        let hed = IndexBuffer::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, hed.to_le_bytes());
    }

    #[test]
    fn test_vertex_buffer_cycle_bytes() {
        let mut buf = vec![];

        buf.push(0); // attributes
        buf.push(3); // num_uv_channels
        buf.extend_from_slice(&24_u16.to_le_bytes()); // stride
        buf.extend_from_slice(&57_u32.to_le_bytes()); // num_vertices
        buf.extend_from_slice(&(-1_i32).to_le_bytes()); // ptr_render_data
        buf.extend_from_slice(&0_i32.to_le_bytes()); // unk_0c

        let hed = VertexBuffer::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, hed.to_le_bytes());
    }
}
