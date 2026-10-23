//! JDK HashSet<String> iteration, including resizing and red-black tree bins.
use std::collections::HashSet;

#[derive(Clone, Copy, Default)]
struct Bucket { first: Option<usize>, tree: bool }

struct Entry {
    key: String,
    hash: u32,
    prev: Option<usize>,
    next: Option<usize>,
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    red: bool,
}

struct Set {
    entries: Vec<Entry>,
    buckets: Vec<Bucket>,
    seen: HashSet<String>,
}

impl Set {
    fn new(capacity: usize) -> Self {
        Self { entries: vec![], buckets: vec![Bucket::default(); capacity], seen: HashSet::new() }
    }

    fn ids(&self, first: Option<usize>) -> Vec<usize> {
        let mut out=Vec::new();
        let mut current=first;
        while let Some(index)=current { out.push(index); current=self.entries[index].next; }
        out
    }

    fn link(&mut self, ids: &[usize]) {
        for (i,&id) in ids.iter().enumerate() {
            self.entries[id].prev=i.checked_sub(1).map(|p|ids[p]);
            self.entries[id].next=ids.get(i+1).copied();
        }
    }

    fn rotate(&mut self, root: usize, p: usize, left: bool) -> usize {
        let r=if left {self.entries[p].right} else {self.entries[p].left}.unwrap();
        let inside=if left {self.entries[r].left} else {self.entries[r].right};
        if left {self.entries[p].right=inside} else {self.entries[p].left=inside}
        if let Some(inside)=inside {self.entries[inside].parent=Some(p)}
        let parent=self.entries[p].parent;
        self.entries[r].parent=parent;
        if let Some(parent)=parent {
            if self.entries[parent].left==Some(p) {self.entries[parent].left=Some(r)} else {self.entries[parent].right=Some(r)}
        }
        if left {self.entries[r].left=Some(p)} else {self.entries[r].right=Some(p)}
        self.entries[p].parent=Some(r);
        if parent.is_none() {r} else {root}
    }

    fn balance(&mut self, mut root: usize, mut x: usize) -> usize {
        self.entries[x].red=true;
        while let Some(parent)=self.entries[x].parent {
            if !self.entries[parent].red {break}
            let Some(grand)=self.entries[parent].parent else {break};
            let parent_left=self.entries[grand].left==Some(parent);
            let uncle=if parent_left {self.entries[grand].right} else {self.entries[grand].left};
            if uncle.is_some_and(|u|self.entries[u].red) {
                self.entries[parent].red=false;
                self.entries[uncle.unwrap()].red=false;
                self.entries[grand].red=true;
                x=grand;
            } else {
                let inside=if parent_left {self.entries[parent].right==Some(x)} else {self.entries[parent].left==Some(x)};
                if inside {x=parent;root=self.rotate(root,x,parent_left)}
                let parent=self.entries[x].parent.unwrap();
                let grand=self.entries[parent].parent.unwrap();
                self.entries[parent].red=false;
                self.entries[grand].red=true;
                root=self.rotate(root,grand,!parent_left);
            }
        }
        self.entries[root].red=false;
        root
    }

    fn direction(&self, x: usize, parent: usize) -> bool {
        (self.entries[x].hash as i32).cmp(&(self.entries[parent].hash as i32))
            .then_with(||self.entries[x].key.encode_utf16().cmp(self.entries[parent].key.encode_utf16()))
            .is_lt()
    }

    fn insert_tree(&mut self, root: usize, x: usize, linked: bool) -> usize {
        let mut parent=root;
        loop {
            let left=self.direction(x,parent);
            let child=if left {self.entries[parent].left} else {self.entries[parent].right};
            if let Some(child)=child {parent=child;continue}
            if left {self.entries[parent].left=Some(x)} else {self.entries[parent].right=Some(x)}
            self.entries[x].parent=Some(parent);
            if linked {
                let next=self.entries[parent].next;
                self.entries[x].next=next;
                self.entries[x].prev=Some(parent);
                self.entries[parent].next=Some(x);
                if let Some(next)=next {self.entries[next].prev=Some(x)}
            }
            return self.balance(root,x);
        }
    }

    fn root_first(&mut self, bucket: usize, root: usize) {
        let first=self.buckets[bucket].first.unwrap();
        if first==root {return}
        let prev=self.entries[root].prev;
        let next=self.entries[root].next;
        if let Some(prev)=prev {self.entries[prev].next=next}
        if let Some(next)=next {self.entries[next].prev=prev}
        self.entries[first].prev=Some(root);
        self.entries[root].prev=None;
        self.entries[root].next=Some(first);
        self.buckets[bucket].first=Some(root);
    }

