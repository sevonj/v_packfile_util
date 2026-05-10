use egui::CollapsingHeader;
use egui::UiBuilder;
use egui::Widget;
use egui_extras::Column;
use egui_extras::TableBuilder;
use v_types::Geometry;
use v_types::IndexBuffer;
use v_types::LodMeshData;
use v_types::LodMeshHeader;
use v_types::MeshHeader;
use v_types::StaticMesh;
use v_types::Surface;
use v_types::VertexBuffer;

use crate::app::widgets::VectorDisplay;

const ROW_H: f32 = 18.0;
const SPACE: f32 = 8.0;

pub struct LodMeshInspector<'a> {
    smesh: &'a StaticMesh,
}

impl<'a> LodMeshInspector<'a> {
    pub fn new(smesh: &'a StaticMesh) -> Self {
        Self { smesh }
    }
}

impl Widget for LodMeshInspector<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.scope(|ui| {
            let mesh = self.smesh;

            ui.label("Geometry");

            header_ui(ui, &mesh.mesh_header);

            ui.add_space(SPACE);

            CollapsingHeader::new("Lods")
                .default_open(true)
                .show(ui, |ui| {
                    lods_ui(ui, &mesh.lod_meshes);
                });
        })
        .response
    }
}

fn header_ui(ui: &mut egui::Ui, header: &LodMeshHeader) {
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
                ui.add(VectorDisplay::new(header.bbox.min));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("BBox max:");
            });
            row.col(|ui| {
                ui.add(VectorDisplay::new(header.bbox.max));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("Flags:");
            });
            row.col(|ui| {
                ui.monospace(format!("{:08X?}", header.flags));
            });
        });

        body.row(ROW_H, |mut row| {
            row.col(|ui| {
                ui.label("Lods:");
            });
            row.col(|ui| {
                ui.label(format!("{}", header.num_lods));
            });
        });
    });
}

fn lods_ui(ui: &mut egui::Ui, meshes: &[LodMeshData]) {
    for (i, mesh) in meshes.iter().enumerate() {
        CollapsingHeader::new(i.to_string())
            .show(ui, |ui| {
                ui.scope_builder(UiBuilder::new().id_salt("gpu"), |ui| {
                    ui.weak("Geometry (GPU)");
                    geom_ui(ui, &mesh.gpu_geometry);
                });

                ui.separator();

                ui.scope_builder(UiBuilder::new().id_salt("cpu"), |ui| {
                    ui.weak("Geometry (CPU)");
                    let Some(data) = &mesh.cpu_geometry else {
                        ui.label("Doesn't exist");
                        return;
                    };
                    geom_ui(ui, data);
                });
            })
            .header_response
            .on_disabled_hover_text("Doesn't exist");
    }
}

fn geom_ui(ui: &mut egui::Ui, data: &Geometry) {
    geom_header_ui(ui, &data.surface_header);

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

fn geom_header_ui(ui: &mut egui::Ui, header: &MeshHeader) {
    let table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .column(Column::remainder());

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
    });
}

fn index_header_ui(ui: &mut egui::Ui, header: &IndexBuffer) {
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

fn vbufs_ui(ui: &mut egui::Ui, vbufs: &[VertexBuffer]) {
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
                        ui.label(format!("Vertex Format: {}", vbuf.format));
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
