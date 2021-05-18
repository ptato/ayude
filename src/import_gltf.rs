use std::borrow::Cow;

use image::{DynamicImage, EncodableLayout, ImageError, ImageFormat};

use crate::{Catalog, Entity, catalog::Id, graphics::{Material, Mesh, Primitive, Texture, texture::{MagFilter, MinFilter, TextureFormat, TextureWrap}}};

pub fn import(
    file_name: &str,
    entities: &mut Catalog<Entity>,
    meshes: &mut Catalog<Mesh>,
    materials: &mut Catalog<Material>,
    textures: &mut Catalog<Texture>,
) -> Result<(), ImportGltfError> {
    let gltf = gltf::Gltf::open(file_name).unwrap();
    let base_path = file_name[0..file_name.rfind("/").unwrap()].to_string();
    let mut importer = Importer {
        entities_catalog: entities,
        meshes_catalog: meshes,
        materials_catalog: materials,
        textures_catalog: textures,
        blob: gltf.blob,
        buffers: vec![],
        images: vec![],
        textures: vec![],
        materials: vec![],
        meshes: vec![],
        base_path,
    };

    importer.import(gltf.document)
}
struct Importer<'catalogs> {
    blob: Option<Vec<u8>>,
    base_path: String,

    entities_catalog: &'catalogs mut Catalog<Entity>,
    meshes_catalog: &'catalogs mut Catalog<Mesh>,
    materials_catalog: &'catalogs mut Catalog<Material>,
    textures_catalog: &'catalogs mut Catalog<Texture>,

    // relate gltf indices to data or catalog ids
    buffers: Vec<Vec<u8>>,
    images: Vec<(Vec<u8>, u32, u32, TextureFormat)>,
    textures: Vec<Id<Texture>>,
    materials: Vec<Id<Material>>,
    meshes: Vec<Id<Mesh>>,
}

impl<'catalogs> Importer<'catalogs> {
    fn import(&mut self, document: gltf::Document) -> Result<(), ImportGltfError> {
        for buffer in document.buffers() {
            let b = self.import_gltf_buffer(buffer)?;
            self.buffers.push(b);
        }

        for image in document.images() {
            self.images.push(self.import_gltf_image(image)?);
        }

        for texture in document.textures() {
            let t = self.import_gltf_texture(texture)?;
            self.textures.push(t);
        }

        for material in document.materials() {
            let m = self.import_gltf_material(material)?;
            self.materials.push(m);
        }

        for mesh in document.meshes() {
            let m = self.import_gltf_mesh(mesh)?;
            self.meshes.push(m);
        }

        let scene = document.default_scene().unwrap();
        for node in scene.nodes() {
            self.import_gltf_node(node, None)?;
        }

        Ok(())
    }