    fn treeify(&mut self, bucket: usize) {
        let ids=self.ids(self.buckets[bucket].first);
        for &id in &ids {
            self.entries[id].parent=None;
            self.entries[id].left=None;
            self.entries[id].right=None;
            self.entries[id].red=false;
        }
        let mut root=ids[0];
        for &id in &ids[1..] {root=self.insert_tree(root,id,false)}
        self.buckets[bucket].tree=true;
        self.root_first(bucket,root);
    }

    fn resize(&mut self) {
        let old_capacity=self.buckets.len();
        let old=std::mem::replace(&mut self.buckets,vec![Bucket::default();old_capacity*2]);
        for (bucket,entry) in old.into_iter().enumerate() {
            let ids=self.ids(entry.first);
            let (lo,hi):(Vec<_>,Vec<_>)=ids.into_iter().partition(|&id|self.entries[id].hash as usize & old_capacity==0);
            let split=!lo.is_empty() && !hi.is_empty();
            for (index,ids) in [(bucket,lo),(bucket+old_capacity,hi)] {
                self.link(&ids);
                self.buckets[index]=Bucket {first:ids.first().copied(),tree:entry.tree && ids.len()>6};
                if self.buckets[index].tree && split {self.treeify(index)}
            }
        }
    }

    fn insert(&mut self, key: String) {
        let raw=key.encode_utf16().fold(0u32,|h,c|h.wrapping_mul(31).wrapping_add(u32::from(c)));
        self.insert_hashed(key, raw);
    }

    fn insert_hashed(&mut self, key: String, raw: u32) {
        if !self.seen.insert(key.clone()) {return}
        let hash=raw ^ (raw>>16);
        let bucket=hash as usize & (self.buckets.len()-1);
        let index=self.entries.len();
        self.entries.push(Entry {key,hash,prev:None,next:None,parent:None,left:None,right:None,red:false});
        if let Some(first)=self.buckets[bucket].first {
            if self.buckets[bucket].tree {
                let root=self.insert_tree(first,index,true);
                self.root_first(bucket,root);
            } else {
                let ids=self.ids(Some(first));
                let tail=*ids.last().unwrap();
                self.entries[tail].next=Some(index);
                self.entries[index].prev=Some(tail);
                if ids.len()>=8 {
                    if self.buckets.len()<64 {self.resize()} else {self.treeify(bucket)}
                }
            }
        } else {self.buckets[bucket].first=Some(index)}
        if self.entries.len()>self.buckets.len()*3/4 {self.resize()}
    }

    fn values(&self) -> Vec<String> {
        self.buckets.iter().flat_map(|b|self.ids(b.first)).map(|i|self.entries[i].key.clone()).collect()
    }
}

pub(crate) fn attribute_names(values: impl IntoIterator<Item=String>) -> Vec<String> {
    let mut set = Set::new(16);
    for value in values { set.insert(value); }
    set.values()
}

pub(crate) fn scalars(values: impl IntoIterator<Item=String>) -> Vec<String> {
    let mut collected=Set::new(16);
    for value in values {collected.insert(value)}
    let capacity=((collected.entries.len().max(12)*4+2)/3).next_power_of_two();
    let mut result=Set::new(capacity);
    for value in collected.values() {result.insert(value)}
    result.insert("JSON".into());
    result.values()
}

pub(crate) fn auto_function_order(names: &[&str]) -> Option<Vec<usize>> {
    let mut hashes = HashSet::new();
    let mut values = Set::new(16);
    for name in names {
        let raw = name.encode_utf16().fold(0u32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(u32::from(unit))
        }).wrapping_add(31);
        // AutoFunction uses Objects.hash(name), not a prefixed String hash.
        // AutoFunction is not Comparable. Equal-hash tree nodes use original
        // JVM identities; retain that explicitly unresolved caller path.
        if !hashes.insert(raw) {
            return None;
        }
        values.insert_hashed((*name).to_owned(), raw);
    }
    let indices = names.iter().enumerate().map(|(index, name)| (*name, index))
        .collect::<std::collections::HashMap<_, _>>();
    Some(values.values().iter().map(|name| indices[name.as_str()]).collect())
}
