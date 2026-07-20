use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const COCO_CLASSES: [&str; 80] = [
    "person",
    "bicycle",
    "car",
    "motorcycle",
    "airplane",
    "bus",
    "train",
    "truck",
    "boat",
    "traffic light",
    "fire hydrant",
    "stop sign",
    "parking meter",
    "bench",
    "bird",
    "cat",
    "dog",
    "horse",
    "sheep",
    "cow",
    "elephant",
    "bear",
    "zebra",
    "giraffe",
    "backpack",
    "umbrella",
    "handbag",
    "tie",
    "suitcase",
    "frisbee",
    "skis",
    "snowboard",
    "sports ball",
    "kite",
    "baseball bat",
    "baseball glove",
    "skateboard",
    "surfboard",
    "tennis racket",
    "bottle",
    "wine glass",
    "cup",
    "fork",
    "knife",
    "spoon",
    "bowl",
    "banana",
    "apple",
    "sandwich",
    "orange",
    "broccoli",
    "carrot",
    "hot dog",
    "pizza",
    "donut",
    "cake",
    "chair",
    "couch",
    "potted plant",
    "bed",
    "dining table",
    "toilet",
    "tv",
    "laptop",
    "mouse",
    "remote",
    "keyboard",
    "cell phone",
    "microwave",
    "oven",
    "toaster",
    "sink",
    "refrigerator",
    "book",
    "clock",
    "vase",
    "scissors",
    "teddy bear",
    "hair drier",
    "toothbrush",
];

pub(crate) fn load_class_names(
    path: Option<&Path>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    match path {
        None => Ok(COCO_CLASSES.iter().map(|name| (*name).to_owned()).collect()),
        Some(path) => {
            let file = File::open(path)?;
            let names = BufReader::new(file)
                .lines()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>();
            if names.is_empty() {
                Err("el archivo de clases está vacío".into())
            } else {
                Ok(names)
            }
        }
    }
}
