use itertools::Itertools;

pub fn lin_reg(xs: &[f32], ys: &[&Vec<f32>]) -> Regression {
    let mean_x = xs.iter().fold(0.0, |acc, x| acc + x) / xs.len() as f32;
    let mean_y: Vec<f32> = ys
        .iter()
        .map(|vec| vec.iter().fold(0.0, |acc, y| acc + y) / ys.len() as f32)
        .collect_vec();

    let dev_x = xs.iter().map(|x| x - mean_x);
    let dev_ys = ys
        .iter()
        .map(|vec| vec.iter().enumerate().map(|(i, y)| y - mean_y[i]));

    let x_squared = dev_x.fold(0.0, |acc, x| acc + x*x);

    let slopes = dev_ys.clone().map(|dev_y| dev_y.zip(dev_x.clone()).fold(0.0, |acc, (y, x)| acc + x*y));

    let y_offsets = slopes
        .zip(mean_y)
        .map(|(slope, mean_y)| mean_y - slope * mean_x);

    Regression {
        slopes: slopes.collect(),
        y_offsets: y_offsets.collect_vec(),
    }
}

pub struct Regression {
    pub slopes: Vec<f32>,
    pub y_offsets: Vec<f32>,
}
