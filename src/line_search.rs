pub trait Gradient {
    fn gradient(&self, parameters: Vec<f32>) -> Vec<f32>;
}

pub trait Cost {
    fn cost(&self, parameters: Vec<f32>) -> f32;
}

pub fn search_minimum<P>(
    problem: P,
    initial_params: Vec<f32>,
    max_iterations: u32,
    initial_step_size: f32,
) -> Vec<f32>
where
    P: Gradient + Cost,
{
    // impelmented after https://en.wikipedia.org/wiki/Gradient_descent and https://en.wikipedia.org/wiki/Backtracking_line_search
    // control factors c and tau and intitial step size
    let c = 0.5; // e (0, 1)
    let tau = 0.8; // e (0, 1)
    let mut last_step_size = initial_step_size; // this value should be better determined TODO
    let mut parameters = initial_params;
    for i in 0..max_iterations {
        let gradient = problem.gradient(parameters.clone());
        if i % 400 == 0 {
            println!("gradient: {:?}", gradient);
            println!("last step size: {}", last_step_size);
            println!(
                "at step {} the error is {}",
                i,
                problem.cost(parameters.clone())
            );
            println!("tan(alpha) = {}", parameters[0]);
            println!("distance to sensor / sensor width = {}", parameters[1]);
            println!(
                "offset of lightray normal / sensor width = {}\n\n",
                parameters[2]
            );
        }


        let t = -c * inner_product(gradient.clone(), gradient.clone());
        let mut current_alpha = last_step_size;
        let step_size = loop {
            if problem.cost(parameters.clone())
                - problem.cost(add(
                    parameters.clone(),
                    scale(gradient.clone(), -current_alpha),
                ))
                >= current_alpha * t
            {
                break current_alpha;
            }
            current_alpha *= tau
        };
        parameters = add(parameters, scale(gradient.clone(), -step_size));
        last_step_size = step_size;
    }
    parameters
}

fn add(x1: Vec<f32>, x2: Vec<f32>) -> Vec<f32> {
    assert_eq!(x1.len(), x2.len());
    x1.iter().zip(x2.iter()).map(|(x1, x2)| x1 + x2).collect()
}

pub fn scale(x: Vec<f32>, factor: f32) -> Vec<f32> {
    x.iter().map(|x| x * factor).collect()
}

fn inner_product(x1: Vec<f32>, x2: Vec<f32>) -> f32 {
    assert_eq!(x1.len(), x2.len());
    x1.iter().zip(x2.iter()).map(|(x1, x2)| x1 * x2).sum()
}
