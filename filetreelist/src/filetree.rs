use crate::{
	error::Result, filetreeitems::FileTreeItems,
	tree_iter::TreeIterator, TreeItemInfo,
};
use std::{collections::BTreeSet, path::Path};

///
#[derive(Copy, Clone, Debug)]
pub enum MoveSelection {
	Up,
	Down,
	Left,
	Right,
	Top,
	End,
	PageDown,
	PageUp,
}

#[derive(Debug, Clone, Copy)]
pub struct VisualSelection {
	pub count: usize,
	pub index: usize,
}

/// wraps `FileTreeItems` as a datastore and adds selection functionality
#[derive(Default)]
pub struct FileTree {
	items: FileTreeItems,
	selection: Option<usize>,
	// caches the absolute selection translated to visual index
	visual_selection: Option<VisualSelection>,
}

impl FileTree {
	///
	pub fn new(
		list: &[&Path],
		collapsed: &BTreeSet<&String>,
	) -> Result<Self> {
		let mut new_self = Self {
			items: FileTreeItems::new(list, collapsed)?,
			selection: if list.is_empty() { None } else { Some(0) },
			visual_selection: None,
		};
		new_self.visual_selection = new_self.calc_visual_selection();

		Ok(new_self)
	}

	///
	pub const fn is_empty(&self) -> bool {
		self.items.file_count() == 0
	}

	///
	pub const fn selection(&self) -> Option<usize> {
		self.selection
	}

	///
	pub fn collapse_but_root(&mut self) {
		if !self.is_empty() {
			self.items.collapse(0, true);
			self.items.expand(0, false);
		}
	}

	/// iterates visible elements starting from `start_index_visual`
	pub fn iterate(
		&self,
		start_index_visual: usize,
		max_amount: usize,
	) -> TreeIterator<'_> {
		let start = self
			.visual_index_to_absolute(start_index_visual)
			.unwrap_or_default();
		TreeIterator::new(
			self.items.iterate(start, max_amount),
			self.selection,
		)
	}

	///
	pub const fn visual_selection(&self) -> Option<&VisualSelection> {
		self.visual_selection.as_ref()
	}

	///
	pub fn selected_file(&self) -> Option<&TreeItemInfo> {
		self.selection.and_then(|index| {
			let item = &self.items.tree_items[index];
			if item.kind().is_path() {
				None
			} else {
				Some(item.info())
			}
		})
	}

	///
	pub fn collapse_recursive(&mut self) {
		if let Some(selection) = self.selection {
			self.items.collapse(selection, true);
		}
	}

	///
	pub fn expand_recursive(&mut self) {
		if let Some(selection) = self.selection {
			self.items.expand(selection, true);
		}
	}

	///
	pub fn move_selection(&mut self, dir: MoveSelection) -> bool {
		self.selection.is_some_and(|selection| {
			let new_index = match dir {
				MoveSelection::Up => {
					self.selection_updown(selection, true)
				}
				MoveSelection::Down => {
					self.selection_updown(selection, false)
				}
				MoveSelection::Left => self.selection_left(selection),
				MoveSelection::Right => {
					self.selection_right(selection)
				}
				MoveSelection::Top => {
					Self::selection_start(selection)
				}
				MoveSelection::End => self.selection_end(selection),
				MoveSelection::PageDown | MoveSelection::PageUp => {
					None
				}
			};

			let changed_index =
				new_index.is_some_and(|i| i != selection);

			if changed_index {
				self.selection = new_index;
				self.visual_selection = self.calc_visual_selection();
			}

			changed_index || new_index.is_some()
		})
	}

	pub fn select_file(&mut self, path: &Path) -> bool {
		let new_selection = self
			.items
			.tree_items
			.iter()
			.position(|item| item.info().full_path() == path);

		if new_selection == self.selection {
			return false;
		}

		self.selection = new_selection;
		if let Some(selection) = self.selection {
			self.items.show_element(selection);
		}
		self.visual_selection = self.calc_visual_selection();
		true
	}

	fn visual_index_to_absolute(
		&self,
		visual_index: usize,
	) -> Option<usize> {
		self.items
			.iterate(0, self.items.len())
			.enumerate()
			.find_map(|(i, (abs, _))| {
				if i == visual_index {
					Some(abs)
				} else {
					None
				}
			})
	}

	fn calc_visual_selection(&self) -> Option<VisualSelection> {
		self.selection.map(|selection_absolute| {
			let mut count = 0;
			let mut visual_index = 0;
			for (index, _item) in
				self.items.iterate(0, self.items.len())
			{
				if selection_absolute == index {
					visual_index = count;
				}

				count += 1;
			}

			VisualSelection {
				index: visual_index,
				count,
			}
		})
	}

	const fn selection_start(current_index: usize) -> Option<usize> {
		if current_index == 0 {
			None
		} else {
			Some(0)
		}
	}

	fn selection_end(&self, current_index: usize) -> Option<usize> {
		let items_max = self.items.len().saturating_sub(1);

		let mut new_index = items_max;

		loop {
			if self.is_visible_index(new_index) {
				break;
			}

			if new_index == 0 {
				break;
			}

			new_index = new_index.saturating_sub(1);
			new_index = std::cmp::min(new_index, items_max);
		}

		if new_index == current_index {
			None
		} else {
			Some(new_index)
		}
	}

	fn selection_updown(
		&self,
		current_index: usize,
		up: bool,
	) -> Option<usize> {
		let mut index = current_index;

		loop {
			index = {
				let new_index = if up {
					index.saturating_sub(1)
				} else {
					index.saturating_add(1)
				};

				// when reaching usize bounds
				if new_index == index {
					break;
				}

				if new_index >= self.items.len() {
					break;
				}

				new_index
			};

			if self.is_visible_index(index) {
				break;
			}
		}

		if index == current_index {
			None
		} else {
			Some(index)
		}
	}

	fn select_parent(&self, current_index: usize) -> Option<usize> {
		let indent =
			self.items.tree_items[current_index].info().indent();

		let mut index = current_index;

		while let Some(selection) = self.selection_updown(index, true)
		{
			index = selection;

			if self.items.tree_items[index].info().indent() < indent {
				break;
			}
		}

		if index == current_index {
			None
		} else {
			Some(index)
		}
	}

	fn selection_left(
		&mut self,
		current_index: usize,
	) -> Option<usize> {
		let item = &mut self.items.tree_items[current_index];

		if item.kind().is_path() && !item.kind().is_path_collapsed() {
			self.items.collapse(current_index, false);
			return Some(current_index);
		}

		self.select_parent(current_index)
	}

	fn selection_right(
		&mut self,
		current_selection: usize,
	) -> Option<usize> {
		let item = &mut self.items.tree_items[current_selection];

		if item.kind().is_path() {
			if item.kind().is_path_collapsed() {
				self.items.expand(current_selection, false);
				return Some(current_selection);
			}
			return self.selection_updown(current_selection, false);
		}

		None
	}

	fn is_visible_index(&self, index: usize) -> bool {
		self.items
			.tree_items
			.get(index)
			.is_some_and(|item| item.info().is_visible())
	}
}

