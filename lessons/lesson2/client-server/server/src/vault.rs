use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub items: Vec<Item>,
    pub capacity: u32,   // вместимость ячейки
    pub used_space: u32, // сколько занято
}

#[derive(Debug)]
pub enum CellError {
    Full,
    NotFound
}

impl Cell {
    pub fn new(capacity: u32) -> Self {
        Self {
            items: Vec::new(),
            capacity,
            used_space: 0,
        }
    }

    pub fn put_item(&mut self, item: Item) -> Result<(), CellError> {
        if self.used_space + item.size > self.capacity {
            return Err(CellError::Full);
        }
        self.used_space += item.size;
        self.items.push(item);
        Ok(())
    }

    pub fn list_items(&self) -> Option<String> {
        if self.items.is_empty() {
            None
        } else {
            let descriptions: Vec<String> = self
                .items
                .iter()
                .map(|i| format!("{}({})", i.name, i.size))
                .collect();
            Some(format!(
                "Items: {} | Used: {}/{}\n",
                descriptions.join(", "),
                self.used_space,
                self.capacity
            ))
        }
    }

    pub fn take(&mut self, name: &str) -> Result<Item, CellError> {
        let item = self.items.pop_if(|i| i.name == name);
        if let Some(i) = item {
            self.used_space -= self.used_space.saturating_sub(i.size);
            Ok(i)
        } else {
            Err(CellError::NotFound)
        }
    }
}

pub struct Vault {
    cells: HashMap<u32, Cell>,
    capacity: usize, // максимальное количество ячеек
}

#[derive(Debug)]
pub enum VaultError {
    VaultFull,
    CellFull,
    CellNotFound,
    ItemNotFound
}

impl Vault {
    pub fn new(capacity: usize) -> Self {
        Self {
            cells: HashMap::new(),
            capacity,
        }
    }

    // Положить предмет в ячейку
    pub fn put(&mut self, id: u32, item: Item, cell_capacity: u32) -> Result<(), VaultError> {
        if self.cells.len() >= self.capacity && !self.cells.contains_key(&id) {
            return Err(VaultError::VaultFull);
        }

        let cell = self
            .cells
            .entry(id)
            .or_insert_with(|| Cell::new(cell_capacity));
        cell.put_item(item).map_err(|_| VaultError::CellFull)
    }

    // Показать содержимое ячейки
    pub fn get(&self, id: u32) -> Result<Option<String>, VaultError> {
        match self.cells.get(&id) {
            Some(cell) => Ok(cell.list_items()),
            None => Err(VaultError::CellNotFound),
        }
    }

    // Показать список занятых ячеек
    pub fn list(&self) -> Option<String> {
        if self.cells.is_empty() {
            None
        } else {
            let keys: Vec<String> = self.cells.keys().map(|id| id.to_string()).collect();
            Some(format!("Occupied cells: {}\n", keys.join(", ")))
        }
    }

    pub fn take(&mut self, id: u32, name: &str) -> Result<Item, VaultError> {
        match self.cells.get_mut(&id) {
            Some(cell) => {
                cell.take(name).map_err(|e| match e {
                    CellError::NotFound => VaultError::ItemNotFound,
                    CellError::Full => VaultError::CellFull, // unlikely here
                })
            },
            None => Err(VaultError::CellNotFound)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_item_from_cell() {
        let mut cell = Cell {
            items: vec![
                Item {
                    name: "gold".to_string(),
                    size: 10,
                },
                Item {
                    name: "silver".to_string(),
                    size: 5,
                },
            ],
            capacity: 100,
            used_space: 15,
        };

        // берём предмет, который есть
        let item = cell.take("gold").expect("should take gold");
        assert_eq!(item.name, "gold");
        assert_eq!(item.size, 10);
        assert_eq!(cell.used_space, 5); // used_space уменьшился
        assert_eq!(cell.items.len(), 1);

        // берём предмет, который остался
        let item2 = cell.take("silver").expect("should take silver");
        assert_eq!(item2.name, "silver");
        assert_eq!(item2.size, 5);
        assert_eq!(cell.used_space, 0);
        assert!(cell.items.is_empty());

        // пытаемся взять несуществующий предмет
        let res = cell.take("diamond");
        assert!(matches!(res, Err(CellError::NotFound)));
    }

    #[test]
    fn test_take_item_from_vault() {
        let mut vault = Vault {
            cells: std::collections::HashMap::new(),
            capacity: 100,
        };

        vault.cells.insert(
            1,
            Cell {
                items: vec![
                    Item {
                        name: "gold".to_string(),
                        size: 10,
                    },
                    Item {
                        name: "silver".to_string(),
                        size: 5,
                    },
                ],
                capacity: 100,
                used_space: 15,
            },
        );

        // забираем существующий предмет
        let item = vault.take(1, "gold").expect("should take gold");
        assert_eq!(item.name, "gold");
        assert_eq!(item.size, 10);

        // забираем второй предмет
        let item2 = vault.take(1, "silver").expect("should take silver");
        assert_eq!(item2.name, "silver");
        assert_eq!(item2.size, 5);

        // пытаемся взять из пустой ячейки
        let res = vault.take(1, "diamond");
        assert!(matches!(res, Err(VaultError::ItemNotFound)));

        // пытаемся взять из несуществующей ячейки
        let res = vault.take(2, "gold");
        assert!(matches!(res, Err(VaultError::CellNotFound)));
    }
}