pub fn lin_reg(xs: Vec<f32>, yss: &[Vec<f32>]) -> Regression {
    let mean_x = xs.iter().sum::<f32>() / xs.len() as f32;
    let mean_ys: Vec<f32> = yss
        .iter()
        .map(|ys| ys.iter().sum::<f32>() / yss.len() as f32)
        .collect();

    let dev_xs = xs.iter().map(|x| x - mean_x);
    let dev_yss = yss
        .iter()
        .map(|ys| ys.iter().zip(&mean_ys).map(|(y, mean_y)| y - mean_y));

    let x_squared = dev_xs.clone().fold(0.0, |acc, x| acc + x * x);

    let slopes: Vec<f32> = dev_yss
        .map(|dev_ys| {
            dev_ys
                .zip(dev_xs.clone())
                .fold(0.0, |acc, (y, x)| acc + x * y)
                / x_squared
        })
        .collect();

    let y_offsets = slopes
        .iter()
        .zip(&mean_ys)
        .map(|(slope, mean_y)| mean_y - slope * mean_x)
        .collect();

    Regression { slopes, y_offsets }
}

pub struct Regression {
    pub slopes: Vec<f32>,
    pub y_offsets: Vec<f32>,
}
