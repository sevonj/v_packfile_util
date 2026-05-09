use egui::CollapsingHeader;
use egui::UiBuilder;
use egui::Widget;
use egui_extras::Column;
use egui_extras::TableBuilder;
use v_types::IndexBufferHeader;
use v_types::Mesh;
use v_types::MeshHeader;
use v_types::Submesh;
use v_types::SubmeshData;
use v_types::Surface;
use v_types::SurfaceHeader;
use v_types::VertexBufferHeader;

use crate::app::widgets::VectorDisplay;

const ROW_H: f32 = 18.0;
const SPACE: f32 = 8.0;

pub struct MeshInspector<'a> {
    mesh: &'a Mesh,
}

impl<'a> MeshInspector<'a> {
    pub fn new(mesh: &'a Mesh) -> Self {
        Self { mesh }
    }
}

impl Widget for MeshInspector<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.scope(|ui| {
            let mesh = self.mesh;

            CollapsingHeader::new("Header").show(ui, |ui| {
                header_ui(ui, &mesh.header);
            });

            ui.add_space(SPACE);

            CollapsingHeader::new("Submeshes")
                .default_open(true)
                .show(ui, |ui| {
                    submeshes_ui(ui, &mesh.submeshes);
                });
        })
        .response
    }
}

fn header_ui(ui: &mut egui::Ui, header: &MeshHeader) {
    let table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_H, |mut row| {
            row.col(|ui| {
                ui.weak("Field");
            });
            row.col(|ui| {
                ui.weak("Value");
            });
        });

    table_builder.body(|mut body| {
        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("BBox min:");
            });
            row.col(|ui| {
                ui.add(VectorDisplay::new(header.aabb.min));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("BBox max:");
            });
            row.col(|ui| {
                ui.add(VectorDisplay::new(header.aabb.max));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("unk_18:");
            });
            row.col(|ui| {
                ui.monospace(format!("{:08X?}", header.unk_18));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("Submeshes:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.num_submeshes));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("unk_1e:");
            });
            row.col(|ui| {
                ui.monospace(format!("{:04X?}", header.unk_1e));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("ptr_gpu:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.ptr_gpu));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("ptr_cpu:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.ptr_cpu));
            });
        });
    });
}

fn submeshes_ui(ui: &mut egui::Ui, submeshes: &[Submesh]) {
    for (i, submesh) in submeshes.iter().enumerate() {
        CollapsingHeader::new(i.to_string())
            .enabled(submesh.gpu.is_some())
            .show(ui, |ui| {
                ui.scope_builder(UiBuilder::new().id_salt("gpu"), |ui| {
                    ui.weak("GPU Geometry");
                    let Some(data) = &submesh.gpu else {
                        ui.label("Doesn't exist");
                        return;
                    };
                    submesh_data_ui(ui, data);
                });

                ui.separator();

                ui.scope_builder(UiBuilder::new().id_salt("cpu"), |ui| {
                    ui.weak("CPU Geometry");
                    let Some(data) = &submesh.cpu else {
                        ui.label("Doesn't exist");
                        return;
                    };
                    submesh_data_ui(ui, data);
                });
            })
            .header_response
            .on_disabled_hover_text("Doesn't exist");
    }
}

fn submesh_data_ui(ui: &mut egui::Ui, data: &SubmeshData) {
    surf_header_ui(ui, &data.surface_header);

    CollapsingHeader::new("Surfaces")
        .default_open(true)
        .id_salt("surfaces")
        .show(ui, |ui| {
            surfaces_ui(ui, &data.surfaces);
        })
        .header_response
        .on_disabled_hover_text("Doesn't exist");

    index_header_ui(ui, &data.index_header);

    CollapsingHeader::new("Vertex Buffers")
        .default_open(true)
        .id_salt("vbuf_headers")
        .show(ui, |ui| {
            vbufs_ui(ui, &data.vertex_headers);
        })
        .header_response
        .on_disabled_hover_text("Doesn't exist");
}

fn surf_header_ui(ui: &mut egui::Ui, header: &SurfaceHeader) {
    let table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .column(Column::remainder())
        .header(ROW_H, |mut row| {
            row.col(|ui| {
                ui.weak("Surface Header");
            });
        });

    table_builder.body(|mut body| {
        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label("unk_00:");
                    ui.monospace(format!("{:04X?}", header.unk_00));
                });
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Surfaces:");
                    ui.label(format!("{}", header.num_surfaces));
                });
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label("unk_04:");
                    ui.monospace(format!("{:08X?}", header.unk_04));
                });
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label("unk_08:");
                    ui.monospace(format!("{:08X?}", header.unk_08));
                });
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label("unk_0c:");
                    ui.monospace(format!("{:08X?}", header.unk_0c));
                });
            });
        });
    });
}

fn index_header_ui(ui: &mut egui::Ui, header: &IndexBufferHeader) {
    let table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_H, |mut row| {
            row.col(|ui| {
                ui.weak("Index Buffer");
            });
        });

    table_builder.body(|mut body| {
        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("Type:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.mesh_type));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("Vertex Buffers:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.num_vertex_buffers));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("Indices:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.num_indices));
            });
        });
    });
}

fn surfaces_ui(ui: &mut egui::Ui, surfaces: &[Surface]) {
    let num_surfaces = surfaces.len();

    let table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .column(Column::remainder());

    table_builder.body(|body| {
        body.rows(ROW_H * 5.0, num_surfaces, |mut row| {
            let surf = &surfaces[row.index()];

            row.col(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Vertex buffer:");
                        ui.label(format!("{}", surf.vbuf));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Start index:");
                        ui.label(format!("{}", surf.start_index));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Start vertex:");
                        ui.label(format!("{}", surf.start_vertex));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Indices:");
                        ui.label(format!("{}", surf.num_indices));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Material Index:");
                        ui.label(format!("{}", surf.material));
                    });
                });
            });
        });
    });
}

fn vbufs_ui(ui: &mut egui::Ui, vbufs: &[VertexBufferHeader]) {
    let num_vbufs = vbufs.len();

    let table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .column(Column::remainder());

    table_builder.body(|body| {
        body.rows(ROW_H * 4.0, num_vbufs, |mut row| {
            let vbuf = &vbufs[row.index()];

            row.col(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Vertex Format: {}", vbuf.vertex_format));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("UV Channels: {}", vbuf.num_uvs));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Stride: {}", vbuf.stride));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Vertices: {}", vbuf.num_vertices));
                    });
                });
            });
        });
    });
}
