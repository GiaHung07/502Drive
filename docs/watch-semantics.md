# Watch Semantics

Watch uses Drive `changes.list` at account/corpus scope. It does not poll each watched folder independently.

Source removal, trash, or permission loss preserves destination files by default. Move-out detaches mappings. Move-back reuses the existing mapping and destination item.

