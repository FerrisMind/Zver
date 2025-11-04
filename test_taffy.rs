use taffy::prelude::*;

fn main() -> Result<(), taffy::TaffyError> {
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    let node = taffy.new_leaf(Style {
        size: Size { width: length(100.0), height: length(100.0) },
        ..Default::default()
    })?;

    let root = taffy.new_with_children(Style::default(), &[node])?;

    taffy.compute_layout(root, Size::MAX_CONTENT)?;

    let layout = taffy.layout(node)?;
    println!("Layout: {:?}", layout);

    Ok(())
}
