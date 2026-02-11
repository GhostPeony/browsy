pub mod dom;
pub mod css;
pub mod layout;
pub mod output;
pub mod js;
#[cfg(feature = "fetch")]
pub mod fetch;

use output::SpatialDom;

/// Parse an HTML string and compute the Spatial DOM.
/// This is the primary entry point for browsy-core.
pub fn parse(html: &str, viewport_width: f32, viewport_height: f32) -> SpatialDom {
    let dom_tree = dom::parse_html(html);
    let styled = css::compute_styles_with_viewport(&dom_tree, viewport_width, viewport_height);
    let laid_out = layout::compute_layout(&styled, viewport_width, viewport_height);
    output::generate_spatial_dom(&laid_out, viewport_width, viewport_height)
}