#[cfg(test)]
mod test {
	use crate::{FileTree, MoveSelection};
	use pretty_assertions::assert_eq;
	use std::{collections::BTreeSet, path::Path};

	#[test]
	fn test_selection() {
		let items = vec![
			Path::new("a/b"), //
		];

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		assert!(tree.move_selection(MoveSelection::Down));

		assert_eq!(tree.selection, Some(1));

		assert!(!tree.move_selection(MoveSelection::Down));

		assert_eq!(tree.selection, Some(1));
	}

	#[test]
	fn test_selection_skips_collapsed() {
		let items = vec![
			Path::new("a/b/c"), //
			Path::new("a/d"),   //
		];

		//0 a/
		//1   b/
		//2     c
		//3   d

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		tree.items.collapse(1, false);
		tree.selection = Some(1);

		assert!(tree.move_selection(MoveSelection::Down));

		assert_eq!(tree.selection, Some(3));
	}

	#[test]
	fn test_selection_left_collapse() {
		let items = vec![
			Path::new("a/b/c"), //
			Path::new("a/d"),   //
		];

		//0 a/
		//1   b/
		//2     c
		//3   d

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		tree.selection = Some(1);

		//collapses 1
		assert!(tree.move_selection(MoveSelection::Left));
		// index will not change
		assert_eq!(tree.selection, Some(1));

		assert!(tree.items.tree_items[1].kind().is_path_collapsed());
		assert!(!tree.items.tree_items[2].info().is_visible());
	}

	#[test]
	fn test_selection_left_parent() {
		let items = vec![
			Path::new("a/b/c"), //
			Path::new("a/d"),   //
		];

		//0 a/
		//1   b/
		//2     c
		//3   d

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		tree.selection = Some(2);

		assert!(tree.move_selection(MoveSelection::Left));
		assert_eq!(tree.selection, Some(1));

		assert!(tree.move_selection(MoveSelection::Left));
		assert_eq!(tree.selection, Some(1));

		assert!(tree.move_selection(MoveSelection::Left));
		assert_eq!(tree.selection, Some(0));
	}

	#[test]
	fn test_selection_right_expand() {
		let items = vec![
			Path::new("a/b/c"), //
			Path::new("a/d"),   //
		];

		//0 a/
		//1   b/
		//2     c
		//3   d

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		tree.items.collapse(1, false);
		tree.items.collapse(0, false);
		tree.selection = Some(0);

		assert!(tree.move_selection(MoveSelection::Right));
		assert_eq!(tree.selection, Some(0));
		assert!(!tree.items.tree_items[0].kind().is_path_collapsed());

		assert!(tree.move_selection(MoveSelection::Right));
		assert_eq!(tree.selection, Some(1));
		assert!(tree.items.tree_items[1].kind().is_path_collapsed());

		assert!(tree.move_selection(MoveSelection::Right));
		assert_eq!(tree.selection, Some(1));
		assert!(!tree.items.tree_items[1].kind().is_path_collapsed());
	}

	#[test]
	fn test_selection_top() {
		let items = vec![
			Path::new("a/b/c"), //
			Path::new("a/d"),   //
		];

		//0 a/
		//1   b/
		//2     c
		//3   d

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		tree.selection = Some(3);

		assert!(tree.move_selection(MoveSelection::Top));
		assert_eq!(tree.selection, Some(0));
	}

	#[test]
	fn test_visible_selection() {
		let items = vec![
			Path::new("a/b/c"),  //
			Path::new("a/b/c2"), //
			Path::new("a/d"),    //
		];

		//0 a/
		//1   b/
		//2     c
		//3     c2
		//4   d

		let mut tree =
			FileTree::new(&items, &BTreeSet::new()).unwrap();

		tree.selection = Some(1);
		assert!(tree.move_selection(MoveSelection::Left));
		assert!(tree.move_selection(MoveSelection::Down));
		let s = tree.visual_selection().unwrap();

		assert_eq!(s.count, 3);
		assert_eq!(s.index, 2);
	}
}
