mod graphics;

use proc_macro::TokenStream;

/// Implements the `ExtractComponent` trait for a component.
/// The component must implement [`Clone`]
/// The component will be extracted into the render world as is (`.clone()`)
/// 
/// # Example
/// ```no_compile
/// // This will extract any Foo with a Camera into the render world via Clone
/// #[derive(Component, Clone, ExtractComponent)]
/// #[extract_component_filter(With<Camera>)]
/// pub struct Foo {
///     // Snip --
/// }
/// 
/// // This will extract all Bar into the render world via Clone
/// #[derive(Component, Clone, ExtractComponent)]
/// pub struct Bar {
///     // Snip --
/// }
/// ```
#[proc_macro_derive(ExtractComponent)]
pub fn derive_extract_component(item: TokenStream) -> TokenStream {
    graphics::derive_extract_component(item)
}