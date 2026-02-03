use crate::camt053_format::Tag;
use crate::camt053_iterator::Camt053IterStatus::{Empty, Exists, NoOtherChildrens};
use std::cell::RefCell;
use std::rc::Rc;

pub struct TagView {
    node: Rc<RefCell<Tag>>,
    path: String,
}

impl TagView {
    pub fn new(node: Rc<RefCell<Tag>>, path: String) -> TagView {
        Self { node, path }
    }

    pub fn text(&self) -> String {
        if let Some(value) = &self.node.borrow().text {
            return value.clone();
        }
        String::new()
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn get_attr(&self, name: &str) -> Option<String> {
        self.node
            .borrow()
            .attrs
            .iter()
            .find(|&x| x.0 == name)
            .map(|x| x.1.clone())
    }

}

pub struct Camt053Iter {
    tag: Rc<RefCell<Tag>>,
    path: Vec<String>,
    first: bool,
}

impl Camt053Iter {
    pub fn new(tag: Rc<RefCell<Tag>>) -> Self {
        let p: Vec<String> = vec![tag.borrow().name.clone()];
        Self { tag, path: p, first: true}
    }

    fn get_next_in_parent(&mut self) -> Camt053IterStatus {
        let parent = match self.tag.borrow().parent.upgrade() {
            Some(p) => {
                self.path.pop();
                p
            }
            None => return Empty,
        };

        let status: Camt053IterStatus;
        let next = {
            let pb = parent.borrow();

            let idx = match pb.childrens.iter().position(|x| Rc::ptr_eq(x, &self.tag)) {
                Some(i) => i,
                None => return Empty,
            };

            if idx + 1 >= pb.childrens.len() {
                status = NoOtherChildrens;
                Rc::clone(&parent)
            } else {
                status = Exists;
                self.path.push(pb.childrens[idx + 1].borrow().name.clone());
                Rc::clone(&pb.childrens[idx + 1])
            }
        };

        self.tag = next;
        status
    }
}

enum Camt053IterStatus {
    NoOtherChildrens,
    Exists,
    Empty,
}

impl Iterator for Camt053Iter {
    type Item = TagView;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            return Some(TagView {
                node: self.tag.clone(),
                path: '/'.to_string() + self.path.join("/").as_str(),
            })
        }
        
        let children = {
            let g = self.tag.borrow();
            let mut result = None;
            if !g.childrens.is_empty() {
                self.path.push(g.childrens[0].borrow().name.clone());
                result = Some(g.childrens[0].clone());
            }
            result
        };

        if let Some(children) = children {
            self.tag = children;
            return Some(TagView {
                node: self.tag.clone(),
                path: '/'.to_string() + self.path.join("/").as_str(),
            });
        }

        loop {
            match self.get_next_in_parent() {
                NoOtherChildrens => continue,
                Exists => {
                    return Some(TagView {
                        node: self.tag.clone(),
                        path: '/'.to_string() + self.path.join("/").as_str(),
                    });
                }
                Empty => break,
            }
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Weak;

    fn tag(name: &str, text: Option<&str>) -> Rc<RefCell<Tag>> {
        Rc::new(RefCell::new(Tag {
            name: name.to_string(),
            text: text.map(|s| s.to_string()),
            attrs: Vec::new(),
            childrens: Vec::new(),
            parent: Weak::new(),
        }))
    }

    #[test]
    fn iter_builds_paths_and_text() {
        // virtual root
        let root = tag("root", None);
        let a = tag("A", None);
        let b = tag("B", Some("bbb"));
        let c = tag("C", None);

        b.borrow_mut().parent = Rc::downgrade(&a);
        c.borrow_mut().parent = Rc::downgrade(&a);
        a.borrow_mut().childrens.push(Rc::clone(&b));
        a.borrow_mut().childrens.push(Rc::clone(&c));
        a.borrow_mut().parent = Rc::downgrade(&root);
        root.borrow_mut().childrens.push(Rc::clone(&a));

        let got: Vec<(String, String)> = Camt053Iter::new(root)
            .map(|v| (v.path().to_string(), v.text()))
            .collect();

        assert_eq!(
            got,
            vec![
                ("/root".to_string(), "".to_string()),
                ("/root/A".to_string(), "".to_string()),
                ("/root/A/B".to_string(), "bbb".to_string()),
                ("/root/A/C".to_string(), "".to_string()),
            ]
        );
    }

    #[test]
    fn tagview_attr_works() {
        let n = tag("Amt", Some("10.00"));
        n.borrow_mut()
            .attrs
            .push(("Ccy".to_string(), "EUR".to_string()));

        let v = TagView::new(n, "/Stmt/Ntry/Amt".to_string());
        assert_eq!(v.get_attr("Ccy").as_deref(), Some("EUR"));
        assert_eq!(v.get_attr("Missing"), None);
    }
}