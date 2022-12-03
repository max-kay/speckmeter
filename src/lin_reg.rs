pub fn lin_reg(xs: &[f32], ys: &[f32]) -> Regression {
    let mean_x = xs.iter().sum::<f32>() / xs.len() as f32;
    let mean_y = ys.iter().sum::<f32>() / ys.len() as f32;

    let dev_xs = xs.iter().map(|x| x - mean_x);
    let dev_ys = ys.iter().map(|y| y - mean_y);

    let x_squared = dev_xs.clone().fold(0.0, |acc, x| acc + x * x);

    let slope = dev_ys.zip(dev_xs).fold(0.0, |acc, (y, x)| acc + x * y) / x_squared;
    let y_offset = mean_y - slope * mean_x;
    Regression { slope, y_offset }
}

pub struct Regression {
    pub slope: f32,
    pub y_offset: f32,
}
