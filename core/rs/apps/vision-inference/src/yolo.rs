use std::path::Path;

use opencv::{
    core::{self, Mat, Scalar, Size},
    dnn, imgproc,
    prelude::*,
};
use vision_core::{DetectionCandidate, NormalizedBoundingBox, class_aware_nms};

const INPUT_SIZE: i32 = 640;

struct Letterbox {
    image: Mat,
    scale: f32,
    pad_left: f32,
    pad_top: f32,
}

pub(crate) struct YoloEngine {
    net: dnn::Net,
    class_names: Vec<String>,
    confidence_threshold: f32,
    nms_threshold: f32,
}

impl YoloEngine {
    pub(crate) fn load(
        model: &Path,
        class_names: Vec<String>,
        confidence_threshold: f32,
        nms_threshold: f32,
    ) -> opencv::Result<Self> {
        let model_path = model.to_string_lossy();
        let mut net = dnn::read_net_from_onnx(&model_path)?;
        net.set_preferable_backend(dnn::DNN_BACKEND_OPENCV)?;
        net.set_preferable_target(dnn::DNN_TARGET_CPU)?;
        Ok(Self {
            net,
            class_names,
            confidence_threshold,
            nms_threshold,
        })
    }

    pub(crate) fn infer(&mut self, frame: &Mat) -> opencv::Result<Vec<DetectionCandidate>> {
        let prepared = letterbox(frame)?;
        let blob = dnn::blob_from_image(
            &prepared.image,
            1.0 / 255.0,
            Size::new(INPUT_SIZE, INPUT_SIZE),
            Scalar::default(),
            true,
            false,
            core::CV_32F,
        )?;
        self.net.set_input_def(&blob)?;
        let mut output = Mat::default();
        self.net.forward_layer_def(&mut output)?;

        let candidates = decode_yolo_output(
            &output,
            frame.cols(),
            frame.rows(),
            &prepared,
            &self.class_names,
            self.confidence_threshold,
        )?;
        Ok(class_aware_nms(candidates, self.nms_threshold))
    }
}

fn letterbox(frame: &Mat) -> opencv::Result<Letterbox> {
    let scale =
        (INPUT_SIZE as f32 / frame.cols() as f32).min(INPUT_SIZE as f32 / frame.rows() as f32);
    let resized_width = (frame.cols() as f32 * scale).round() as i32;
    let resized_height = (frame.rows() as f32 * scale).round() as i32;
    let pad_x = INPUT_SIZE - resized_width;
    let pad_y = INPUT_SIZE - resized_height;
    let left = pad_x / 2;
    let right = pad_x - left;
    let top = pad_y / 2;
    let bottom = pad_y - top;

    let mut resized = Mat::default();
    imgproc::resize(
        frame,
        &mut resized,
        Size::new(resized_width, resized_height),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let mut image = Mat::default();
    core::copy_make_border(
        &resized,
        &mut image,
        top,
        bottom,
        left,
        right,
        core::BORDER_CONSTANT,
        Scalar::new(114.0, 114.0, 114.0, 0.0),
    )?;

    Ok(Letterbox {
        image,
        scale,
        pad_left: left as f32,
        pad_top: top as f32,
    })
}

fn decode_yolo_output(
    output: &Mat,
    image_width: i32,
    image_height: i32,
    letterbox: &Letterbox,
    class_names: &[String],
    confidence_threshold: f32,
) -> opencv::Result<Vec<DetectionCandidate>> {
    let shape = output.mat_size().to_vec();
    let dimensions = shape
        .into_iter()
        .filter(|dimension| *dimension > 1)
        .map(|dimension| dimension as usize)
        .collect::<Vec<_>>();
    if dimensions.len() != 2 {
        return Err(opencv::Error::new(
            core::StsBadSize,
            format!("salida YOLO no soportada: dimensiones={dimensions:?}"),
        ));
    }

    let attributes = class_names.len() + 4;
    let (candidate_count, channel_major) = if dimensions[0] == attributes {
        (dimensions[1], true)
    } else if dimensions[1] == attributes {
        (dimensions[0], false)
    } else {
        return Err(opencv::Error::new(
            core::StsBadSize,
            format!(
                "salida YOLO incompatible: dimensiones={dimensions:?}, clases={}",
                class_names.len()
            ),
        ));
    };

    let data = output.data_typed::<f32>()?;
    let expected_values = candidate_count * attributes;
    if data.len() < expected_values {
        return Err(opencv::Error::new(
            core::StsBadSize,
            format!(
                "salida YOLO incompleta: valores={}, esperados={expected_values}",
                data.len()
            ),
        ));
    }
    let value = |candidate: usize, attribute: usize| -> f32 {
        if channel_major {
            data[attribute * candidate_count + candidate]
        } else {
            data[candidate * attributes + attribute]
        }
    };
    let mut candidates = Vec::new();

    for candidate_index in 0..candidate_count {
        let (class_id, confidence) = class_names
            .iter()
            .enumerate()
            .map(|(class_id, _)| (class_id, value(candidate_index, class_id + 4)))
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap_or((0, 0.0));
        if confidence < confidence_threshold {
            continue;
        }

        let center_x = value(candidate_index, 0);
        let center_y = value(candidate_index, 1);
        let width = value(candidate_index, 2);
        let height = value(candidate_index, 3);
        let left = (center_x - width / 2.0 - letterbox.pad_left) / letterbox.scale;
        let top = (center_y - height / 2.0 - letterbox.pad_top) / letterbox.scale;
        let right = (center_x + width / 2.0 - letterbox.pad_left) / letterbox.scale;
        let bottom = (center_y + height / 2.0 - letterbox.pad_top) / letterbox.scale;
        let Some(bounding_box) = NormalizedBoundingBox::from_pixel_edges(
            left,
            top,
            right,
            bottom,
            image_width as u32,
            image_height as u32,
        ) else {
            continue;
        };

        candidates.push(DetectionCandidate {
            class_id: class_id as u32,
            class_name: class_names[class_id].clone(),
            confidence,
            bounding_box,
        });
    }

    Ok(candidates)
}
