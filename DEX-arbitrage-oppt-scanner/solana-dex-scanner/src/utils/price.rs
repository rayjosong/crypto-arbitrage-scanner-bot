use crate::models::pool::PoolReserves;

pub fn calculate_price(reserves: &PoolReserves) -> f64 {
    let amount_a = reserves.token_a as f64 / 10f64.powi(reserves.decimals_a as i32);
    let amount_b = reserves.token_b as f64 / 10f64.powi(reserves.decimals_b as i32);
    
    if amount_a == 0.0 {
        return 0.0;
    }
    
    amount_b / amount_a
}

pub fn calculate_profit_margin(price_a: f64, price_b: f64) -> f64 {
    if price_a > price_b && price_b > 0.0 {
        price_a / price_b - 1.0
    } else if price_b > price_a && price_a > 0.0 {
        price_b / price_a - 1.0
    } else {
        0.0
    }
} 