    fn import_gltf_buffer(&mut self, buffer: gltf::Buffer) -> Result<Vec<u8>, ImportGltfError> {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                self.blob.take().ok_or(ImportGltfError::BinSectionNotFound)
            }
            gltf::buffer::Source::Uri(uri) => {
                if uri.starts_with("data:") {
                    Ok(data_uri_to_bytes_and_type(uri)?.0)
                } else {
                    Ok(std::fs::read(format!("{}/{}", self.base_path, uri))?)
                }
            }
        }
    }

    // result is (rgba bytes, width, height, format)
    fn import_gltf_image(
        &self,
        image: gltf::Image,
    ) -> Result<(Vec<u8>, u32, u32, TextureFormat), ImportGltfError> {
        let (data, mime_type) = match image.source() {
            gltf::image::Source::Uri { uri, mime_type } => {
                let (data, parsed_mt) = if uri.starts_with("data:") {
                    data_uri_to_bytes_and_type(uri)?
                } else {
                    let bytes = std::fs::read(&format!("{}/{}", self.base_path, uri))?;
                    let format = if uri.ends_with(".png") {
                        "image/png"
                    } else if uri.ends_with(".jpg") || uri.ends_with(".jpeg") {
                        "image/jpeg"
                    } else {
                        "application/octet-stream"
                    };
                    (bytes, format)
                };

                let mime_type = match mime_type {
                    Some(mt) => mt,
                    None => parsed_mt,
                };

                (Cow::from(data), mime_type)
            }
            gltf::image::Source::View { view, mime_type } => {
                let buffer_index = view.buffer().index();
                let buffer = &self
                    .buffers
                    .get(buffer_index)
                    .ok_or(ImportGltfError::UnknownBufferIndex(buffer_index))?;
                let from = view.offset();
                let to = view.offset() + view.length();
                let data = buffer
                    .get(from..to)
                    .ok_or(ImportGltfError::BufferRangeOutOfBounds(
                        buffer_index,
                        from,
                        to,
                    ))?;
                (Cow::from(data), mime_type)
            }
        };

        let format = match mime_type {
            "image/jpeg" => Ok(ImageFormat::Jpeg),
            "image/png" => Ok(ImageFormat::Png),
            fmt => Err(ImportGltfError::UnknownImageFormat(
                fmt.to_string(),
                image.index(),
            )),
        }?;

        let image = image::load_from_memory_with_format(&data, format)
            .map_err(|e| ImportGltfError::ImageLoadingFailed(image.index().to_string(), e))?;
        match image {
            DynamicImage::ImageRgb8(rgb) => Ok((
                rgb.as_bytes().to_owned(),
                rgb.width(),
                rgb.height(),
                TextureFormat::RGB,
            )),
            DynamicImage::ImageRgba8(rgba) => Ok((
                rgba.as_bytes().to_owned(),
                rgba.width(),
                rgba.height(),
                TextureFormat::RGBA,
            )),
            _ => {
                let rgba = image.into_rgba();
                Ok((
                    rgba.as_bytes().to_owned(),
                    rgba.width(),
                    rgba.height(),
                    TextureFormat::RGBA,
                ))
            }
        }
    }

    fn import_gltf_texture(
        &mut self,
        texture: gltf::Texture,
    ) -> Result<Id<Texture>, ImportGltfError> {
        let image_index = texture.source().index();
        let (data, width, height, format) = &self
            .images
            .get(image_index)
            .ok_or(ImportGltfError::UnknownImageIndex(image_index))?;

        let sampler = texture.sampler();

        let mut builder = Texture::builder(&data, *width as u16, *height as u16, *format)
            .wrap_s(match sampler.wrap_s() {
                gltf::texture::WrappingMode::ClampToEdge => TextureWrap::ClampToEdge,
                gltf::texture::WrappingMode::MirroredRepeat => TextureWrap::MirroredRepeat,
                gltf::texture::WrappingMode::Repeat => TextureWrap::Repeat,
            })
            .wrap_t(match sampler.wrap_t() {
                gltf::texture::WrappingMode::ClampToEdge => TextureWrap::ClampToEdge,
                gltf::texture::WrappingMode::MirroredRepeat => TextureWrap::MirroredRepeat,
                gltf::texture::WrappingMode::Repeat => TextureWrap::Repeat,
            });
        
        if let Some(min_filter) = sampler.min_filter() {
            builder = builder.min_filter(match min_filter {
                gltf::texture::MinFilter::Nearest => MinFilter::Nearest,
                gltf::texture::MinFilter::Linear => MinFilter::Linear,
                gltf::texture::MinFilter::NearestMipmapNearest => MinFilter::NearestMipmapNearest,
                gltf::texture::MinFilter::LinearMipmapNearest => MinFilter::LinearMipmapNearest,
                gltf::texture::MinFilter::NearestMipmapLinear => MinFilter::NearestMipmapLinear,
                gltf::texture::MinFilter::LinearMipmapLinear => MinFilter::LinearMipmapNearest,
            });
        }

        if let Some(mag_filter) = sampler.mag_filter() {
            builder = builder.mag_filter(match mag_filter {
                gltf::texture::MagFilter::Nearest => MagFilter::Nearest,
                gltf::texture::MagFilter::Linear => MagFilter::Linear,
            });
        }
            
        let texture = builder.build();

        Ok(self.textures_catalog.add(texture))
    }

    fn import_gltf_material(
        &mut self,
        material: gltf::Material,
    ) -> Result<Id<Material>, ImportGltfError> {
        let normal = match material.normal_texture().as_ref() {
            Some(info) => self.textures.get(info.texture().index()).copied(),
            None => None,
        };
        let diffuse = match material
            .pbr_metallic_roughness()
            .base_color_texture()
            .as_ref()
        {
            Some(info) => self.textures.get(info.texture().index()).copied(),
            None => None,
        };
        let base_diffuse_color = material.pbr_metallic_roughness().base_color_factor();
        Ok(self.materials_catalog.add(Material {
            normal,
            diffuse,
            base_diffuse_color,
        }))
    }

    fn import_gltf_mesh(&mut self, mesh: gltf::Mesh) -> Result<Id<Mesh>, ImportGltfError> {
        let mut primitives = vec![];
        for primitive in mesh.primitives() {
            let reader =
                primitive.reader(|buffer| self.buffers.get(buffer.index()).map(Vec::as_slice));

            let positions = reader
                .read_positions()
                .ok_or(ImportGltfError::RequiredMeshPropertyMissing(
                    "positions",
                    mesh.index(),
                    primitive.index(),
                ))?
                .collect::<Vec<_>>();

            let normals = reader
                .read_normals()
                .ok_or(ImportGltfError::RequiredMeshPropertyMissing(
                    "normals",
                    mesh.index(),
                    primitive.index(),
                ))?
                .collect::<Vec<_>>();

            let uvs = reader
                .read_tex_coords(0)
                .ok_or(ImportGltfError::RequiredMeshPropertyMissing(
                    "uvs",
                    mesh.index(),
                    primitive.index(),
                ))?
                .into_f32()
                .collect::<Vec<_>>();

            let indices = reader
                .read_indices()
                .ok_or(ImportGltfError::RequiredMeshPropertyMissing(
                    "indices",
                    mesh.index(),
                    primitive.index(),
                ))?
                .into_u32()
                .map(|it| it as u16) // TODO! this sucks
                .collect::<Vec<_>>();

            let material = match primitive.material().index() {
                Some(i) => self
                    .materials
                    .get(i)
                    .copied()
                    .ok_or(ImportGltfError::UnknownMaterialIndex(i))?,
                None => self.import_gltf_material(primitive.material())?, // i'm importing the default material, which doesn't make much sense
            };

            primitives.push(Primitive::new(
                &positions, &normals, &uvs, &indices, material,
            ));
        }
        Ok(self.meshes_catalog.add(Mesh { primitives }))
    }

    fn import_gltf_node(
        &mut self,
        node: gltf::Node,
        parent: Option<Id<Entity>>,
    ) -> Result<Id<Entity>, ImportGltfError> {
        let mesh = match node.mesh() {
            Some(mesh) => self.meshes.get(mesh.index()).copied(),
            None => None,
        };
        let entity = Entity {
            children: vec![],
            parent,
            mesh,
            transform: node.transform().matrix(),
        };
        let id = self.entities_catalog.add(entity);

        let mut children = vec![];
        for child in node.children() {
            let child_id = self.import_gltf_node(child, Some(id))?;
            children.push(child_id);
        }
        self.entities_catalog
            .get_mut(id)
            .ok_or(ImportGltfError::Unreachable)?
            .children = children;

        Ok(id)
    }
}

