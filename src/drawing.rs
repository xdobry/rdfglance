use eframe::egui::{Color32, Painter, Pos2, Stroke};

pub fn draw_arrow_to_circle(painter: &Painter, point: Pos2, center: Pos2, radius: f32, color: Color32) {
    // Compute direction vector from point to center
    let dir_x = center.x - point.x;
    let dir_y = center.y - point.y;
    
    // Compute the length (Euclidean distance)
    let length = (dir_x.powi(2) + dir_y.powi(2)).sqrt();
    
    // Normalize and scale to radius
    let unit_x = dir_x / length;
    let unit_y = dir_y / length;
    
    // Find intersection on circle's surface
    let edge_x = center.x - unit_x * radius;
    let edge_y = center.y - unit_y * radius;
    let edge_point = Pos2::new(edge_x, edge_y);
    
    // Draw arrow (line + head)
    painter.line_segment([point, edge_point], Stroke::new(2.0, color));
    
    // Draw arrowhead
    let arrow_size = 8.0;  // Size of the arrowhead
    let arrow_angle = std::f32::consts::PI / 6.0; // 30 degrees
    
    // Rotate vector by ±arrow_angle to get arrowhead points
    let cos_theta = arrow_angle.cos();
    let sin_theta = arrow_angle.sin();
    
    let left_x = edge_x - arrow_size * (cos_theta * unit_x - sin_theta * unit_y);
    let left_y = edge_y - arrow_size * (sin_theta * unit_x + cos_theta * unit_y);
    
    let right_x = edge_x - arrow_size * (cos_theta * unit_x + sin_theta * unit_y);
    let right_y = edge_y - arrow_size * (-sin_theta * unit_x + cos_theta * unit_y);
    
    // Draw arrowhead lines
    painter.line_segment([edge_point, Pos2::new(left_x, left_y)], Stroke::new(2.0, color));
    painter.line_segment([edge_point, Pos2::new(right_x, right_y)], Stroke::new(2.0, color));
}