fn data_uri_to_bytes_and_type(uri: &str) -> Result<(Vec<u8>, &str), base64::DecodeError> {
    let bytes = base64::decode(&uri[uri.find(",").unwrap_or(0) + 1..])?;
    let mt = &uri[uri.find(":").unwrap() + 1..uri.find(";").unwrap()];
    Ok((bytes, mt))
}

#[derive(thiserror::Error, Debug)]
pub enum ImportGltfError {
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("base 64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("image loading failed for file '{0}': {1}")]
    ImageLoadingFailed(String, ImageError),
    #[error("unknown image format '{0:?}' for image {1}")]
    UnknownImageFormat(String, usize),
    #[error("binary section of gltf not found")]
    BinSectionNotFound,
    #[error(
        "required property '{0}' is missing for mesh with index {1} and primitive with index {2}"
    )]
    RequiredMeshPropertyMissing(&'static str, usize, usize),
    #[error("unknown buffer index {0}")]
    UnknownBufferIndex(usize),
    #[error("buffer {0} has a view with range ({1}..{2}) that is out of bounds")]
    BufferRangeOutOfBounds(usize, usize, usize),
    #[error("unknown image index {0}")]
    UnknownImageIndex(usize),
    #[error("unknown material index {0}")]
    UnknownMaterialIndex(usize),
    #[error("unreachable")]
    Unreachable,
}
